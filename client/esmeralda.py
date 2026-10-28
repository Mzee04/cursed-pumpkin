"""Drive a real Cursed Pumpkin game on the Esmeralda testnet.

    python client/esmeralda.py <command> [args]

Keys live in secrets/esmeralda.json (testnet throwaways, git-ignored). Every accepted transaction is appended to
web/data/recorded-game.json with its explorer link, which is what the web app replays.
"""

from __future__ import annotations

import json
import os
import secrets as rnd
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import pumpkin as p  # noqa: E402
from ootle import ComponentAddress, SubstateId, TemplateBlob  # noqa: E402
from ootle._types.instructions import InstructionArgWorkspace, WorkspaceOffsetId  # noqa: E402
from ootle._types.want_input import WantInput  # noqa: E402
from ootle._types._instruction_variants import PayFeeFromBucket  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
STATE = ROOT / "secrets" / "esmeralda.json"
RECORD = ROOT / "web" / "data" / "recorded-game.json"
WASM = ROOT / "templates" / "cursed_pumpkin" / "target" / "wasm32-unknown-unknown" / "release" / "cursed_pumpkin.wasm"
NAMES = ["Vampire", "Mummy", "Witch", "Ghost"]
FEE = 100_000
BOND = 1_000_000  # 1 tTARI
PASS_FEE = 60_000


def load() -> dict:
    return json.loads(STATE.read_text()) if STATE.exists() else {}


def save(s: dict) -> None:
    STATE.parent.mkdir(exist_ok=True)
    STATE.write_text(json.dumps(s, indent=2))
    os.chmod(STATE, 0o600)


def record(step: str, sent: p.Sent, public: dict, private: dict | None = None) -> None:
    RECORD.parent.mkdir(parents=True, exist_ok=True)
    data = json.loads(RECORD.read_text()) if RECORD.exists() else {"network": "esmeralda", "steps": []}
    data["steps"].append({"step": step, **sent.to_json(), "public": public, "private": private or {}})
    RECORD.write_text(json.dumps(data, indent=2))


def meta(key: str, value) -> None:
    RECORD.parent.mkdir(parents=True, exist_ok=True)
    data = json.loads(RECORD.read_text()) if RECORD.exists() else {"network": "esmeralda", "steps": []}
    data[key] = value
    RECORD.write_text(json.dumps(data, indent=2))


def key(s: dict, name: str, kind: str = "player"):
    return p.key_from_json(s["players"][name][kind])


# ------------------------------------------------------------------------------------------------ commands


def cmd_keys() -> None:
    s = load()
    if s.get("players"):
        print("keys exist")
        return
    s["players"] = {n: {"player": p.key_to_json(p.new_key()), "ghost": p.key_to_json(p.new_key())} for n in NAMES}
    save(s)
    for n in NAMES:
        print(n, p.account_of(key(s, n)))


def cmd_tap(name: str, times: str = "1") -> None:
    s = load()
    k = key(s, name)
    acc = p.account_of(k)
    with p.connect(k) as c:
        for _ in range(int(times)):
            exists = c.fetch_substate(SubstateId(opaque=acc)) is not None
            b = p.builder(c)

            def fee(f):
                f.call_method(p.TAP, "take").put_last_instruction_output_on_workspace("tap")
                fid = f.alloc_workspace("fee")
                f.add_instruction(p.take_from_bucket(0, FEE, fid))
                f.add_instruction(PayFeeFromBucket(bucket=WorkspaceOffsetId(fid)))
                if exists:
                    f.call_method(acc, "deposit", (InstructionArgWorkspace(WorkspaceOffsetId(0)),))
                else:
                    f.create_account(p.pk(k), "tap")
                return f

            b.with_fee_instructions_builder(fee)
            wants = {
                WantInput.SpecificSubstate(id=SubstateId(opaque=p.TAP)),
                WantInput.VaultForResource(component=ComponentAddress(p.TAP), resource=p.TARI),
            }
            if exists:
                wants |= {WantInput.SpecificSubstate(id=SubstateId(opaque=acc)),
                          WantInput.VaultForResource(component=ComponentAddress(acc), resource=p.TARI, required=False)}
            p.send(c, f"tap {name}", b, wants)
        print(name, "balance", c.get_account_balance(ComponentAddress(acc), p.TARI))


def cmd_balances() -> None:
    s = load()
    with p.connect(key(s, NAMES[0])) as c:
        for n in NAMES:
            acc = p.account_of(key(s, n))
            try:
                print(n, acc, c.get_account_balances(ComponentAddress(acc)))
            except Exception as e:  # noqa: BLE001
                print(n, acc, "no account", repr(e)[:120])


def cmd_publish(max_fee: str = "30000000") -> None:
    s = load()
    k = key(s, NAMES[0])
    blob = WASM.read_bytes()
    with p.connect(k) as c:
        u = c.account().pay_fee(int(max_fee)).publish_template(TemplateBlob(blob=blob)).prepare()
        d = c.send_dry_run(u)
        print("dry", d.outcome, "estimated fee", d.estimated_fee)
        if not d.outcome.is_commit:
            return
        pend = c.send_transaction(c.seal_transaction(u))
        out = pend.watch()
        rec = pend.get_receipt()
        print(out.kind, pend.tx_id, [x.substate_id.opaque for x in rec.diff_summary.upped])
        tpl = next(x.substate_id.opaque for x in rec.diff_summary.upped if "template" in x.substate_id.opaque)
        print(out.kind, tpl, f"{p.EXPLORER}/tx/{pend.tx_id}")
        s["template"] = tpl
        save(s)
        meta("template", {"address": tpl, "publish_tx": str(pend.tx_id), "link": f"{p.EXPLORER}/tx/{pend.tx_id}",
                          "fee": rec.fee_receipt.total_fees_charged, "wasm_bytes": len(blob)})


# ------------------------------------------------------------------------------------------------ the game


def sub(x: str):
    return WantInput.SpecificSubstate(id=SubstateId(opaque=x))


def game_state(c) -> dict:
    s = load()
    body = c.get_substate(SubstateId(opaque=s["game"]))
    raw = body.raw if hasattr(body, "raw") else body
    st = json.loads(json.dumps(raw, default=str)) if not isinstance(raw, dict) else raw
    return st


def withdraw_into(b, name_key, amount: int, label: str) -> set:
    acc = p.account_of(name_key)
    b.call_method(acc, "withdraw", (p.a_resource(str(p.TARI)), p.a_amount(amount)))
    b.put_last_instruction_output_on_workspace(label)
    return {sub(acc), WantInput.VaultForResource(component=ComponentAddress(acc), resource=p.TARI), sub(str(p.TARI))}


def fee_from(b, k, fee: int = FEE) -> set:
    acc = p.account_of(k)
    b.pay_fee_from_component(acc, fee)
    return {sub(acc), WantInput.VaultForResource(component=ComponentAddress(acc), resource=p.TARI)}


def ws(b, label: str):
    return InstructionArgWorkspace(b._resolve_workspace_offset(label))  # noqa: SLF001


def cmd_create(midnight_in: str = "1", window: str = "2", name: str = "All Hallows Esmeralda") -> None:
    from ootle._types._instruction_variants import CallFunction
    from ootle import TemplateAddress

    s = load()
    host = key(s, NAMES[0])
    with p.connect(host) as c:
        now = c.get_epoch()
        midnight = now + int(midnight_in)
        b = p.builder(c, 40)
        wants = fee_from(b, host)
        b.add_instruction(CallFunction(template_address=TemplateAddress(s["template"]), function="new", args=(
            p.a_str(name), p.a_resource(str(p.TARI)), p.a_amount(BOND), p.a_u64(midnight), p.a_u64(int(window)),
            p.a_amount(500_000))))
        wants |= {sub(s["template"]), sub(str(p.TARI))}
        sent = p.send(c, "create", b, wants)
        game = next(u for u in sent.upped if u.startswith("component_") and u != p.account_of(host))
        res = next(u for u in sent.upped if u.startswith("resource_"))
        s.update(game=game, pumpkin=res, midnight=midnight, reveal_until=midnight + int(window), created_epoch=now)
        save(s)
        meta("game", {"component": game, "pumpkin": res, "name": name, "created_epoch": now, "midnight_epoch": midnight,
                      "reveal_until_epoch": midnight + int(window), "bond": BOND, "template": s["template"]})
        record("create", sent, {"who": "host (public key visible)", "what": f"game created; one PUMPKIN minted into the cauldron; midnight at epoch {midnight}"},
               {"host": NAMES[0]})
        print("game", game, "pumpkin", res, "epoch now", now, "midnight", midnight)


def cmd_gas(amount: str = "2000000") -> None:
    s = load()
    host = key(s, NAMES[0])
    with p.connect(host) as c:
        b = p.builder(c)
        wants = fee_from(b, host) | withdraw_into(b, host, int(amount), "gas")
        b.call_method(s["game"], "add_gas", (ws(b, "gas"),))
        wants |= p.game_wants(s["game"])
        sent = p.send(c, "add_gas", b, wants)
        record("add_gas", sent, {"who": "host", "what": f"{int(amount)/1e6} tTARI into the gas tank that pays for anonymous passes"})


def cmd_join(name: str) -> None:
    s = load()
    k = key(s, name)
    with p.connect(k) as c:
        b = p.builder(c)
        wants = fee_from(b, k) | withdraw_into(b, k, BOND, "bond")
        b.call_method(s["game"], "join", (ws(b, "bond"),))
        wants |= p.game_wants(s["game"])
        sent = p.send(c, f"join {name}", b, wants)
        record("join", sent, {"who": f"player key {p.pk(k).hex()[:12]}…", "what": "joined and locked a 1 tTARI bond"},
               {"player": name})


def pumpkin_wants(s: dict, commitment: bytes | None = None) -> set:
    w = p.game_wants(s["game"]) | {sub(s["pumpkin"])}
    if commitment is not None:
        w.add(p.utxo_want(s["pumpkin"], commitment))
    return w


def cmd_summon(to: str) -> None:
    s = load()
    host = key(s, NAMES[0])
    ghost = key(s, to, "ghost")
    with p.connect(host) as c:
        st, signers, outs = p.build_statement(c, s["pumpkin"], [], 1, [ghost], None)
        b = p.builder(c)
        wants = fee_from(b, host)
        b.call_method(s["game"], "summon", (p.lit(p.statement_cbor(s["pumpkin"], st)), p.a_amount(0)))
        sent = p.send(c, "summon", b, wants | pumpkin_wants(s))
        s["live"] = outs[0].hex()
        s["holder"] = to
        s.setdefault("chain", []).append(to)
        save(s)
        record("summon", sent, {"who": "host", "what": "the pumpkin left the cauldron into a hidden UTXO",
                                "commitment": outs[0].hex()}, {"to": to})


def cmd_pass(to: str, fee: str = str(PASS_FEE)) -> None:
    s = load()
    holder = s["holder"]
    ghost = key(s, holder, "ghost")
    nxt = key(s, to, "ghost")
    throwaway = p.new_key()  # seals the transaction; never used again, never funded
    live = bytes.fromhex(s["live"])
    # Only the throwaway key goes in the wallet. ootle-py's wallet.seal adds a signature from
    # every registered signer, so registering the ghost key would publish it on every pass.
    # The stealth input is authorised by the UTXO's one-time key (stealth_signers) instead.
    with p.connect(throwaway) as c:
        st, signers, outs = p.build_statement(c, s["pumpkin"], [(ghost, live)], 0, [nxt], None)
        b = p.builder(c)
        arg = p.lit(p.statement_cbor(s["pumpkin"], st))
        # The whole pass lives in the fee block: the game pays the fee from its gas tank,
        # so the sealing key is a never-funded one-time key with no account behind it.
        b.with_fee_instructions_builder(lambda f: f.call_method(s["game"], "pass", (arg, p.a_amount(int(fee)))))
        sent = p.send(c, f"pass {holder}->{to}", b, pumpkin_wants(s, live), stealth_signers=signers)
        s["live"] = outs[0].hex()
        s["holder"] = to
        s["chain"].append(to)
        save(s)
        record("pass", sent, {"who": f"one-time key {p.pk(throwaway).hex()[:12]}… (unlinked to any player)",
                              "what": "the pumpkin moved: old commitment spent, new hidden owner",
                              "spent": live.hex(), "commitment": outs[0].hex()}, {"from": holder, "to": to})


def cmd_guess(name: str, suspect: str) -> None:
    s = load()
    k = key(s, name)
    salt = rnd.token_bytes(32)
    commit = p.guess_commitment(s["game"], p.pk(k), p.pk(key(s, suspect)), salt)
    s["players"][name]["guess"] = {"suspect": suspect, "salt": salt.hex()}
    save(s)
    with p.connect(k) as c:
        b = p.builder(c)
        wants = fee_from(b, k)
        b.call_method(s["game"], "seal_guess", (p.a_bytes(commit),))
        sent = p.send(c, f"seal guess {name}", b, wants | p.game_wants(s["game"]))
        record("seal_guess", sent, {"who": f"player key {p.pk(k).hex()[:12]}…", "what": "sealed a guess (a hash; the suspect stays secret)",
                                    "sealed": commit.hex()}, {"player": name, "suspect": suspect})


def cmd_reveal() -> None:
    s = load()
    holder = s["holder"]
    k = key(s, holder)
    ghost = key(s, holder, "ghost")
    live = bytes.fromhex(s["live"])
    with p.connect(k) as c:  # do not register the ghost key: it would be signed into the tx
        st, signers, outs = p.build_statement(c, s["pumpkin"], [(ghost, live)], 0, [], (1, p.pk(k)))
        b = p.builder(c)
        wants = fee_from(b, k)
        b.call_method(s["game"], "reveal", (p.lit(p.statement_cbor(s["pumpkin"], st)),))
        sent = p.send(c, f"reveal {holder}", b, wants | pumpkin_wants(s, live), stealth_signers=signers)
        record("reveal", sent, {"who": f"player key {p.pk(k).hex()[:12]}…",
                                "what": "midnight: the holder proved ownership by spending the live pumpkin back into the cauldron",
                                "spent": live.hex()}, {"holder": holder})


def cmd_open(name: str) -> None:
    s = load()
    k = key(s, name)
    g = s["players"][name]["guess"]
    with p.connect(k) as c:
        b = p.builder(c)
        wants = fee_from(b, k)
        b.call_method(s["game"], "open_guess", (p.a_bytes(p.pk(key(s, g["suspect"]))), p.a_bytes(bytes.fromhex(g["salt"]))))
        sent = p.send(c, f"open guess {name}", b, wants | p.game_wants(s["game"]))
        record("open_guess", sent, {"who": f"player key {p.pk(k).hex()[:12]}…",
                                    "what": f"opened their guess: suspect {p.pk(key(s, g['suspect'])).hex()[:12]}…"},
               {"player": name, "suspect": g["suspect"]})


def cmd_claim(name: str) -> None:
    s = load()
    k = key(s, name)
    acc = p.account_of(k)
    with p.connect(k) as c:
        before = c.get_account_balance(ComponentAddress(acc), p.TARI)
        b = p.builder(c)
        wants = fee_from(b, k)
        b.call_method(s["game"], "claim", ())
        b.put_last_instruction_output_on_workspace("prize")
        b.call_method(acc, "deposit", (ws(b, "prize"),))
        sent = p.send(c, f"claim {name}", b, wants | p.game_wants(s["game"]))
        after = c.get_account_balance(ComponentAddress(acc), p.TARI)
        record("claim", sent, {"who": f"player key {p.pk(k).hex()[:12]}…", "what": "claimed their share of the pot",
                               "balance_change": int(after) - int(before)}, {"player": name})
        print(name, "balance change", int(after) - int(before))


def cmd_epoch() -> None:
    s = load()
    with p.connect(key(s, NAMES[0])) as c:
        print("epoch", c.get_epoch(), "midnight", s.get("midnight"), "reveal_until", s.get("reveal_until"))


if __name__ == "__main__":
    globals()["cmd_" + sys.argv[1]](*sys.argv[2:])
