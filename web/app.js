/* The Cursed Pumpkin: static front end. No build step, no dependencies, no server of our own. */
"use strict";

const INDEXER = "https://ootle-indexer-a.tari.com";
const EXPLORER = "https://explorer.tari.mw/tx/";
const FACES = { Vampire: "🧛", Mummy: "🧟", Witch: "🧙", Ghost: "👻" };

const $ = (s, el = document) => el.querySelector(s);
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const short = (h, n = 10) => (h && h.length > n * 2 ? `${h.slice(0, n)}…${h.slice(-4)}` : h || "");
const tari = (micro) => `${(Number(micro) / 1e6).toLocaleString(undefined, { maximumFractionDigits: 6 })} tTARI`;
const bare = (id) => String(id).replace(/^[a-z]+_/, "");

/* ───────────────────────────── tabs ───────────────────────────── */
document.querySelectorAll(".tabs button").forEach((b) =>
  b.addEventListener("click", () => {
    document.querySelectorAll(".tabs button").forEach((x) => x.setAttribute("aria-selected", x === b));
    document.querySelectorAll(".panel").forEach((p) => (p.hidden = p.id !== b.dataset.tab));
    if (b.dataset.tab === "live") loadLive();
  }),
);

/* ───────────────────────────── replay ───────────────────────────── */
let REC = null;
let frames = [];
let at = 0;
let timer = null;

function storyFor(step, i, all) {
  const pv = step.private || {};
  switch (step.step) {
    case "create": return ["The host lights the cauldron", `${pv.host} creates the game. One PUMPKIN is minted and sits in the cauldron.`];
    case "add_gas": return ["The gas tank is filled", "The host tops up the tank that will pay for anonymous passes."];
    case "join": return [`${pv.player} joins`, `${pv.player} locks a 1 tTARI bond. Their public player key is now on the guest list.`];
    case "summon": return ["The pumpkin is summoned", `The host secretly hands the pumpkin to ${pv.to}'s ghost key.`];
    case "pass": return [`Pass #${all.slice(0, i + 1).filter((s) => s.step === "pass").length}`, `${pv.from} passes the pumpkin to ${pv.to}.`];
    case "seal_guess": return [`${pv.player} seals a guess`, `${pv.player} secretly suspects ${pv.suspect}.`];
    case "reveal": return ["🕛 Midnight: the holder unmasks", `${pv.holder} proves they hold the pumpkin and returns it to the cauldron.`];
    case "open_guess": return [`${pv.player} opens their guess`, `${pv.player} said ${pv.suspect}.`];
    case "claim": { const bc = step.public.balance_change ?? 0; return [`${pv.player} claims`, bc >= 0 ? `${pv.player} receives ${tari(bc)} (net of fee).` : `${pv.player} gets nothing back (bond forfeited) and pays ${tari(-bc)} in fees.`]; }
    default: return [step.label, ""];
  }
}

function buildFrames(rec) {
  let holder = null; let revealed = null;
  const guesses = {}; const opened = {}; const joined = new Set();
  return rec.steps.map((s, i) => {
    const pv = s.private || {};
    if (s.step === "join") joined.add(pv.player);
    if (s.step === "summon") holder = pv.to;
    if (s.step === "pass") holder = pv.to;
    if (s.step === "seal_guess") guesses[pv.player] = pv.suspect;
    if (s.step === "open_guess") opened[pv.player] = pv.suspect;
    if (s.step === "reveal") revealed = pv.holder;
    return { s, i, holder, revealed, guesses: { ...guesses }, opened: { ...opened }, joined: new Set(joined) };
  });
}

function renderTable(f) {
  const names = Object.keys(REC.players);
  $("#table").innerHTML = names.map((n) => {
    const p = REC.players[n];
    const cls = [f.holder === n && !f.revealed ? "holder" : "", f.revealed === n ? "revealed holder" : ""].join(" ");
    const tag = !f.joined.has(n) ? "not joined" : f.revealed === n ? "revealed 🎃" : f.guesses[n] ? `suspects ${f.guesses[n]}` : "playing";
    return `<div class="seat ${cls}"><div class="face">${FACES[n] || "💀"}</div><div class="name">${esc(n)}</div>
      <div class="key" title="player key">${short(p.player_key, 6)}</div><div class="tag">${esc(tag)}</div></div>`;
  }).join("");
}

function renderPublic(f) {
  const s = f.s; const pub = s.public || {};
  const events = (s.events || []).filter((e) => !e.topic.startsWith("std.component"))
    .map((e) => `<span class="chip ev">${esc(e.topic.replace("CursedPumpkin.CursedPumpkin.", "").replace("std.", ""))}${
      e.payload && Object.keys(e.payload).length ? ` ${esc(Object.entries(e.payload).map(([k, v]) => `${k}=${short(String(v), 8)}`).join(" "))}` : ""}</span>`).join("");
  const rows = [
    ["Tx", `<a href="${esc(s.link)}" target="_blank" rel="noopener">${short(s.tx_id, 12)}</a>`],
    ["Epoch", s.epoch],
    ["Signed by", esc(pub.who || "")],
    ["Fee", tari(s.fee)],
    ["Seen", esc(pub.what || "")],
  ];
  if (pub.spent) rows.push(["Spent", `<code>${short(pub.spent, 12)}</code>`]);
  if (pub.commitment) rows.push(["New UTXO", `<code>${short(pub.commitment, 12)}</code> <span class="hint">(a Pedersen commitment; owner hidden)</span>`]);
  if (pub.sealed) rows.push(["Sealed hash", `<code>${short(pub.sealed, 12)}</code>`]);
  rows.push(["Events", events || "none"]);
  $("#pub").innerHTML = `<dl class="kv">${rows.map(([k, v]) => `<dt>${k}</dt><dd>${v}</dd>`).join("")}</dl>`;
}

function renderPrivate(f) {
  const [, text] = storyFor(f.s, f.i, REC.steps);
  const chain = frames.slice(0, f.i + 1).filter((x) => x.s.step === "summon" || x.s.step === "pass").map((x) => x.s.private.to);
  const rows = [["What happened", `<span class="secret">${esc(text)}</span>`]];
  if (chain.length) rows.push(["Chain of hands", esc(["Host", ...chain].join(" → "))]);
  if (f.holder && !f.revealed) rows.push(["Holder now", `<span class="secret">${esc(f.holder)}</span> (only ${esc(f.holder)} knows this for sure)`]);
  const g = Object.entries(f.guesses);
  if (g.length) rows.push(["Sealed guesses", g.map(([a, b]) => `${esc(a)} → ${esc(b)}${f.opened[a] ? " (opened)" : ""}`).join("<br>")]);
  $("#priv").innerHTML = `<dl class="kv">${rows.map(([k, v]) => `<dt>${k}</dt><dd>${v}</dd>`).join("")}</dl>`;
}

function show(i) {
  at = Math.max(0, Math.min(frames.length - 1, i));
  const f = frames[at];
  $("#scrub").value = at;
  const [title] = storyFor(f.s, f.i, REC.steps);
  $("#step-title").textContent = `Step ${at + 1}/${frames.length} · ${title}`;
  renderTable(f); renderPublic(f); renderPrivate(f);
}

async function loadRecord() {
  try {
    REC = await (await fetch("data/recorded-game.json", { cache: "no-cache" })).json();
  } catch (e) {
    $("#replay").insertAdjacentHTML("beforeend", `<p class="bad">Could not load data/recorded-game.json (${esc(e.message)}). Serve this folder over http, e.g. <code>python3 -m http.server</code>.</p>`);
    return;
  }
  frames = buildFrames(REC);
  $("#scrub").max = frames.length - 1;
  const t = REC.steps[0]?.time_utc;
  $("#rec-date").textContent = t ? new Date(t).toUTCString().replace(" GMT", " UTC") : "Esmeralda";
  if (REC.publish_tx) $("#tpl-link").href = EXPLORER + REC.publish_tx;
  $("#game-id").value = REC.game.component;
  const m = location.hash.match(/step=(\d+)/);
  show(m ? +m[1] - 1 : 0);
  const t2 = location.hash.match(/tab=(\w+)/);
  if (t2) document.querySelector(`.tabs [data-tab="${t2[1]}"]`)?.click();
}
$("#prev").onclick = () => show(at - 1);
$("#next").onclick = () => show(at + 1);
$("#scrub").oninput = (e) => show(+e.target.value);
$("#play").onclick = () => {
  if (timer) { clearInterval(timer); timer = null; $("#play").textContent = "Play"; return; }
  if (at >= frames.length - 1) show(0);
  $("#play").textContent = "Pause";
  timer = setInterval(() => { if (at >= frames.length - 1) $("#play").click(); else show(at + 1); }, 2200);
};
document.addEventListener("keydown", (e) => {
  if ($("#replay").hidden || e.target.tagName === "INPUT") return;
  if (e.key === "ArrowRight") show(at + 1);
  if (e.key === "ArrowLeft") show(at - 1);
});

/* ───────────────────────────── live ───────────────────────────── */
const hex = (v) => (v && v["@cbor"] === "bytes" ? v.hex : v && v["@cbor"] === "tag" ? hex(v.value) : v);
async function substate(id) {
  const r = await fetch(`${INDEXER}/substates/${id}`);
  if (!r.ok) throw new Error(`${r.status} ${await r.text()}`);
  return r.json();
}
async function vaultBalance(vid) {
  try {
    const v = (await substate(`vault_${vid}`)).state.up.substate.Vault;
    const c = v.resource_container || v;
    const inner = Object.values(c)[0];
    return inner && (inner.amount ?? inner.revealed_amount);
  } catch { return null; }
}
function decodeGame(raw) {
  const st = raw.state.up.substate.Component.body.state;
  return {
    name: st[0], host: hex(st[1]), pumpkin: hex(st[2]), live: hex(st[3]), passes: st[4], lastPass: st[5],
    midnight: st[6], revealUntil: st[7], bond: st[8], pot: hex(st[9]), gas: hex(st[10]), maxFee: st[11],
    players: (st[12] || []).map((p) => ({ key: hex(p[0]), sealed: !!p[1], opened: hex(p[2]), claimed: p[3] })),
    summoned: st[14], holder: hex(st[15]),
  };
}
function phaseOf(g, epoch) {
  if (epoch < g.midnight) return g.summoned ? "Haunting 👻 (passes open)" : "Gathering";
  if (epoch < g.revealUntil) return g.holder ? "Midnight 🕛 (holder revealed, guesses opening)" : "Midnight 🕛 (waiting for the holder)";
  return "Settled ⚰️ (claims open)";
}
async function loadLive() {
  const id = $("#game-id").value.trim();
  const out = $("#live-cards");
  out.innerHTML = "<p>Loading…</p>";
  try {
    const [net, raw] = await Promise.all([fetch(`${INDEXER}/network`).then((r) => r.json()), substate(id)]);
    const g = decodeGame(raw);
    window.LIVE = g;
    const [pot, gas] = await Promise.all([vaultBalance(g.pot), vaultBalance(g.gas)]);
    const name = (k) => Object.entries(REC?.players || {}).find(([, p]) => p.player_key === k)?.[0];
    const live = g.live ? `<a href="${INDEXER}/substates/utxo_${g.pumpkin}_${g.live}" target="_blank" rel="noopener"><code>${short(g.live, 10)}</code></a>` : "—";
    out.innerHTML = [
      ["Game", esc(g.name), `epoch ${net.epoch}`],
      ["Phase", phaseOf(g, net.epoch), `midnight at epoch ${g.midnight}, claims from ${g.revealUntil}`],
      ["Passes", g.passes, `last at epoch ${g.lastPass}`],
      ["Live pumpkin", live, "the only public trace: a commitment and a one-time key"],
      ["Holder", g.holder ? `${short(g.holder, 8)} ${name(g.holder) ? `(${esc(name(g.holder))})` : ""}` : "unknown 🕳️", "set only by the midnight reveal"],
      ["Pot", pot == null ? "?" : tari(pot), `bond ${tari(g.bond)} each`],
      ["Gas tank", gas == null ? "?" : tari(gas), `max ${tari(g.maxFee)} per pass`],
      ["Players", g.players.length, g.players.map((p) => `${short(p.key, 5)}${name(p.key) ? ` ${esc(name(p.key))}` : ""}: ${p.opened ? `said ${short(p.opened, 4)}` : p.sealed ? "sealed 🔒" : "no guess"}${p.claimed ? " · claimed" : ""}`).join("<br>")],
    ].map(([l, b, s]) => `<div class="card"><div class="lbl">${l}</div><div class="big">${b}</div><div class="hint">${s}</div></div>`).join("");
  } catch (e) {
    out.innerHTML = `<p class="bad">Could not read the game: ${esc(e.message)}</p>`;
  }
}
$("#reload").onclick = loadLive;

/* ───────────────────────────── wallet (Sapient / Universe) ───────────────────────────── */
let wallet = null;
let found = [];
function provider() {
  return window.tari || (Array.isArray(window.tariProviders) && window.tariProviders.find((p) => p && p.isTariWallet)) || null;
}
function noWallet() {
  $("#w-none").innerHTML = "No Tari wallet found. Install <b>Sapient</b>, switch it to Esmeralda, then reload. You can still watch the replay and the live game without one.";
}
$("#w-connect").onclick = async () => {
  wallet = provider();
  if (!wallet || wallet.isAvailable === false || (wallet.isEmbedded === false && !window.tari)) return noWallet();
  try {
    const accts = await wallet.request({ method: "tari_requestAccounts" });
    let net = ""; try { net = JSON.stringify(await wallet.request({ method: "tari_getNetwork" })); } catch {}
    $("#w-acct").innerHTML = `<span class="ok">Connected</span> <code>${esc(short(String(accts?.[0] ?? ""), 14))}</code> <span class="hint">${esc(net)}</span>`;
    $("#w-scan").disabled = false;
  } catch (e) {
    $("#w-acct").innerHTML = `<span class="bad">${esc(e.message || e)}</span>`;
    if (/unavailable/i.test(e.message || "")) noWallet();
  }
};
$("#w-scan").onclick = async () => {
  const out = $("#w-scan-out");
  out.textContent = "Scanning… (this can take a few seconds)";
  try {
    const g = window.LIVE || decodeGame(await substate($("#game-id").value.trim()));
    window.LIVE = g;
    const resourceAddress = `resource_${g.pumpkin}`;
    const view = await wallet.request({ method: "tari_requestViewAccess" }).catch(() => ({ granted: true }));
    if (view && view.granted === false) { out.innerHTML = `<span class="bad">View access declined.</span>`; return; }
    // Outputs the wallet already knows are not returned by a scan, so ask for them first, then scan for new ones.
    const known = await wallet.request({ method: "tari_getShieldedOutputs", params: { resourceAddress } }).catch(() => []);
    const res = await wallet.request({ method: "tari_scanForResourceUtxos", params: { resourceAddress, maxPages: 10 } });
    const scanned = (res && (res.found || res.utxos || res.outputs || res)) || [];
    const seen = new Set();
    const list = [...(Array.isArray(known) ? known : []), ...(Array.isArray(scanned) ? scanned : [])]
      .filter((u) => { const c = String(u.commitment || u.substateId || u.substate_id || ""); if (seen.has(c)) return false; seen.add(c); return true; });
    found = list.map((u) => ({
      commitment: String(u.commitment || u.substateId || u.substate_id || "").replace(/^utxo_[0-9a-f]+_/, ""),
      substateId: u.substateId || u.substate_id || null,
      raw: u,
    }));
    const live = found.find((u) => u.commitment === g.live || (u.substateId || "").endsWith(g.live || "-"));
    if (live) {
      out.innerHTML = `<p class="secret">🎃 You are haunted. The live pumpkin <code>${short(g.live, 10)}</code> is in your wallet.</p>`;
      $("#w-prove").disabled = false;
      window.HAUNT = live;
    } else if (found.length) {
      out.innerHTML = `<p>You held this pumpkin once (${found.length} old output${found.length > 1 ? "s" : ""}), but it has moved on. 👻</p>`;
    } else {
      out.innerHTML = "<p class=ok>Not haunted. No pumpkin in this wallet.</p>";
    }
  } catch (e) {
    out.innerHTML = `<span class="bad">${esc(e.message || e)}</span>`;
  }
};
$("#w-prove").onclick = async () => {
  const out = $("#w-prove-out");
  const g = window.LIVE;
  const substateId = window.HAUNT?.substateId || `utxo_${g.pumpkin}_${g.live}`;
  const nonce = crypto.getRandomValues(new Uint8Array(16)).reduce((s, b) => s + b.toString(16).padStart(2, "0"), "");
  const challenge = `cursed-pumpkin:${$("#game-id").value.trim()}:${g.live}:${nonce}`;
  out.textContent = "Waiting for the wallet…";
  try {
    const sig = await wallet.request({ method: "tari_signOwnershipChallenge", params: { resourceAddress: `resource_${g.pumpkin}`, substateId, challenge } });
    const onChain = (await substate(substateId)).state.up.substate.Utxo.output.auth.Key;
    const match = sig.publicKey && onChain && sig.publicKey.toLowerCase() === onChain.toLowerCase();
    out.innerHTML = `<dl class="kv"><dt>Challenge</dt><dd><code>${esc(challenge)}</code></dd>
      <dt>Signed by</dt><dd><code>${esc(short(sig.publicKey, 12))}</code></dd>
      <dt>Chain says</dt><dd><code>${esc(short(onChain, 12))}</code></dd>
      <dt>Result</dt><dd class="${match ? "ok" : "bad"}">${match ? "The wallet's key matches the live pumpkin's one-time key ✔" : "Key does not match ✘"}</dd></dl>
      <p class="hint">Share the challenge with the signature (<code>${esc(short(sig.signature, 10))}</code>) and anyone can verify the Schnorr signature against that key (this page only compares the keys). The on-chain midnight reveal is the binding proof.</p>`;
  } catch (e) {
    out.innerHTML = `<span class="bad">${esc(e.message || e)}</span>`;
  }
};

loadRecord();
