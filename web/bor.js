/* Decode the base64 CBOR substate values the Esmeralda indexer has served since Ootle 0.48.
   The blob is @tari-project/ootle-wasm-bor 0.50.1 (BSD-3-Clause); see bor/VERSION.
   It turns those bytes back into the JSON shape this page already reads. */
"use strict";

let borReady = null;

function loadBor() {
  if (borReady) return borReady;
  borReady = (async () => {
    const bytes = await (await fetch("bor/ootle_wasm_bor_bg.wasm")).arrayBuffer();
    const memory = { current: null };
    const readStr = (ptr, len) => new TextDecoder().decode(new Uint8Array(memory.current.buffer, ptr, len));
    const imports = {
      "./ootle_wasm_bor_bg.js": {
        __wbg___wbindgen_throw_5d9e815e6fdf150f(ptr, len) {
          throw new Error(readStr(ptr, len));
        },
        __wbg_Error_67e7344beaa85059(ptr, len) {
          return new Error(readStr(ptr, len));
        },
        __wbindgen_init_externref_table() {},
      },
    };
    const { instance } = await WebAssembly.instantiate(bytes, imports);
    memory.current = instance.exports.memory;
    instance.exports.__wbindgen_start();
    return instance.exports;
  })();
  return borReady;
}

async function decodeSubstateB64(b64) {
  const wasm = await loadBor();
  const encoded = new TextEncoder().encode(b64);
  const ptr = wasm.__wbindgen_malloc(encoded.length, 1);
  new Uint8Array(wasm.memory.buffer).set(encoded, ptr);
  const [outPtr, outLen, err, isErr] = wasm.decodeSubstateValue(ptr, encoded.length);
  if (isErr) {
    const msg = err && err.message ? err.message : "substate decode failed";
    throw new Error(msg);
  }
  const json = new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, outPtr, outLen));
  wasm.__wbindgen_free(outPtr, outLen, 1);
  return JSON.parse(json);
}
