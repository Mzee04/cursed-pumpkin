"""Check who signed every transaction of the recorded Esmeralda game, straight from the indexer.

    python client/verify_signers.py

For each tx it prints the seal key and the extra signatures, naming any key that belongs to a player (player or ghost
key, from web/data/recorded-game.json). Rules checked:
  * a pass is sealed by an unknown throwaway key (the one shown in the replay) and carries exactly one extra signature:
    the one-time key of the pumpkin UTXO it spends;
  * the reveal is sealed by the holder's player key and carries exactly that one-time key;
  * no ghost key appears in any transaction.
Exits non-zero if any check fails.
"""

from __future__ import annotations

import json
import pathlib
import sys

import cbor2
import httpx

INDEXER = "https://ootle-indexer-a.tari.com"
REC = json.loads((pathlib.Path(__file__).resolve().parents[1] / "web/data/recorded-game.json").read_text())


def get(path: str):
    r = httpx.get(f"{INDEXER}/{path}", timeout=30)
    return r.json() if r.status_code == 200 else None


NAMES = {}
for n, v in REC["players"].items():
    NAMES[v["player_key"]] = f"{n}.player"
    NAMES[v["ghost_key"]] = f"{n}.GHOST"


def tag(k: str) -> str:
    return NAMES.get(k, "unknown")


def _find_key(node, commitment: bytes) -> str | None:
    """Walk a decoded stealth statement for the output [[commitment, nonce, ...], [0, [one_time_key]], ...]."""
    if isinstance(node, (list, tuple)):
        if (len(node) >= 2 and isinstance(node[0], (list, tuple)) and node[0] and node[0][0] == commitment
                and isinstance(node[1], (list, tuple)) and len(node[1]) == 2 and node[1][0] == 0):
            key = node[1][1][0] if isinstance(node[1][1], (list, tuple)) else node[1][1]
            return key.hex() if isinstance(key, bytes) and len(key) == 32 else None
        for x in node:
            found = _find_key(x, commitment)
            if found:
                return found
    return None


def one_time_key(commitment: str) -> str | None:
    """The one-time key of a pumpkin UTXO (its SpendAuthorization::Key).

    Spent UTXOs are "down" on the indexer, so the key is read from the public stealth output statement inside the
    transaction that created the UTXO (the CBOR literal passed to summon/pass).
    """
    src = next((x for x in REC["steps"] if x["public"].get("commitment") == commitment), None)
    if not src:
        return None
    d = get(f"transactions/{src['tx_id']}")
    if not d:
        return None
    body = d["transaction"]["transaction"]["V1"]["body"]["transaction"]
    found = set()
    for ins in body.get("fee_instructions", []) + body.get("instructions", []):
        for arg in (ins.get("CallMethod") or ins.get("CallFunction") or {}).get("args", []):
            lit = arg.get("Literal") if isinstance(arg, dict) else None
            if not lit:
                continue
            try:
                k = _find_key(cbor2.loads(bytes.fromhex(lit)), bytes.fromhex(commitment))
            except Exception:  # noqa: BLE001 - not every literal is a statement
                continue
            if k:
                found.add(k)
    return found.pop() if len(found) == 1 else None


failures = 0
for s in REC["steps"]:
    d = get(f"transactions/{s['tx_id']}")
    if not d:
        print(f"{s['label']:<22} {s['tx_id']} FETCH FAILED")
        failures += 1
        continue
    v1 = d["transaction"]["transaction"]["V1"]
    seal = v1["seal_signature"]["public_key"]
    sigs = [x["public_key"] for x in v1["body"]["signatures"]]
    ghosts = [k for k in [seal, *sigs] if tag(k).endswith("GHOST")]
    ok = not ghosts
    note = ""
    if s["step"] in ("pass", "reveal"):
        otk = one_time_key(s["public"]["spent"])
        if s["step"] == "pass":
            shown = s["public"]["who"].split()[2].rstrip("…")
            ok &= tag(seal) == "unknown" and seal.startswith(shown) and otk is not None and sigs == [otk]
        else:
            ok &= tag(seal) == f"{s['private']['holder']}.player" and otk is not None and sigs == [otk]
        note = f" | spent UTXO one-time key {otk[:16] if otk else '?'}"
    failures += not ok
    signed = ", ".join(f"{k[:16]}… ({tag(k)})" for k in sigs) or "none"
    print(f"{'OK ' if ok else 'BAD'} {s['label']:<20} {s['tx_id'][:16]}… seal {seal[:16]}… ({tag(seal)}) | extra sigs: {signed}"
          f" | ghost keys: {len(ghosts)}{note}")
print("VERIFY", "PASS" if failures == 0 else f"FAIL ({failures})")
sys.exit(1 if failures else 0)
