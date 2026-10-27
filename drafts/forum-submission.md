<!-- DRAFT: not posted. Reply in https://community.tari.com/t/396 once the repo is public. -->

# 🎃 The Cursed Pumpkin: a stealth hot potato on Ootle

**Code:** [GITHUB_REPO_LINK] (MIT)
**Demo (no wallet needed):** [DEMO_LINK]
**Template on Esmeralda:** `template_7660acdce0b99082374f3488654af0ea4cd7e01a2e912b84d53ccce0ead9e1d9`

There is exactly **one** pumpkin, and it is cursed. Players pass it to each other in secret. The chain records every pass, but never who gave it or who got it. At midnight (an epoch), whoever is holding it has to prove it on-chain, and everyone who guessed right splits the pot.

**What makes it private**
- The pumpkin is a supply-1 **stealth resource**, so the holder is just a Pedersen commitment plus a one-time key. Players receive it on secret "ghost" keys that never appear on-chain.
- Each **pass is sealed by a fresh throwaway key**, and the **fee is paid from the game's shared gas tank** inside the template. So the pass links to no account and no player.
- **Guesses are sealed** as salted hashes and opened only after midnight.
- **The midnight reveal is the proof of ownership.** Only the owner of the live UTXO can spend it into a revealed output to their own player key. The template checks every rule on-chain.

**What still leaks:** pass count and timing, the host's first hop, off-chain metadata, and the final holder (by design). The README has the full threat model.

**It really ran on Esmeralda:** a four-player game with a summon, 3 anonymous passes, 4 sealed guesses, the reveal, openings and claims. Every tx is linked in the README, and the demo replays it side by side: *what the public sees* vs *what the players know*. With Sapient installed, the "Am I haunted?" tab scans your wallet for the pumpkin and signs an ownership challenge with its one-time key.

**Built with:** Rust template (tari_template_lib 0.35), 12 engine tests, a Python client on ootle-py, and a static web app with Sapient integration.

**Sapient bonus:** the web app uses Sapient's `scanForResourceUtxos` and `signOwnershipChallenge` to answer "am I haunted?" privately.

**Tari payment address:** [TARI_PAYMENT_ADDRESS]
**Social post:** [SOCIAL_POST_LINK]
