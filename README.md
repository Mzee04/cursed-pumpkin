# 🎃 The Cursed Pumpkin

**A stealth hot potato on Tari Ootle.** There is exactly one pumpkin, and it is passed from hand to hand in secret.
The chain records every pass but never who gave it or who received it. At midnight, whoever is holding it has
to prove it on-chain, and everyone who guessed right splits the pot.

Built for the Tari October 2026 build contest, *Spooky Secrets*. All code is new and written for this entry.
Everything runs on the **Esmeralda testnet** with testnet TARI only.

- `templates/cursed_pumpkin`: the Ootle template (Rust → WASM), built with `tari_template_lib` 0.38 for Ootle 0.50.1 (template definition V2). A template built with 0.35 no longer loads.
- `tests/game.rs`: 13 engine tests that run the real Ootle engine in-process
- `client/`: a Python client (ootle-py) that played a real four-player game on Esmeralda, plus `verify_signers.py`, which checks every tx's signers on-chain
- `web/`: a static web app with a replay of that game, a live view read from the indexer, and a Sapient "am I haunted?" check

## How a game goes

| Phase | What happens | What the chain shows |
|---|---|---|
| Create | The host deploys a game. One `PUMPKIN` (a **stealth resource**, supply 1, no mint, no burn) sits in the cauldron vault. | the host's key, the rules |
| Join | Each player locks a bond (1 tTARI) with their public **player key**. | who is playing |
| Summon | The host sends the pumpkin to one player's secret **ghost key**. It becomes a stealth UTXO. | a commitment and a one-time key |
| Pass | The holder spends the UTXO into a new one owned by another player's ghost key. The tx is **sealed by a fresh throwaway key** and **the fee comes from the game's gas tank**. | "a pass happened": old commitment spent, new one created |
| Guess | Any player seals `sha256(domain, game, me, suspect, salt)`. | a hash |
| Midnight | Passes and guesses close. The holder spends the live pumpkin into a revealed output to **their own player key**, back into the cauldron. Only the owner of the live UTXO can produce that spend. | the final holder |
| Open | Guessers open their hashes. | who suspected whom |
| Claim | After the window: right guessers split the other bonds, and the holder gets their own bond back (plus rounding dust). If nobody caught them, the holder takes the whole pot. If the holder never reveals, everyone gets half their bond back. | payouts |

The template checks every rule on-chain:
- Only the host summons, and only once.
- A pass must spend exactly the live commitment into exactly one hidden output, with no revealed value leaking in or out.
- The fee is capped (`max_pass_fee`).
- Nothing moves after midnight.
- The reveal must go to the signer's own player key.
- A guess must open to its sealed hash, and you cannot accuse yourself.

## Privacy: what stays secret, and how

1. **Who holds it.** The pumpkin is an Ootle stealth UTXO: a Pedersen commitment, encrypted data that only the receiver's view key can read, and a one-time spend key. Ghost keys never appear on-chain: the client signs a pass only with the throwaway key and the UTXO's one-time key, and the reveal only with the holder's player key and that one-time key.
2. **Who passed it.** A normal transaction is sealed by its sender, and the sender also pays the fee. Both would give the passer away. Here:
   - each pass is sealed by a never-funded, single-use key;
   - the stealth input is authorised by the UTXO's own one-time key signature;
   - the whole call runs in the fee phase, paying from the game's shared gas tank (`pass(transfer, fee)`). Any unused fee goes back to the tank.

   So the pass links to no account and no player key. `client/verify_signers.py` re-checks this for every tx of the recorded game against the indexer. Spent UTXOs are no longer served by the indexer, so it reads each one-time key from the public stealth output in the transaction that created the UTXO, and checks that the pass (or reveal) is signed by exactly that key.
3. **Who suspects whom.** Guesses are salted hash commitments until midnight.

## Threat model and what still leaks (honestly)

- **Counts and timing.** Everyone sees *that* a pass happened and when. In a small game, timing combined with "who was online" narrows things down.
- **The first hop.** The host summons publicly, so the first holder received it from the host, though which player that is stays hidden. The host also knows who got it first.
- **The receiver knows the sender**, if the sender told them off-chain. Asking someone for their ghost address is itself a social signal. Ghost addresses should be shared in private.
- **The final holder is revealed.** That is the point of midnight.
- **Network metadata.** The indexer sees IP addresses. Use a VPN or Tor if that matters to you.
- **Gas griefing.** Any player can call `pass` with a valid statement, but only the live holder can sign one. The fee is capped per pass. A stalled holder only harms themselves.
- **Self-sabotage.** A holder can move the pumpkin with a plain wallet "send privately" that bypasses `pass`. The game then loses track of it, nobody can reveal, and the holder forfeits their bond (everyone gets half back).
- **Small anonymity set.** With 4 players, guessing is meant to be fun, not impossible. Bigger games give more cover.
- **Client discipline.** The template cannot see who signs a transaction beyond what it checks, so privacy also depends on the client. An earlier version of this client also signed passes with the passer's ghost key (ootle-py signs with every key registered in the wallet), which would let anyone who knows the ghost addresses follow the hands. That was fixed before the game below was played, and the game recorded here was re-run from scratch with the fixed client; `verify_signers.py` shows no ghost key in any of its transactions.
- **Testnet only.** The code is unaudited. Do not use it with real funds.

## Run it

### Web app (no wallet needed)
```bash
cd web && python3 -m http.server 8000
# open http://localhost:8000
```
- **Replay** steps through the recorded Esmeralda game, showing *what the public sees* next to *what the players know*.
- **Live game** reads the game component straight from `https://ootle-indexer-a.tari.com`. The indexer now returns substate values as CBOR; `web/bor.js` decodes them with the vendored `@tari-project/ootle-wasm-bor` 0.50.1 blob.
- **Am I haunted?** needs the **Sapient** extension on Esmeralda. It scans your wallet for this game's PUMPKIN and signs an ownership challenge with the pumpkin's one-time key, then checks that the key matches the live UTXO's on-chain `auth.Key`. It spends nothing.

### Template tests
```bash
rustup target add wasm32-unknown-unknown
cargo test            # 13 tests: full games, cheating attempts, fee cap, payouts
```

### Build the WASM
```bash
cd templates/cursed_pumpkin && cargo build --release --target wasm32-unknown-unknown
```

### Play on Esmeralda with the Python client
```bash
python3 -m venv .venv && .venv/bin/pip install -r client/requirements.txt   # Python 3.13, ootle-py 0.5.0
E=".venv/bin/python client/esmeralda.py"
$E keys                     # four throwaway players (stored in secrets/, git-ignored)
# The genesis faucet is empty after the November 2026 reset (`tap` tries it and says so).
# Claim testnet tTARI from https://testnet-faucet.taritracker.com/ as a public payout to the host address `keys` prints.
$E publish 30000000         # publish the template (this rebuild paid about 6.3 tTARI)
$E create 1 1               # midnight in 1 epoch, reveal window of 1 epoch
$E gas; for n in Vampire Mummy Witch Ghost; do $E join $n; done
$E summon Witch; $E pass Vampire; $E pass Ghost; $E pass Mummy
$E guess Vampire Mummy; $E guess Witch Ghost; $E guess Ghost Mummy; $E guess Mummy Vampire
# after the midnight epoch:
$E reveal; for n in Vampire Witch Ghost Mummy; do $E open $n; done
# after the window:
for n in Mummy Vampire Ghost Witch; do $E claim $n; done
.venv/bin/python client/verify_signers.py   # who signed what, from the indexer
```
Every step is appended to `web/data/recorded-game.json`, which the replay reads.

## The recorded Esmeralda game

Played on 7 November 2026 (created in epoch 12942, revealed in 12943, claims from 12944) after the Esmeralda reset, with this template rebuilt for Ootle 0.50.1. The genesis faucet vault is empty, so the host was funded from the TariTracker community faucet and then paid the other three players. Witch got the pumpkin, then it went to Vampire, Ghost and finally Mummy, who revealed at midnight. Vampire and Ghost guessed Mummy and split the other three bonds (1.5 tTARI each); Mummy got their own bond back; Witch's bond was forfeited. `client/verify_signers.py` passes for all 23 transactions: no ghost key appears anywhere, each pass is sealed by a single-use key and signed only by the spent UTXO's one-time key, and the reveal is signed by Mummy's player key plus that one-time key.

<!-- LINKS -->
- Template: `template_cb295c2957234e85ed7de45c45e5c024b5351ff23a3737ffba464b1673d00343` ([publish tx](https://explorer.tari.mw/tx/27d48b78458d3959c7d56a7f80df6c6e59a1ef4e4e9a66482ae2f98bbb1c94cd))
- Game component: `component_f0e06041353d946f32994793a7062d485615e929cbefe7ba1eb382be68814ddf`
- PUMPKIN resource: `resource_f0003b31428c78cf6622658a9d3bab884b2db890294ad8793f69c12b6a3acf69`
- Midnight epoch 12943, claims from epoch 12944

| # | Step | Public view | Tx |
|---|---|---|---|
| 1 | create | game created; one PUMPKIN minted into the cauldron; midnight at epoch 12943 | [af135c385b…](https://explorer.tari.mw/tx/af135c385bf3acbb70a4125efba80b46f194a790a86f847247d62a8d1ce946bb) |
| 2 | add_gas | 2.0 tTARI into the gas tank that pays for anonymous passes | [a0c6e808e1…](https://explorer.tari.mw/tx/a0c6e808e1c401c814eb816608240c5da451bd6e6c34f3ff65811f16c63ed197) |
| 3 | join Vampire | joined and locked a 1 tTARI bond | [4d82cd14ff…](https://explorer.tari.mw/tx/4d82cd14ff668f90a0037600dfb5364c1a51eb3dd7ddf26e8c0cdc6cd5def50c) |
| 4 | join Mummy | joined and locked a 1 tTARI bond | [7812ce4165…](https://explorer.tari.mw/tx/7812ce416518c85aab644276a5171e13d71ce52eb145b2bf470216d943d3d215) |
| 5 | join Witch | joined and locked a 1 tTARI bond | [fa68d1f4cd…](https://explorer.tari.mw/tx/fa68d1f4cd22523c75801576329249bce27fd24dde25772d2bed65aedd7ed30b) |
| 6 | join Ghost | joined and locked a 1 tTARI bond | [c40815b833…](https://explorer.tari.mw/tx/c40815b83398f8c0895ca8e714d03b5b9207480faebb37317601834d0a6c9a89) |
| 7 | summon | the pumpkin left the cauldron into a hidden UTXO | [23b71c95fd…](https://explorer.tari.mw/tx/23b71c95fd247db754601988dcf82bf182b80589da4b0f0c008479739af00017) |
| 8 | pass Witch->Vampire | the pumpkin moved: old commitment spent, new hidden owner | [f9f3d6027b…](https://explorer.tari.mw/tx/f9f3d6027b580ed193dfae7106957cdd0d1e602c82b7904c4625913455ce8780) |
| 9 | pass Vampire->Ghost | the pumpkin moved: old commitment spent, new hidden owner | [9d404d6125…](https://explorer.tari.mw/tx/9d404d61254117aae6583e8aa2cf5890766d2e76d5ab21e8d03032a50a3b9c14) |
| 10 | pass Ghost->Mummy | the pumpkin moved: old commitment spent, new hidden owner | [da9a3bfb44…](https://explorer.tari.mw/tx/da9a3bfb44cc07d6ba3c2c1caa72f57c2e8eea95ed1544c4d84866c398b90b72) |
| 11 | seal guess Vampire | sealed a guess (a hash; the suspect stays secret) | [7939edbfbe…](https://explorer.tari.mw/tx/7939edbfbeb7dcd3294e09d7efc39cc9a3e250bd76f06ea88d635c08d99beda9) |
| 12 | seal guess Witch | sealed a guess (a hash; the suspect stays secret) | [f565bef690…](https://explorer.tari.mw/tx/f565bef69016c555068b0331598a2c68644bdcf9f6e1e15b6a64d789a71f186b) |
| 13 | seal guess Ghost | sealed a guess (a hash; the suspect stays secret) | [2e99841048…](https://explorer.tari.mw/tx/2e99841048bf1c5b542f733399af609209374684b34b094d31bc937442714bfd) |
| 14 | seal guess Mummy | sealed a guess (a hash; the suspect stays secret) | [9bd81eb4ab…](https://explorer.tari.mw/tx/9bd81eb4abafd2e2e6a5a2e79f41b5b57a1492574a6219dadcf242490565525e) |
| 15 | reveal Mummy | midnight: the holder proved ownership by spending the live pumpkin back into the cauldron | [f15c4ba626…](https://explorer.tari.mw/tx/f15c4ba6265f62733c6325cfa1aeef6a2f01e3924aa164ab046e6fdabe23e31b) |
| 16 | open guess Vampire | opened their guess: suspect 94170f8a1e89… | [1e0c5e8397…](https://explorer.tari.mw/tx/1e0c5e8397708beaec2e807f588782920ac0939d87f915c1a1b26606efbebad1) |
| 17 | open guess Witch | opened their guess: suspect 1425eb2ee318… | [9cf2205c09…](https://explorer.tari.mw/tx/9cf2205c096e514cabcc51ca45d4a2f9672b43d9ea3f63f71ddc20667c9a1460) |
| 18 | open guess Ghost | opened their guess: suspect 94170f8a1e89… | [52887d3712…](https://explorer.tari.mw/tx/52887d3712f313fbcda52fa01cbf0d78d64d1a64b5f7124f94dff3e44f3677a7) |
| 19 | open guess Mummy | opened their guess: suspect 706d47c8204c… | [58e5607df9…](https://explorer.tari.mw/tx/58e5607df9fbfaf459a2857c16ec0ff9184b2ac846ed37c4a1f170685d526a11) |
| 20 | claim Mummy | claimed their own bond back (the holder) | [4e344b0e54…](https://explorer.tari.mw/tx/4e344b0e5499a16cc7d3e57333566a7bc7cfd387bc2e6112aefb6ca3186df906) |
| 21 | claim Vampire | claimed their share of the pot | [0738b7f943…](https://explorer.tari.mw/tx/0738b7f94329a823350120557081c0e625ae409723079dad9bdc3b32bd487959) |
| 22 | claim Ghost | claimed their share of the pot | [46fe5a824b…](https://explorer.tari.mw/tx/46fe5a824b65780352d4a88326f819c1021a07aa70b3500d47b1028e57b859f8) |
| 23 | claim Witch | claimed nothing back (bond forfeited to the right guessers); paid only the fee | [1a549053bf…](https://explorer.tari.mw/tx/1a549053bfedfe789ffd6201dd86d975105253c6ac246b944e8b581ecf5a3f1e) |
<!-- /LINKS -->

## License
MIT. See [LICENSE](LICENSE).
