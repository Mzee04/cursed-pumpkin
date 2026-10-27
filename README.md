# 🎃 The Cursed Pumpkin

**A stealth hot potato on Tari Ootle.** There is exactly one pumpkin, and it is passed from hand to hand in secret.
The chain records every pass but never who gave it or who received it. At midnight, whoever is holding it has
to prove it on-chain, and everyone who guessed right splits the pot.

Built for the Tari October 2026 build contest, *Spooky Secrets*. All code is new and written for this entry.
Everything runs on the **Esmeralda testnet** with testnet TARI only.

- `templates/cursed_pumpkin`: the Ootle template (Rust → WASM)
- `tests/game.rs`: 12 engine tests that run the real Ootle engine in-process
- `client/`: a Python client (ootle-py) that played a real four-player game on Esmeralda
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

1. **Who holds it.** The pumpkin is an Ootle stealth UTXO: a Pedersen commitment, encrypted data that only the receiver's view key can read, and a one-time spend key. Ghost keys never appear on-chain.
2. **Who passed it.** A normal transaction is sealed by its sender, and the sender also pays the fee. Both would give the passer away. Here:
   - each pass is sealed by a never-funded, single-use key;
   - the stealth input is authorised by the UTXO's own one-time key signature;
   - the whole call runs in the fee phase, paying from the game's shared gas tank (`pass(transfer, fee)`). Any unused fee goes back to the tank.

   So the pass links to no account and no player key.
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
- **Testnet only.** The code is unaudited. Do not use it with real funds.

## Run it

### Web app (no wallet needed)
```bash
cd web && python3 -m http.server 8000
# open http://localhost:8000
```
- **Replay** steps through the recorded Esmeralda game, showing *what the public sees* next to *what the players know*.
- **Live game** reads the game component straight from `https://ootle-indexer-a.tari.com`.
- **Am I haunted?** needs the **Sapient** extension on Esmeralda. It scans your wallet for this game's PUMPKIN and signs an ownership challenge with the pumpkin's one-time key, then checks that the key matches the live UTXO's on-chain `auth.Key`. It spends nothing.

### Template tests
```bash
rustup target add wasm32-unknown-unknown
cargo test            # 12 tests: full games, cheating attempts, payouts
```

### Build the WASM
```bash
cd templates/cursed_pumpkin && cargo build --release --target wasm32-unknown-unknown
```

### Play on Esmeralda with the Python client
```bash
python3 -m venv .venv && .venv/bin/pip install -r client/requirements.txt
E=".venv/bin/python client/esmeralda.py"
$E keys                     # four throwaway players (stored in secrets/, git-ignored)
$E tap Vampire 6                # testnet TARI from a community tap (5 tTARI per call)
$E publish 9000000          # publish the template (needs about 6.4 tTARI in fees)
$E create 1 2               # midnight in 1 epoch, reveal window of 2 epochs
$E gas; for n in Vampire Mummy Witch Ghost; do $E join $n; done
$E summon Mummy; $E pass Witch; $E pass Ghost; $E pass Witch
$E guess Vampire Witch; $E guess Mummy Witch; $E guess Ghost Mummy; $E guess Witch Vampire
# after the midnight epoch:
$E reveal; for n in Vampire Mummy Ghost Witch; do $E open $n; done
# after the window:
for n in Witch Vampire Mummy Ghost; do $E claim $n; done
```
Every step is appended to `web/data/recorded-game.json`, which the replay reads.

## The recorded Esmeralda game

<!-- LINKS -->
- Template: `template_7660acdce0b99082374f3488654af0ea4cd7e01a2e912b84d53ccce0ead9e1d9` ([publish tx](https://explorer.tari.mw/tx/8991ab11019d61e6bb1664444c0d37617d2477977845d53dcd4d1da9959da477))
- Game component: `component_7526be50b34ce0de312a6e5f314300a2243719ab1c0eb931dd897a2256c5ac32`
- PUMPKIN resource: `resource_75b25f3f217738815fbe2c9426e9bb7f13937da9b207dff64ce85b82d4833a07`
- Midnight epoch 12684, claims from epoch 12686

| # | Step | Public view | Tx |
|---|---|---|---|
| 1 | create | game created; one PUMPKIN minted into the cauldron; midnight at epoch 12684 | [2c99b81e13…](https://explorer.tari.mw/tx/2c99b81e130dc2c3f36701b1d2004d40f2357583f5d5802d1202c4eea0cd68bc) |
| 2 | add_gas | 2.0 tTARI into the gas tank that pays for anonymous passes | [cdc0a73506…](https://explorer.tari.mw/tx/cdc0a735060599f61f0859cb37f8c17f1172b6b8c527cb0a1855937f7fbbc6b2) |
| 3 | join Vampire | joined and locked a 1 tTARI bond | [201f0eaaa8…](https://explorer.tari.mw/tx/201f0eaaa8fbd3378e73234505036207b06a28f25e70f10abb92bf919e856ef1) |
| 4 | join Mummy | joined and locked a 1 tTARI bond | [774fe51d0e…](https://explorer.tari.mw/tx/774fe51d0e5ebfa42de1d3d2263eca7f80fb0fa918f57471fdc7b57918b78677) |
| 5 | join Witch | joined and locked a 1 tTARI bond | [52c4663b5f…](https://explorer.tari.mw/tx/52c4663b5f743b28f770d6629e9eecac7966f84467f5c3d85d7b6708d53662b7) |
| 6 | join Ghost | joined and locked a 1 tTARI bond | [c08e6fccc8…](https://explorer.tari.mw/tx/c08e6fccc887eb0bed2d41a2d27da49fb2d60f4d99a094670e77a78e3255d25a) |
| 7 | summon | the pumpkin left the cauldron into a hidden UTXO | [2de6a50f3d…](https://explorer.tari.mw/tx/2de6a50f3da102b09fd972ef7476dd270850fc6c642f0b9cdd8e62f44e12dba2) |
| 8 | pass Mummy->Witch | the pumpkin moved: old commitment spent, new hidden owner | [5588bd45d8…](https://explorer.tari.mw/tx/5588bd45d8f8a18557de67bd58387d839b79b4cdda64769a2a6904ec1b3570f0) |
| 9 | pass Witch->Ghost | the pumpkin moved: old commitment spent, new hidden owner | [7470860d26…](https://explorer.tari.mw/tx/7470860d26b01d5eee6bc9be859412c6193e9c8f549797ee743387e7687c2efa) |
| 10 | pass Ghost->Witch | the pumpkin moved: old commitment spent, new hidden owner | [9f9135273a…](https://explorer.tari.mw/tx/9f9135273ac46abb35c1909a878d79b69f0f507dce9500019ab334769dfca3bd) |
| 11 | seal guess Vampire | sealed a guess (a hash; the suspect stays secret) | [8d969309b2…](https://explorer.tari.mw/tx/8d969309b29567ba0a26893d26b74090a13502d285faf71835e9f012d5b7423c) |
| 12 | seal guess Mummy | sealed a guess (a hash; the suspect stays secret) | [e74eccde3a…](https://explorer.tari.mw/tx/e74eccde3afeaf7a04d59a53e9826839a7d742597eca0cbc4c33340e2aa7acb0) |
| 13 | seal guess Ghost | sealed a guess (a hash; the suspect stays secret) | [b661e3a177…](https://explorer.tari.mw/tx/b661e3a17716f82733c13bf81f683a713bbad45723d968da6410395a84226343) |
| 14 | seal guess Witch | sealed a guess (a hash; the suspect stays secret) | [4131f3b31c…](https://explorer.tari.mw/tx/4131f3b31c2baace187e50f359b1c6d1bdfbae87bf0df216b41e0f03ab72fc53) |
| 15 | open guess Ghost | opened their guess: suspect d604df43dad1… | [b5330f34e1…](https://explorer.tari.mw/tx/b5330f34e132428a1db08452dfff6b23a34a11adde96293f3fb58692710f90bd) |
| 16 | open guess Witch | opened their guess: suspect 52ba27bb2725… | [006bc38ba3…](https://explorer.tari.mw/tx/006bc38ba3ee9d8b1bc3fa3fff8f48f6ca317498ca2bc8cfc3ca53d7b5cfad9b) |
| 17 | reveal Witch | midnight: the holder proved ownership by spending the live pumpkin back into the cauldron | [88f8bd0625…](https://explorer.tari.mw/tx/88f8bd0625e40b7cc8572afc77c69f191a8e064f7e9ea9a791a01532062ac89b) |
| 18 | open guess Vampire | opened their guess: suspect b8e4845b14ec… | [b8311c9d73…](https://explorer.tari.mw/tx/b8311c9d73e4303a61f5944690e6019a59c8171a04d6cd5e9e61baf75aae0414) |
| 19 | open guess Mummy | opened their guess: suspect b8e4845b14ec… | [84dda6cbc6…](https://explorer.tari.mw/tx/84dda6cbc68cf6f6d15f98235bb81df90aa96e7daf5781a4cdb823da2c39c901) |
| 20 | claim Witch | claimed their share of the pot | [d70bbaa8ef…](https://explorer.tari.mw/tx/d70bbaa8efa7ab2078b6b219105d5a8d85a608bc9545922e24311a58baf2b07a) |
| 21 | claim Vampire | claimed their share of the pot | [73e6a9929c…](https://explorer.tari.mw/tx/73e6a9929c4a864002886988fd4be375bae4e4b83b2be5ff1e572d972055d810) |
| 22 | claim Mummy | claimed their share of the pot | [73bad3d79b…](https://explorer.tari.mw/tx/73bad3d79b6133d501b1c0a531ca8940dc8eb9b9f35c1e01b095b57282b915cf) |
| 23 | claim Ghost | claimed their share of the pot | [08656a84a5…](https://explorer.tari.mw/tx/08656a84a595b8bbb12408a0031d11dd2415a300f7778243aa45f4e74d53508a) |
<!-- /LINKS -->

## License
MIT. See [LICENSE](LICENSE).
