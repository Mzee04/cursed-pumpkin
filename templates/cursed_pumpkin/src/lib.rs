//! # The Cursed Pumpkin
//!
//! A private hot-potato party game on Tari Ootle.
//!
//! * At creation the template mints exactly **one** PUMPKIN: a *stealth* token (divisibility 0, supply 1). The host
//!   then *summons* it out of the game's cauldron into a stealth UTXO, whose owner and amount are hidden.
//! * Players join by locking an equal bond in the game's pot. Joining is public: everybody can see who is playing.
//! * Whoever holds the pumpkin may **pass** it to anyone through [`CursedPumpkin::pass`]. The transfer is a stealth
//!   transfer signed with a throwaway key, so the network learns only that *a* pass happened (the public pass counter
//!   ticks up) and never who gave it or who received it. The template remembers only the commitment of the single
//!   live pumpkin UTXO, which hides its owner.
//! * Before midnight every player may lock in a **sealed guess** of who will be holding the pumpkin at midnight: a
//!   SHA-256 commitment, so the holder cannot see who suspects them.
//! * At **midnight** (an epoch fixed at creation) the template refuses any further pass. During the reveal window the
//!   holder unmasks themselves: they spend the live pumpkin UTXO into the template (the engine checks the UTXO's
//!   spend authorization, which is the ownership proof) and sign with their player key, which receives the revealed
//!   pumpkin. Guessers open their sealed guesses in the same window.
//! * **Payout.** If nobody guessed the holder, the ghost escapes and takes the whole pot. If someone did, the holder
//!   gets their own bond back and the correct guessers split the rest. Everyone else loses their bond.
//! * If the holder never reveals, after the window every player may take back half their bond; the other half rots
//!   inside the pumpkin forever. Revealing is therefore always better for the holder than hiding.
//!
//! Moving the pumpkin by any other route (a plain wallet stealth transfer) is not forbidden by the network, but the
//! template only follows the UTXO it last saw, so a pumpkin moved behind the template's back can never be revealed:
//! the curse only follows sanctioned passes. That is also what freezes the game at midnight.
//!
//! All code in this crate was written for the Tari October 2026 build contest. MIT licensed.

use tari_template_lib::prelude::*;

/// Domain tag for sealed guesses: `sha256(GUESS_DOMAIN || game || guesser || suspect || salt)`.
pub const GUESS_DOMAIN: &[u8] = b"cursed-pumpkin/guess/v1";
/// Upper bound on players, so the component state and every call stay bounded.
pub const MAX_PLAYERS: usize = 32;

/// Compute a sealed-guess commitment. Exposed so off-chain tools and tests use exactly the same formula.
pub fn guess_commitment(
    game: &[u8],
    guesser: &RistrettoPublicKeyBytes,
    suspect: &RistrettoPublicKeyBytes,
    salt: &[u8; 32],
) -> Hash32 {
    intrinsics::hash_sha256_parts(&[GUESS_DOMAIN, game, guesser.as_ref(), suspect.as_ref(), salt])
}

#[template]
mod cursed_pumpkin {
    use super::*;

    pub struct CursedPumpkin {
        name: String,
        host: RistrettoPublicKeyBytes,
        pumpkin: ResourceAddress,
        /// Commitment of the one live pumpkin UTXO. Public, but it hides who can spend it.
        live_pumpkin: Option<PedersenCommitmentBytes>,
        passes: u64,
        last_pass_epoch: u64,
        midnight_epoch: u64,
        reveal_until_epoch: u64,
        bond: Amount,
        pot: Vault,
        /// Optional TARI held to pay the fees of passes, so a pass needs no account (and links to none).
        gas: Vault,
        max_pass_fee: Amount,
        players: Vec<Player>,
        /// The pumpkin waits here before it is summoned and comes back here after the reveal.
        cauldron: Vault,
        summoned: bool,
        holder: Option<RistrettoPublicKeyBytes>,
        /// Pot snapshot taken at the first claim, so the split never depends on claim order.
        pot_at_settlement: Option<Amount>,
    }

    impl CursedPumpkin {
        /// Create a game. The single PUMPKIN is minted into the game's cauldron (revealed, so the mint is public) and
        /// stays there until the host summons it into a hidden UTXO with [`Self::summon`]. `bond_resource` is what
        /// players stake (TARI on a live network). `max_pass_fee` caps what one pass may draw from the gas tank.
        pub fn new(
            name: String,
            bond_resource: ResourceAddress,
            bond: Amount,
            midnight_epoch: u64,
            reveal_epochs: u64,
            max_pass_fee: Amount,
        ) -> Component<Self> {
            let now = Consensus::current_epoch();
            assert!(midnight_epoch > now, "Midnight must be in the future");
            assert!(reveal_epochs > 0, "The reveal window must be at least one epoch");
            assert!(bond.is_positive(), "The bond must be positive");
            assert!(name.len() <= 64, "Name too long");

            let alloc = CallerContext::allocate_component_address(None);
            let game = alloc.get_address();
            let pumpkin_bucket = ResourceBuilder::stealth()
                .with_token_symbol("PUMPKIN")
                .with_divisibility(0)
                .add_metadata("name", "The Cursed Pumpkin")
                .add_metadata("game", game.to_string())
                .initial_supply(1u64);
            let pumpkin = pumpkin_bucket.resource_address();

            let host = CallerContext::transaction_signer_public_key();
            emit_event("CursedPumpkin.Created", metadata! {
                "name" => name.clone(),
                "pumpkin" => pumpkin.to_string(),
                "midnight_epoch" => midnight_epoch.to_string(),
                "reveal_until_epoch" => (midnight_epoch + reveal_epochs).to_string(),
                "bond" => bond.to_string(),
            });

            Component::new(Self {
                name,
                host,
                pumpkin,
                live_pumpkin: None,
                passes: 0,
                last_pass_epoch: now,
                midnight_epoch,
                reveal_until_epoch: midnight_epoch + reveal_epochs,
                bond,
                pot: Vault::new_empty(bond_resource),
                gas: Vault::new_empty(TARI_TOKEN),
                max_pass_fee,
                players: Vec::new(),
                cauldron: Vault::from_bucket(pumpkin_bucket),
                summoned: false,
                holder: None,
                pot_at_settlement: None,
            })
            .with_access_rules(AccessRules::allow_all())
            .with_address_allocation(alloc)
            .create()
        }

        /// The host sends the pumpkin out of the cauldron into one hidden UTXO (for themselves or anyone). `transfer`
        /// must take the revealed pumpkin as its only input and create exactly one stealth output. Once only.
        pub fn summon(&mut self, transfer: StealthTransferStatement, fee: Amount) {
            self.assert_before_midnight();
            assert_eq!(
                CallerContext::transaction_signer_public_key(),
                self.host,
                "Only the host can summon the pumpkin"
            );
            assert!(!self.summoned, "The pumpkin has already been summoned");
            Self::check_single_hidden_output(&transfer);
            assert!(
                transfer.inputs_statement.inputs.is_empty() &&
                    transfer.inputs_statement.revealed_amount == Amount::from(1u64),
                "The summoning must spend exactly the revealed pumpkin"
            );
            self.pay_from_gas(fee);
            let live = *transfer.outputs_statement.outputs[0].commitment();
            let pumpkin = self.cauldron.withdraw(Amount::from(1u64));
            let rest = ResourceManager::get(self.pumpkin).stealth_transfer_with_opt_input_bucket(transfer, Some(pumpkin));
            assert!(rest.is_none(), "The summoning must not reveal anything");
            self.live_pumpkin = Some(live);
            self.summoned = true;
            emit_event("CursedPumpkin.Summoned", metadata! { "commitment" => live.to_string() });
        }

        // ---------------------------------------------------------------- reads

        pub fn phase(&self) -> Phase {
            let now = Consensus::current_epoch();
            if now < self.midnight_epoch {
                Phase::Haunting
            } else if now < self.reveal_until_epoch {
                Phase::Reveal
            } else {
                Phase::Settled
            }
        }

        pub fn pumpkin_resource(&self) -> ResourceAddress {
            self.pumpkin
        }

        pub fn passes(&self) -> u64 {
            self.passes
        }

        pub fn live_pumpkin(&self) -> Option<PedersenCommitmentBytes> {
            self.live_pumpkin
        }

        pub fn holder(&self) -> Option<RistrettoPublicKeyBytes> {
            self.holder
        }

        pub fn players(&self) -> Vec<RistrettoPublicKeyBytes> {
            self.players.iter().map(|p| p.key).collect()
        }

        pub fn pot_balance(&self) -> Amount {
            self.pot.balance()
        }

        pub fn catchers(&self) -> Vec<RistrettoPublicKeyBytes> {
            self.catcher_keys()
        }

        // ---------------------------------------------------------------- haunting phase

        /// Join with exactly one bond. The transaction signer's key becomes your player key.
        pub fn join(&mut self, bond: Bucket) {
            self.assert_before_midnight();
            let key = CallerContext::transaction_signer_public_key();
            assert!(self.find(&key).is_none(), "Already joined");
            assert!(self.players.len() < MAX_PLAYERS, "The game is full");
            assert_eq!(bond.resource_address(), self.pot.resource_address(), "Wrong bond resource");
            assert_eq!(bond.amount(), self.bond, "The bond must be exactly the game's bond");
            self.pot.deposit(bond);
            self.players.push(Player {
                key,
                sealed_guess: None,
                opened_guess: None,
                claimed: false,
            });
            emit_event("CursedPumpkin.Joined", metadata! {
                "player" => key.to_string(),
                "players" => self.players.len().to_string(),
            });
        }

        /// Anyone may top up the gas tank that pays for passes.
        pub fn add_gas(&mut self, tari: Bucket) {
            self.gas.deposit(tari);
        }

        /// Pass the pumpkin. `transfer` must spend exactly the live pumpkin UTXO into exactly one new stealth UTXO,
        /// revealing nothing. The engine checks the spend authorization of the input, so only the current holder can
        /// produce a valid statement; the transaction itself can be signed by a throwaway key.
        ///
        /// If `fee` is positive the fee is paid from the gas tank (call this in the fee stage).
        pub fn pass(&mut self, transfer: StealthTransferStatement, fee: Amount) {
            self.assert_before_midnight();
            let live = self.live_pumpkin.expect("The pumpkin is not out in the world");
            Self::check_single_hidden_output(&transfer);
            assert!(
                transfer.inputs_statement.inputs.len() == 1 &&
                    transfer.inputs_statement.revealed_amount.is_zero() &&
                    transfer.inputs_statement.inputs[0].commitment == live,
                "A pass must spend exactly the live pumpkin"
            );
            let next = *transfer.outputs_statement.outputs[0].commitment();

            self.pay_from_gas(fee);
            let revealed = ResourceManager::get(self.pumpkin).stealth_transfer(transfer);
            assert!(revealed.is_none(), "A pass must not reveal anything");

            self.live_pumpkin = Some(next);
            self.passes += 1;
            self.last_pass_epoch = Consensus::current_epoch();
            emit_event("CursedPumpkin.Passed", metadata! { "passes" => self.passes.to_string() });
        }

        /// Seal (or re-seal) your guess of who holds the pumpkin at midnight. See [`guess_commitment`].
        pub fn seal_guess(&mut self, commitment: Hash32) {
            self.assert_before_midnight();
            let key = CallerContext::transaction_signer_public_key();
            let i = self.find(&key).expect("Only players may guess");
            self.players[i].sealed_guess = Some(commitment);
            emit_event("CursedPumpkin.GuessSealed", metadata! { "player" => key.to_string() });
        }

        // ---------------------------------------------------------------- after midnight

        /// The holder unmasks. `transfer` must spend the live pumpkin into a revealed output of 1 whose receiver is
        /// the signer's player key. Spending it is the proof of possession; signing is the proof of identity.
        pub fn reveal(&mut self, transfer: StealthTransferStatement) {
            self.assert_reveal_window();
            assert!(self.holder.is_none(), "Already revealed");
            let live = self.live_pumpkin.expect("The pumpkin was never summoned");
            let key = CallerContext::transaction_signer_public_key();
            assert!(self.find(&key).is_some(), "Only a player can be the holder");
            assert!(
                transfer.inputs_statement.inputs.len() == 1 &&
                    transfer.inputs_statement.revealed_amount.is_zero() &&
                    transfer.inputs_statement.inputs[0].commitment == live,
                "The reveal must spend exactly the live pumpkin"
            );
            assert!(transfer.outputs_statement.outputs.is_empty(), "The reveal must not create stealth outputs");
            assert_eq!(
                transfer.revealed_receiver(),
                Some(key),
                "The revealed pumpkin must go to the signer's player key"
            );
            let bucket = ResourceManager::get(self.pumpkin)
                .stealth_transfer(transfer)
                .expect("The reveal must reveal the pumpkin");
            assert_eq!(bucket.amount(), Amount::from(1u64), "That is not the whole pumpkin");
            self.cauldron.deposit(bucket);
            self.live_pumpkin = None;
            self.holder = Some(key);
            emit_event("CursedPumpkin.Revealed", metadata! {
                "holder" => key.to_string(),
                "passes" => self.passes.to_string(),
            });
        }

        /// Open your sealed guess. Wrong or unopened guesses simply do not win.
        pub fn open_guess(&mut self, suspect: RistrettoPublicKeyBytes, salt: Hash32) {
            self.assert_reveal_window();
            let key = CallerContext::transaction_signer_public_key();
            let i = self.find(&key).expect("Only players may open a guess");
            let sealed = self.players[i].sealed_guess.expect("You did not seal a guess");
            assert!(self.players[i].opened_guess.is_none(), "Guess already opened");
            assert!(suspect != key, "You cannot accuse yourself");
            assert!(self.find(&suspect).is_some(), "The suspect is not a player");
            let game = CallerContext::current_component_address();
            let expected = guess_commitment(game.as_bytes(), &key, &suspect, &salt.into_array());
            assert_eq!(expected, sealed, "That does not open your sealed guess");
            self.players[i].opened_guess = Some(suspect);
            emit_event("CursedPumpkin.GuessOpened", metadata! {
                "player" => key.to_string(),
                "suspect" => suspect.to_string(),
            });
        }

        // ---------------------------------------------------------------- settlement

        /// After the reveal window, every player claims once. Returns the payout (possibly empty).
        pub fn claim(&mut self) -> Bucket {
            assert!(
                Consensus::current_epoch() >= self.reveal_until_epoch,
                "Wait until the reveal window closes"
            );
            let key = CallerContext::transaction_signer_public_key();
            let i = self.find(&key).expect("Only players may claim");
            assert!(!self.players[i].claimed, "Already claimed");
            self.players[i].claimed = true;

            let pot = *self.pot_at_settlement.get_or_insert(self.pot.balance());
            let owed = self.entitlement(&key, pot);
            emit_event("CursedPumpkin.Claimed", metadata! {
                "player" => key.to_string(),
                "amount" => owed.to_string(),
            });
            self.pot.withdraw(owed)
        }

        /// What `key` is owed once the window has closed, from a pot of `pot`.
        pub fn entitlement_of(&self, key: RistrettoPublicKeyBytes) -> Amount {
            let pot = self.pot_at_settlement.unwrap_or_else(|| self.pot.balance());
            self.entitlement(&key, pot)
        }

        // ---------------------------------------------------------------- internals

        fn entitlement(&self, key: &RistrettoPublicKeyBytes, pot: Amount) -> Amount {
            let Some(holder) = self.holder else {
                // Nobody unmasked: half the bond back, the rest rots in the pumpkin.
                return self.bond / 2;
            };
            let catchers = self.catcher_keys();
            if catchers.is_empty() {
                // The ghost escapes with everything.
                return if *key == holder { pot } else { Amount::zero() };
            }
            let prize = pot - self.bond;
            let share = prize / catchers.len() as u64;
            let dust = prize - share * catchers.len() as u64;
            if *key == holder {
                self.bond + dust
            } else if catchers.contains(key) {
                share
            } else {
                Amount::zero()
            }
        }

        fn catcher_keys(&self) -> Vec<RistrettoPublicKeyBytes> {
            match self.holder {
                None => Vec::new(),
                Some(h) => self
                    .players
                    .iter()
                    .filter(|p| p.opened_guess == Some(h))
                    .map(|p| p.key)
                    .collect(),
            }
        }

        fn pay_from_gas(&self, fee: Amount) {
            assert!(!fee.is_negative() && fee <= self.max_pass_fee, "Fee above the game's cap");
            if fee.is_positive() {
                self.gas.pay_fee(fee);
            }
        }

        fn find(&self, key: &RistrettoPublicKeyBytes) -> Option<usize> {
            self.players.iter().position(|p| p.key == *key)
        }

        fn assert_before_midnight(&self) {
            assert!(Consensus::current_epoch() < self.midnight_epoch, "Midnight has struck");
        }

        fn assert_reveal_window(&self) {
            let now = Consensus::current_epoch();
            assert!(now >= self.midnight_epoch, "It is not midnight yet");
            assert!(now < self.reveal_until_epoch, "The reveal window has closed");
        }

        fn check_single_hidden_output(t: &StealthTransferStatement) {
            assert!(
                t.outputs_statement.outputs.len() == 1 && t.outputs_statement.revealed_output.is_none(),
                "The pumpkin must go to exactly one hidden owner"
            );
        }
    }

    pub enum Phase {
        /// Before midnight: join, pass, seal guesses.
        Haunting,
        /// After midnight, inside the reveal window: the holder unmasks, guessers open.
        Reveal,
        /// The window has closed: claims only.
        Settled,
    }

    pub struct Player {
        pub key: RistrettoPublicKeyBytes,
        pub sealed_guess: Option<Hash32>,
        pub opened_guess: Option<RistrettoPublicKeyBytes>,
        pub claimed: bool,
    }
}
