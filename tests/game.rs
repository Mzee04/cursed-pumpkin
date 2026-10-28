//! Engine tests for The Cursed Pumpkin, run in-process against the real Ootle engine.
//!
//! In these tests a pumpkin UTXO is spendable by whoever knows its blinding mask (key path from the mask), so "holding
//! the pumpkin" means "knowing the mask of the live UTXO". Every refusal test checks the exact reason.

use tari_template_lib::types::{
    Amount, ComponentAddress, Hash32, ResourceAddress,
    crypto::RistrettoPublicKeyBytes,
    stealth::StealthTransferStatement,
};
use tari_template_test_tooling::{
    TemplateTest,
    byte_type::ToByteType,
    crypto::{PublicKey as _, RistrettoPublicKey, RistrettoSecretKey},
    engine_types::virtual_substate::{VirtualSubstate, VirtualSubstateId},
    support::{
        spec::{InputSpec, OutputSpec},
        stealth::{self, NO_INPUTS},
    },
    transaction::{Transaction, args, builder::named_args::NamedArg},
    wallet_crypto::MaskAndValue,
};

const BOND: u64 = 1_000;
const START: u64 = 10;
const MIDNIGHT: u64 = 20;
const WINDOW: u64 = 5;

struct Player {
    sk: RistrettoSecretKey,
    pk: RistrettoPublicKeyBytes,
    account: ComponentAddress,
}

struct World {
    test: TemplateTest,
    coin: ComponentAddress,
    coin_res: ResourceAddress,
    game: ComponentAddress,
    pumpkin: ResourceAddress,
    players: Vec<Player>,
    /// Mask of the live pumpkin UTXO (whoever knows it holds the pumpkin).
    live_mask: RistrettoSecretKey,
}

fn set_epoch(test: &mut TemplateTest, epoch: u64) {
    test.set_virtual_substate(VirtualSubstateId::CurrentEpoch, VirtualSubstate::CurrentEpoch(epoch));
}

fn tx(w: &World, component: ComponentAddress, method: &str, a: Vec<NamedArg>, signer: &RistrettoSecretKey) -> Transaction {
    w.test.transaction().call_method(component, method, a).finish().seal(signer)
}

fn salt(i: u8) -> [u8; 32] {
    [i; 32]
}

/// The off-chain half of a sealed guess; must match `cursed_pumpkin::guess_commitment`.
fn commitment(game: &ComponentAddress, guesser: &RistrettoPublicKeyBytes, suspect: &RistrettoPublicKeyBytes, s: &[u8; 32]) -> Hash32 {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"cursed-pumpkin/guess/v1");
    h.update(game.as_bytes());
    h.update(guesser.as_ref());
    h.update(suspect.as_ref());
    h.update(s);
    Hash32::from_array(h.finalize().into())
}

/// A game with `n` players (player 0 is the host, who receives the pumpkin) who have all joined.
fn world(n: usize) -> World {
    let mut test = TemplateTest::new(env!("CARGO_MANIFEST_DIR"), ["templates/cursed_pumpkin", "templates/test_coin"]);
    test.enable_auto_add_proofs_from_signers();
    set_epoch(&mut test, START);

    let coin: ComponentAddress = test.call_function("TestCoin", "new", args![Amount::from(1_000_000u64)], vec![]);
    let coin_res: ResourceAddress = test.call_method(coin, "resource", args![], vec![]);

    let mut players = Vec::new();
    for _ in 0..n {
        let (account, _proof, sk) = test.create_empty_account();
        let pk = RistrettoPublicKey::from_secret_key(&sk).to_byte_type();
        players.push(Player { sk, pk, account });
    }

    let host_sk = players[0].sk.clone();
    let t = test
        .transaction()
        .call_function(
            test.get_template_address("CursedPumpkin"),
            "new",
            args!["Test night", coin_res, Amount::from(BOND), MIDNIGHT, WINDOW, Amount::from(2_000u64)],
        )
        .finish()
        .seal(&host_sk);
    let result = test.execute_expect_success(t, vec![]);
    let game = result
        .finalize
        .result
        .accept()
        .unwrap()
        .up_iter()
        .find_map(|(id, _)| id.as_component_address().filter(|a| *a != coin))
        .expect("game component");
    let pumpkin: ResourceAddress = test.call_method(game, "pumpkin_resource", args![], vec![]);

    // The host summons the pumpkin out of the cauldron into one hidden UTXO (for themselves, here).
    let summon = stealth::generate_transfer_data(NO_INPUTS, 1u64, [OutputSpec::key_path_from_mask(1)], 0u64);
    let t = test
        .transaction()
        .call_method(game, "summon", args![summon.statement.clone(), Amount::zero()])
        .finish()
        .seal(&host_sk);
    test.execute_expect_success(t, vec![]);

    let mut w = World {
        test,
        coin,
        coin_res,
        game,
        pumpkin,
        players,
        live_mask: summon.output_masks[0].clone(),
    };
    for i in 0..n {
        join(&mut w, i, BOND).expect("join");
    }
    w
}

fn join(w: &mut World, i: usize, amount: u64) -> Result<(), String> {
    let t = w
        .test
        .transaction()
        .call_method(w.coin, "take", args![Amount::from(amount)])
        .put_last_instruction_output_on_workspace("bond")
        .call_method(w.game, "join", args![Workspace("bond")])
        .finish()
        .seal(&w.players[i].sk);
    run(w, t)
}

fn run(w: &mut World, t: Transaction) -> Result<(), String> {
    let r = w.test.execute_and_commit_on_success(t, vec![]);
    match r.finalize.result.any_reject() {
        Some(reason) => Err(reason.to_string()),
        None => Ok(()),
    }
}

/// Build a pass statement from the live pumpkin into a fresh hidden UTXO, spendable by `next_mask`'s owner.
fn pass_statement(mask: &RistrettoSecretKey) -> (StealthTransferStatement, RistrettoSecretKey) {
    let input = InputSpec::new(MaskAndValue { mask: mask.clone(), value: 1 });
    let data = stealth::generate_transfer_data([input], 0u64, [OutputSpec::key_path_from_mask(1)], 0u64);
    (data.statement, data.output_masks[0].clone())
}

/// Pass the pumpkin. The passer's identity never appears.
fn pass(w: &mut World) -> Result<(), String> {
    // The only key that signs is the UTXO's own one-time spend key: nothing links the pass to a player.
    let mask = w.live_mask.clone();
    let (statement, next) = pass_statement(&mask);
    let t = tx(w, w.game, "pass", args![statement, Amount::zero()], &mask);
    run(w, t)?;
    w.live_mask = next;
    Ok(())
}

fn reveal_statement(mask: &RistrettoSecretKey, receiver: RistrettoPublicKeyBytes) -> StealthTransferStatement {
    let input = InputSpec::new(MaskAndValue { mask: mask.clone(), value: 1 });
    stealth::generate_transfer_data_to([input], 0u64, Vec::<OutputSpec>::new(), 1u64, receiver).statement
}

/// A reveal is signed twice: by the UTXO's spend key (possession) and, as the sealing signer, by the player key
/// (identity).
fn reveal_tx(w: &World, st: StealthTransferStatement, utxo_key: &RistrettoSecretKey, player_sk: &RistrettoSecretKey) -> Transaction {
    let player_pk: RistrettoPublicKeyBytes = RistrettoPublicKey::from_secret_key(player_sk).to_byte_type();
    w.test
        .transaction()
        .call_method(w.game, "reveal", args![st])
        .add_signer(&player_pk, utxo_key)
        .seal(player_sk)
}

fn reveal_as(w: &mut World, i: usize) -> Result<(), String> {
    let mask = w.live_mask.clone();
    let st = reveal_statement(&mask, w.players[i].pk);
    let t = reveal_tx(w, st, &mask, &w.players[i].sk.clone());
    run(w, t)
}

fn seal(w: &mut World, guesser: usize, suspect: usize) -> Result<(), String> {
    let c = commitment(&w.game, &w.players[guesser].pk, &w.players[suspect].pk, &salt(guesser as u8));
    let t = tx(w, w.game, "seal_guess", args![c], &w.players[guesser].sk.clone());
    run(w, t)
}

fn open(w: &mut World, guesser: usize, suspect: usize) -> Result<(), String> {
    let t = tx(
        w,
        w.game,
        "open_guess",
        args![w.players[suspect].pk, Hash32::from_array(salt(guesser as u8))],
        &w.players[guesser].sk.clone(),
    );
    run(w, t)
}

fn claim(w: &mut World, i: usize) -> Result<(), String> {
    let t = w
        .test
        .transaction()
        .call_method(w.game, "claim", args![])
        .put_last_instruction_output_on_workspace("payout")
        .call_method(w.players[i].account, "deposit", args![Workspace("payout")])
        .finish()
        .seal(&w.players[i].sk);
    run(w, t)
}

fn balance(w: &mut World, i: usize) -> Amount {
    let account = w.players[i].account;
    let res = w.coin_res;
    w.test.call_method(account, "balance", args![res], vec![])
}

fn expect_err(r: Result<(), String>, needle: &str) {
    match r {
        Ok(()) => panic!("expected failure containing {needle:?}, got success"),
        Err(e) => assert!(e.contains(needle), "expected {needle:?} in {e:?}"),
    }
}

// ----------------------------------------------------------------------------------------------------------- tests

#[test]
fn ghost_is_caught_and_catchers_split_the_pot() {
    let mut w = world(4);
    pass(&mut w).unwrap(); // host -> someone
    pass(&mut w).unwrap(); // -> player 2 (in the story)
    let passes: u64 = w.test.call_method(w.game, "passes", args![], vec![]);
    assert_eq!(passes, 2);

    seal(&mut w, 0, 2).unwrap();
    seal(&mut w, 1, 2).unwrap();
    seal(&mut w, 2, 3).unwrap();
    seal(&mut w, 3, 1).unwrap();

    set_epoch(&mut w.test, MIDNIGHT);
    reveal_as(&mut w, 2).unwrap();
    for (g, s) in [(0, 2), (1, 2), (2, 3), (3, 1)] {
        open(&mut w, g, s).unwrap();
    }
    let holder: Option<RistrettoPublicKeyBytes> = w.test.call_method(w.game, "holder", args![], vec![]);
    assert_eq!(holder, Some(w.players[2].pk));

    set_epoch(&mut w.test, MIDNIGHT + WINDOW);
    for i in 0..4 {
        claim(&mut w, i).unwrap();
    }
    // Pot of 4 bonds: the caught ghost gets their bond back, the two catchers split the other three.
    assert_eq!(balance(&mut w, 2), Amount::from(BOND));
    assert_eq!(balance(&mut w, 0), Amount::from(1_500u64));
    assert_eq!(balance(&mut w, 1), Amount::from(1_500u64));
    assert_eq!(balance(&mut w, 3), Amount::zero());
    let pot: Amount = w.test.call_method(w.game, "pot_balance", args![], vec![]);
    assert_eq!(pot, Amount::zero());
}

#[test]
fn ghost_who_escapes_takes_everything() {
    let mut w = world(3);
    pass(&mut w).unwrap();
    seal(&mut w, 0, 2).unwrap(); // wrong: player 1 will reveal
    seal(&mut w, 2, 0).unwrap();
    set_epoch(&mut w.test, MIDNIGHT + 1);
    reveal_as(&mut w, 1).unwrap();
    open(&mut w, 0, 2).unwrap();
    open(&mut w, 2, 0).unwrap();
    set_epoch(&mut w.test, MIDNIGHT + WINDOW + 3);
    for i in 0..3 {
        claim(&mut w, i).unwrap();
    }
    assert_eq!(balance(&mut w, 1), Amount::from(3 * BOND));
    assert_eq!(balance(&mut w, 0), Amount::zero());
    assert_eq!(balance(&mut w, 2), Amount::zero());
}

#[test]
fn no_reveal_means_half_back_and_the_rest_rots() {
    let mut w = world(2);
    pass(&mut w).unwrap();
    set_epoch(&mut w.test, MIDNIGHT + WINDOW);
    expect_err(reveal_as(&mut w, 1), "The reveal window has closed");
    claim(&mut w, 0).unwrap();
    claim(&mut w, 1).unwrap();
    assert_eq!(balance(&mut w, 0), Amount::from(BOND / 2));
    assert_eq!(balance(&mut w, 1), Amount::from(BOND / 2));
    expect_err(claim(&mut w, 1), "Already claimed");
}

#[test]
fn midnight_freezes_the_pumpkin() {
    let mut w = world(2);
    set_epoch(&mut w.test, MIDNIGHT);
    expect_err(pass(&mut w), "Midnight has struck");
    expect_err(seal(&mut w, 0, 1), "Midnight has struck");
    expect_err(join(&mut w, 0, BOND), "Midnight has struck");
}

#[test]
fn only_the_holder_can_pass() {
    let mut w = world(2);
    // A stale mask (the host's, after the host passed) cannot move the pumpkin.
    let stale = w.live_mask.clone();
    pass(&mut w).unwrap();
    let (st, _) = pass_statement(&stale);
    let t = tx(&w, w.game, "pass", args![st, Amount::zero()], &stale);
    expect_err(run(&mut w, t), "A pass must spend exactly the live pumpkin");

    // A made-up UTXO is not the pumpkin either.
    let (bogus, _) = w.test.new_key_pair(202);
    let (st, _) = pass_statement(&bogus);
    let t = tx(&w, w.game, "pass", args![st, Amount::zero()], &bogus);
    expect_err(run(&mut w, t), "A pass must spend exactly the live pumpkin");
}

#[test]
fn a_pass_cannot_split_or_reveal_the_pumpkin() {
    let mut w = world(2);
    let input = InputSpec::new(MaskAndValue { mask: w.live_mask.clone(), value: 1 });
    let data = stealth::generate_transfer_data([input], 0u64, Vec::<OutputSpec>::new(), 1u64);
    let mask = w.live_mask.clone();
    let t = tx(&w, w.game, "pass", args![data.statement, Amount::zero()], &mask);
    expect_err(run(&mut w, t), "The pumpkin must go to exactly one hidden owner");
}

#[test]
fn a_pumpkin_moved_behind_the_templates_back_cannot_be_revealed() {
    let mut w = world(2);
    // Move the pumpkin with a plain stealth transfer instead of `pass`.
    let mask = w.live_mask.clone();
    let (st, next) = pass_statement(&mask);
    let pumpkin = w.pumpkin;
    let t = w.test.transaction().stealth_transfer(pumpkin, st).finish().seal(&mask);
    run(&mut w, t).unwrap();
    w.live_mask = next;
    set_epoch(&mut w.test, MIDNIGHT);
    expect_err(reveal_as(&mut w, 1), "The reveal must spend exactly the live pumpkin");
}

#[test]
fn reveal_rules() {
    let mut w = world(3);
    pass(&mut w).unwrap();
    // Too early.
    expect_err(reveal_as(&mut w, 1), "It is not midnight yet");
    set_epoch(&mut w.test, MIDNIGHT);
    // A non-player cannot claim to be the ghost, even holding the pumpkin.
    let (stranger_sk, stranger_pk) = w.test.new_key_pair(204);
    let mask = w.live_mask.clone();
    let st = reveal_statement(&mask, stranger_pk.to_byte_type());
    let t = reveal_tx(&w, st, &mask, &stranger_sk);
    expect_err(run(&mut w, t), "Only a player can be the holder");
    // The revealed pumpkin must go to the signer, not to someone else.
    let st = reveal_statement(&mask, w.players[2].pk);
    let t = reveal_tx(&w, st, &mask, &w.players[1].sk.clone());
    expect_err(run(&mut w, t), "The revealed pumpkin must go to the signer's player key");
    // Knowing who holds it is not enough: you need the UTXO itself.
    let (wrong, _) = w.test.new_key_pair(205);
    let st = reveal_statement(&wrong, w.players[1].pk);
    let t = reveal_tx(&w, st, &wrong, &w.players[1].sk.clone());
    expect_err(run(&mut w, t), "The reveal must spend exactly the live pumpkin");
    // The real holder succeeds, once.
    reveal_as(&mut w, 1).unwrap();
    expect_err(reveal_as(&mut w, 1), "Already revealed");
}

#[test]
fn guess_rules() {
    let mut w = world(3);
    seal(&mut w, 0, 1).unwrap();
    seal(&mut w, 0, 2).unwrap(); // re-sealing replaces the guess
    set_epoch(&mut w.test, MIDNIGHT);
    expect_err(open(&mut w, 0, 1), "That does not open your sealed guess");
    open(&mut w, 0, 2).unwrap();
    expect_err(open(&mut w, 0, 2), "Guess already opened");
    expect_err(open(&mut w, 1, 2), "You did not seal a guess");
    // Self-accusation is refused even with a matching commitment.
    set_epoch(&mut w.test, START);
    seal(&mut w, 1, 1).unwrap();
    set_epoch(&mut w.test, MIDNIGHT);
    expect_err(open(&mut w, 1, 1), "You cannot accuse yourself");
}

#[test]
fn join_rules() {
    let mut w = world(2);
    expect_err(join(&mut w, 0, BOND), "Already joined");
    let (account, _, sk) = w.test.create_empty_account();
    let pk = RistrettoPublicKey::from_secret_key(&sk).to_byte_type();
    w.players.push(Player { sk, pk, account });
    expect_err(join(&mut w, 2, BOND - 1), "The bond must be exactly the game's bond");
    join(&mut w, 2, BOND).unwrap();
    let players: Vec<RistrettoPublicKeyBytes> = w.test.call_method(w.game, "players", args![], vec![]);
    assert_eq!(players.len(), 3);
}

#[test]
fn claims_wait_for_the_window_to_close() {
    let mut w = world(2);
    set_epoch(&mut w.test, MIDNIGHT);
    expect_err(claim(&mut w, 0), "Wait until the reveal window closes");
}

#[test]
fn only_the_host_summons_and_only_once() {
    let mut w = world(2);
    let again = stealth::generate_transfer_data(NO_INPUTS, 1u64, [OutputSpec::key_path_from_mask(1)], 0u64);
    let t = tx(&w, w.game, "summon", args![again.statement.clone(), Amount::zero()], &w.players[1].sk.clone());
    expect_err(run(&mut w, t), "Only the host can summon the pumpkin");
    let t = tx(&w, w.game, "summon", args![again.statement, Amount::zero()], &w.players[0].sk.clone());
    expect_err(run(&mut w, t), "The pumpkin has already been summoned");
}

#[test]
fn pass_fees_are_capped_and_come_only_from_the_gas_tank() {
    let mut w = world(2);
    let mask = w.live_mask.clone();
    // Above the game's cap (2_000): refused before anything is spent.
    let (st, _) = pass_statement(&mask);
    let t = tx(&w, w.game, "pass", args![st, Amount::from(2_001u64)], &mask);
    expect_err(run(&mut w, t), "Fee above the game's cap");
    // Within the cap, but the gas tank is empty: the fee cannot be drawn from anywhere else, so the pass fails.
    let (st, _) = pass_statement(&mask);
    let t = tx(&w, w.game, "pass", args![st, Amount::from(1_000u64)], &mask);
    assert!(run(&mut w, t).is_err(), "an empty gas tank must not pay for a pass");
    // The pumpkin did not move: a zero-fee pass of the same UTXO still works.
    pass(&mut w).unwrap();
    let passes: u64 = w.test.call_method(w.game, "passes", args![], vec![]);
    assert_eq!(passes, 1);
}
