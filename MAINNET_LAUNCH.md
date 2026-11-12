# Mainnet launch checklist

Operator notes for publishing **The Cursed Pumpkin** on Ootle mainnet for the [First to Launch bonus](https://community.tari.com/t/november-bonus-contest-first-to-launch/521).

The template is published on mainnet, and an official game has been instantiated. The committed app default is now Ootle mainnet. [`web/networks.json`](web/networks.json) has the official mainnet `template`, `component`, `pumpkin`, and `publishTx` values. After this is merged to `main`, the public site follows (Pages deploys `web/` from `main`). Do not push `gh-pages` by hand.

Publishing was signed by the human in their own `tari_ootle_walletd` v0.53.0 wallet. This file records the result. It does not sign, spend, or publish further transactions.

## Published on mainnet

Status of the First to Launch steps:

- [x] Publish the contract on Ootle mainnet
- [x] Prove when it was published (transaction id, accepted time, explorer link)
- [x] Instantiate a mainnet game (`CursedPumpkin::new`) so the app can show a live board
- [x] Fill `networks.mainnet` in [`web/networks.json`](web/networks.json) (`template`, `component`, `pumpkin`, `publishTx`)
- [x] Set `defaultNetwork` to `mainnet` (merge to `main` so Pages can deploy; do not push `gh-pages` by hand)
- [ ] Post a thread update with the game component in the [First to Launch](https://community.tari.com/t/november-bonus-contest-first-to-launch/521) thread (deadline `2026-11-21T11:11:11Z`; an initial post with the template already exists)

| Item | Value |
|---|---|
| Network | Ootle mainnet |
| Template | `template_dc7d2ed8f763ce291a0c27b466ed706005279f85831d2d3c9610b4aaabe56b44` |
| Publish transaction | [`ed9b31ee6e1d88837018c4756d61e98bcb01856564198e0ba4d1e6ad91243600`](https://explorer.tari.mw/tx/ed9b31ee6e1d88837018c4756d61e98bcb01856564198e0ba4d1e6ad91243600) |
| Accepted | 12 November 2026, 05:38:14 UTC (06:38 WAT), epoch 38507, Result Accepted |
| Fee | 4.369422 TARI (Template publish 3,341,204 units of cost) |
| WASM blob | 269.1 KiB after wallet `wasm-opt`; blob hash `91e5d59567c55a3b8040e98962431f4075c68391cfd0e8dcdbf1285caf1ca939` |
| Signer public key | `66ff4b83…36dbf22b` |
| Fee account | `component_9d388ba11e941740d673f1077055d515b7a0f1327c8190be5df652da2012af6a` |
| Wallet | `tari_ootle_walletd` v0.53.0 web UI |
| Official game component | `component_f0a10a09bc05a492d96b221a37ef21adf1ad83ffc8fe0c5b2cb02915988357a2` |
| PUMPKIN resource | `resource_f078a6ec49998c27d0470a9688005e10c03e87ff74984515049cd33f885f9946` |
| Create transaction | [`7ff685e08faad7af4dc3e209c91f2ac710974466fdcc6cc7cecc205b385ab351`](https://explorer.tari.mw/tx/7ff685e08faad7af4dc3e209c91f2ac710974466fdcc6cc7cecc205b385ab351) |
| Create accepted | 12 November 2026, 06:34:59 UTC (07:34 WAT). Indexer: created 06:34:58, finalized 06:35:02 UTC, outcome Commit, fee 0.005838 TARI |
| Game rules | name "The Cursed Pumpkin", bond 1 TARI, midnight epoch 138505, reveal window 8640 epochs, max pass fee 0.5 TARI |
| Game phase (checked 2026-11-12, epoch 38508) | Gathering (no players, pumpkin unsummoned, pot and gas 0 TARI) |

Two later `CursedPumpkin::new` transactions (`4711cf…0670f4` and `3d9ad0…c674b0`) also committed, from accidental extra Submit clicks. They are not the official game. Do not point `networks.json` at them.

The sections below are the original operator checklist, kept for the remaining steps.


## When the network opens

tari.com reads the launch instant from <https://rwa.y.at/config> (`ootle-mainnet-launch-at`). Fetched 2026-11-11T00:32Z, that value was:

`2026-11-11T05:00:00Z` (06:00 WAT, UTC+1)

The homepage falls back to the same timestamp and refetches the config about once a minute. Community posts still say 11:11 UTC, including [Veil’s explorer note](https://community.tari.com/t/veil-explorer-is-ready-for-ootle-mainnet/568). Use the live config as the scheduled time, then wait until a mainnet indexer actually answers. Do not publish against a dead endpoint.

```bash
curl -fsS https://rwa.y.at/config
curl -fsS https://indexer-a.ootle.tari.com/network
curl -fsS https://indexer-b.ootle.tari.com/network
```

Proceed only when a response looks like `{"network":"mainnet", ...}`. At 2026-11-11T00:32Z both official mainnet hosts returned Cloudflare HTTP 523. The Esmeralda indexers answered `esmeralda` at epoch 13005. The exchange-integration guide still marks some mainnet values as placeholders, so if those URLs are wrong on the day, take the replacements from the guide and edit `mainnet.indexers` in `web/networks.json`.

## What is already true

| Item | Value |
|---|---|
| Template crate | `templates/cursed_pumpkin` |
| Library | `tari_template_lib` 0.38 (template definition V2, the Ootle 0.50.1 rebuild) |
| Wallet to use | `tari_ootle_walletd` **v0.53.0** (10 Nov 2026), the build named in the [wallet setup guide](https://ootle.tari.com/guides/setup-a-wallet/) |
| Esmeralda template | `template_cb295c2957234e85ed7de45c45e5c024b5351ff23a3737ffba464b1673d00343` |
| Esmeralda game | `component_f0e06041353d946f32994793a7062d485615e929cbefe7ba1eb382be68814ddf` |
| Esmeralda publish tx | [`27d48b78458d3959c7d56a7f80df6c6e59a1ef4e4e9a66482ae2f98bbb1c94cd`](https://testnet-explorer.tari.mw/tx/27d48b78458d3959c7d56a7f80df6c6e59a1ef4e4e9a66482ae2f98bbb1c94cd) |
| Mainnet template | `template_dc7d2ed8f763ce291a0c27b466ed706005279f85831d2d3c9610b4aaabe56b44` |
| Mainnet publish tx | [`ed9b31ee6e1d88837018c4756d61e98bcb01856564198e0ba4d1e6ad91243600`](https://explorer.tari.mw/tx/ed9b31ee6e1d88837018c4756d61e98bcb01856564198e0ba4d1e6ad91243600), accepted 12 Nov 2026 05:38:14 UTC, epoch 38507 |
| Official mainnet game | `component_f0a10a09bc05a492d96b221a37ef21adf1ad83ffc8fe0c5b2cb02915988357a2` (create tx [`7ff685e08faad7af4dc3e209c91f2ac710974466fdcc6cc7cecc205b385ab351`](https://explorer.tari.mw/tx/7ff685e08faad7af4dc3e209c91f2ac710974466fdcc6cc7cecc205b385ab351), accepted 12 Nov 2026 06:34:59 UTC) |
| App config | [`web/networks.json`](web/networks.json). Default network is `mainnet`. Mainnet `template` / `publishTx` / `component` / `pumpkin` hold the official game addresses. Open `?network=esmeralda` for the recorded testnet game. |
| Public site | <https://mzee04.github.io/cursed-pumpkin/> deploys from `main` only ([`.github/workflows/pages.yml`](.github/workflows/pages.yml)). Once this change is on `main`, the site defaults to the official mainnet game. Do not push `gh-pages` by hand. |

### Why the template was not bumped to 0.53 / `tari_template_lib` 0.40

[Ootle 0.53.0](https://github.com/tari-project/tari-ootle/releases/tag/v0.53.0) (10 Nov 2026) is the latest release. Its notes say deployed templates keep working. The breaking template change is only for a **rebuild** with `tari_template_lib` 0.40: library types are tagged `Type::WellKnown`, and a 0.52 wallet or indexer cannot read that definition. 0.51 and 0.52 tell template authors that 0.38 templates keep loading and that nothing needs a rebuild.

This branch therefore keeps `tari_template_lib` 0.38 and `tari_template_test_tooling` 0.50.1. Publish the WASM you build from that pin. Use walletd 0.53.0 to claim TARI and to publish, because the setup guide ships that binary and 0.52 changed burn claims (the claim returns a revealed TARI bucket the wallet must move). That claim change is in the wallet, not in this template. If a newer release on launch morning says 0.38 templates no longer load, stop and rebuild against the version those notes name. Do not bump speculatively.

`web/bor` is still `@tari-project/ootle-wasm-bor` 0.50.1, which decodes the live Esmeralda component. Replace it only if a mainnet substate fails to decode.

## Before 06:00 WAT

Do these without submitting an Ootle transaction.

1. Read the official pages again. They still said mainnet was unlaunched when this file was written.
   - [Publishing templates](https://ootle.tari.com/guides/publishing-templates/)
   - [CLI](https://ootle.tari.com/guides/cli/)
   - [Wallet setup](https://ootle.tari.com/guides/setup-a-wallet/)
   - [Burn Minotari for TARI](https://ootle.tari.com/guides/burn-minotari/)
   - [How burn claims work](https://ootle.tari.com/guides/claim-burn/)
   - [Exchange integration](https://ootle.tari.com/guides/exchange-integration/) (indexers, address prefix `otl_` on mainnet, `otl_esm_` on Esmeralda)
   - [Getting started](https://ootle.tari.com/guides/getting-started/)
   - [Releases](https://github.com/tari-project/tari-ootle/releases)
2. Install [walletd v0.53.0](https://github.com/tari-project/tari-ootle/releases/tag/v0.53.0) for your machine. Check the checksum on the release. The wallet daemon is an operator tool; the setup guide says end-user wallets are separate.
3. Create a **new** mainnet wallet directory. Do not reuse the Esmeralda data directory.

   ```bash
   ./tari_ootle_walletd --network mainnet --base-path ~/ootle-mainnet
   ```

   `TARI_NETWORK=mainnet` is the same switch. The UI is `http://localhost:5100`. Create the default account. Copy the **public key** (64 hex characters) from account details. The `otl_…` string is the address, and it is not the claim key. A mainnet address starts with `otl_` and not `otl_esm_`.
4. Build the WASM locally. This does not contact the network.

   ```bash
   rustup target add wasm32-unknown-unknown
   cd templates/cursed_pumpkin
   cargo build --release --target wasm32-unknown-unknown
   ```

   Publish `templates/cursed_pumpkin/target/wasm32-unknown-unknown/release/cursed_pumpkin.wasm`. The publish guide caps the binary at 1 MiB. The wallet runs `wasm-opt` itself before it estimates the fee.
5. Get mainnet TARI the official way: burn Minotari (XTM) on the base layer and claim it on Ootle. tari.com tells holders to burn XTM before launch and claim TARI when the chain is live. The only path is [Burn Minotari for TARI](https://ootle.tari.com/guides/burn-minotari/). One Minotari burned becomes one TARI, minus the Ootle claim fee. There is no peg back to XTM.

   The human does this. Do not hand it to an agent, and do not use the guide’s gRPC / script section from an unattended process.

   - Run `minotari_console_wallet` on **mainnet**, funded with XTM.
   - Run `tari_ootle_walletd --network mainnet` on the same machine when you can, so they share the burn-proof directory (`~/.local/share/tari/mainnet/burn_proofs/` on Linux).
   - In the Ootle wallet UI, copy the account public key.
   - In the Minotari wallet, open the Burn tab. Claim public key = that hex key. Leave the sidechain key **empty** (empty means Ootle). Amount is in Minotari (`10T` means 10). Keep the default fee-per-gram unless the base layer is busy.
   - A wrong claim key or a non-empty sidechain key destroys the burn. Nobody can claim it. The human checks both fields, then sends and confirms.
   - Leave the Ootle daemon running. It claims when the burn is claimable (`auto_claim_burns` stays on). If you claim by hand: account page, Claim Burn, pick the proof, Estimate Fee, Claim Burn.

   The two official clocks for “when is it claimable” disagree. [Claim burn](https://ootle.tari.com/guides/claim-burn/) says validators wait about 780 base-layer blocks on mainnet (about a day at a 2-minute block) and that the claim must land in a later epoch. tari.com says a burn made before launch can be claimed when the chain goes live, with no extra wait. Believe the wallet: a rejection whose code is `NotYetValid` means the burn is real and the confirmation window is still open. Wait and let the daemon retry. The Esmeralda republish of this template cost about 6.3 tTARI; mainnet fees are whatever Estimate Fee reports. Have more TARI than that estimate before you publish.

## At launch, after `GET /network` says mainnet

Steps 1–7 below were completed on 12 November 2026 (see [Published on mainnet](#published-on-mainnet)). `defaultNetwork` is `mainnet`. After merge to `main`, Pages deploys the public site. Do not push `gh-pages` by hand.

Every step below is signed by the human. `client/esmeralda.py` and `client/pumpkin.py` are pinned to Esmeralda and will submit a transaction if you run `publish`. Do not point them at mainnet. This repo is not a `tari create` workspace, so the wallet UI is the path that matches the publishing guide.

1. Start the mainnet daemon if it is not already running. If its saved indexer is still the placeholder that failed, pass one that answered:

   ```bash
   ./tari_ootle_walletd --network mainnet --base-path ~/ootle-mainnet \
     -i https://indexer-a.ootle.tari.com
   ```

   Repeat `-i` for `https://indexer-b.ootle.tari.com` if you want failover. Indexer URLs chosen in the web UI are stored in the wallet database and override the flag.
2. Confirm the account balance is mainnet TARI, not an Esmeralda balance, and that the address prefix is `otl_`.
3. Publish the template.
   - Home, **Publish Template**.
   - Fee account: the mainnet account that holds the claimed TARI.
   - Upload `cursed_pumpkin.wasm`.
   - **Estimate Fee**, then **Publish Template**.
   - The [CLI](https://ootle.tari.com/guides/cli/) alternative is `tari publish`, and it only submits through a wallet daemon the human is logged into. It prompts before it spends. Use it only if you have pointed that CLI at this mainnet daemon yourself. Skip it if the CLI is still aimed at an Esmeralda or localnet wallet.
4. Capture the proof as soon as the transaction is accepted. Ranking is the publish transaction’s time, not the forum post.
   - Transaction id.
   - Template address from the wallet’s Templates page. It looks like `template_` plus 64 hex characters. Copy it. Do not reuse the Esmeralda template address.
   - The time the indexer reports for that transaction (`GET /transactions/<tx_id>` on the mainnet indexer, status finalized / accepted).
   - Explorer link, once Veil has switched: `https://explorer.tari.mw/tx/<tx_id>`.
   - A screenshot of the wallet confirmation and the indexer JSON. Keep the fee the wallet charged.
5. Instantiate a game if the public site should show a live board. The contest ranks the **template publish**. The live tab reads a component, and a published template has no component until someone calls the constructor. That call is another transaction the human signs.

   Done. The official game is `component_f0a10a09bc05a492d96b221a37ef21adf1ad83ffc8fe0c5b2cb02915988357a2`, pumpkin `resource_f078a6ec49998c27d0470a9688005e10c03e87ff74984515049cd33f885f9946`, create tx [`7ff685e08faad7af4dc3e209c91f2ac710974466fdcc6cc7cecc205b385ab351`](https://explorer.tari.mw/tx/7ff685e08faad7af4dc3e209c91f2ac710974466fdcc6cc7cecc205b385ab351), accepted 12 November 2026, 06:34:59 UTC. Params: name "The Cursed Pumpkin", bond 1 TARI, midnight epoch 138505, reveal window 8640 epochs, max pass fee 0.5 TARI. Two later constructor transactions from accidental extra Submit clicks are not official; ignore them.

6. Fill these fields in [`web/networks.json`](web/networks.json) under `networks.mainnet`:

   - `template`
   - `component`
   - `pumpkin`
   - `publishTx` (the hex transaction id, no URL)

   Done. Those four strings hold the official addresses. The schema has no create-tx field; that hash is recorded in this file and the README instead. `defaultNetwork` is now `mainnet`. Open the app with `?network=esmeralda`, or use the Network selector, to read the recorded testnet game. The selector is stored in this browser only.

7. Point the public site at mainnet. GitHub Pages deploys the `web/` directory from `main` ([`.github/workflows/pages.yml`](.github/workflows/pages.yml)). This commit sets `defaultNetwork` to `mainnet`. After it is merged (no force-push of `main`, no manual `gh-pages` push), <https://mzee04.github.io/cursed-pumpkin/> serves the official mainnet game.

## Forum post

Post yourself on [First to Launch](https://community.tari.com/t/november-bonus-contest-first-to-launch/521). The deadline in that thread is 11:11:11 UTC on 21 November 2026 (`2026-11-21T11:11:11Z`). A post after the publish does not change your rank. A submission that is not reviewable by the deadline is disqualified. An initial post with the template is already up as [reply #7](https://community.tari.com/t/november-bonus-contest-first-to-launch/521/7). A follow-up should include:

- Project: The Cursed Pumpkin
- Mainnet template address
- Publish transaction id, its accepted time, and `https://explorer.tari.mw/tx/<tx_id>`
- Official game component and create transaction id
- How to view it: <https://mzee04.github.io/cursed-pumpkin/>, which defaults to the official mainnet game once this is on `main`. Do not include an explorer link unless that page has been checked.
- The game component address, if you instantiated one
- Eligibility: this project was entered in Spooky Secrets and the Creator Contest

Do not post wallet seeds, account secret keys, or burn proofs.

## Build check on this branch

`cargo +stable build --release --target wasm32-unknown-unknown --locked` in `templates/cursed_pumpkin` completed with `tari_template_lib` 0.38.0 (rustc 1.99.1). The pre-opt WASM is 320,488 bytes, under the 1 MiB cap. `target/` is gitignored. That binary was later published from the wallet UI; after the wallet's `wasm-opt` the blob was 269.1 KiB.

## Blockers checked while this branch was prepared

These were true when the branch was opened. The template publish later cleared the first three for the ranking event.

- ~~No mainnet template address exists.~~ Published: see [Published on mainnet](#published-on-mainnet). ~~The mainnet game component is still missing.~~ Instantiated: official game `component_f0a10a09…8357a2`, recorded in `web/networks.json`. `defaultNetwork` is `mainnet`.
- Mainnet public indexers were unreachable (HTTP 523) during prep. They answered later; indexer-a was flaky and the wallet failed over to indexer-b.
- Mainnet TARI was obtained and used to pay the publish fee (4.369422 TARI). Stealth-only funds required a revealed self-send before fee estimation would succeed.
- Official docs and the wallet setup page still described mainnet as not launched when this was written. walletd v0.53.0 is the binary that was used to publish; the template pin is intentionally older, for the reason above.
- `client/esmeralda.py publish` signs and broadcasts on Esmeralda. It is the wrong tool for this launch.
