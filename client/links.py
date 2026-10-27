"""Write the recorded Esmeralda game's explorer links into README.md (between the LINKS markers)."""
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
rec = json.loads((ROOT / "web/data/recorded-game.json").read_text())
g = rec["game"]
lines = [
    f"- Template: `{g['template']}` ([publish tx](https://explorer.tari.mw/tx/{rec['publish_tx']}))",
    f"- Game component: `{g['component']}`",
    f"- PUMPKIN resource: `{g['pumpkin']}`",
    f"- Midnight epoch {g['midnight_epoch']}, claims from epoch {g['reveal_until_epoch']}",
    "",
    "| # | Step | Public view | Tx |",
    "|---|---|---|---|",
]
for i, s in enumerate(rec["steps"], 1):
    lines.append(f"| {i} | {s['label']} | {s['public']['what']} | [{s['tx_id'][:10]}…]({s['link']}) |")
block = "<!-- LINKS -->\n" + "\n".join(lines) + "\n<!-- /LINKS -->"
readme = (ROOT / "README.md").read_text()
readme = re.sub(r"<!-- LINKS -->.*?(<!-- /LINKS -->|$)", lambda _: block, readme, count=1, flags=re.S) \
    if "<!-- /LINKS -->" in readme else readme.replace("<!-- LINKS -->", block)
(ROOT / "README.md").write_text(readme)
print(len(rec["steps"]), "steps written")
