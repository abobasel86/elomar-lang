// Runs Elomar-Lang programs off the page's main thread, so a long or
// endless program never freezes the page: the page can stop this worker.
// The WebAssembly module is crates/elomar-playground, built by the website
// workflow. It needs two functions from us: the clock and random bytes.

let wasm = null;

async function load(url) {
  const imports = {
    env: {
      elomar_now_ms: () => Date.now(),
      elomar_random_fill: (ptr, len) => {
        crypto.getRandomValues(new Uint8Array(wasm.exports.memory.buffer, ptr, len));
      },
    },
  };
  const response = await fetch(url);
  if (!response.ok) throw new Error(`cannot load ${url}: HTTP ${response.status}`);
  const bytes = await response.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, imports);
  wasm = instance;
}

function run(request) {
  const exports = wasm.exports;
  const input = new TextEncoder().encode(JSON.stringify(request));
  const ptr = exports.elomar_alloc(input.length);
  new Uint8Array(exports.memory.buffer, ptr, input.length).set(input);
  const out = exports.elomar_run(ptr, input.length);
  exports.elomar_free(ptr, input.length);
  const view = new DataView(exports.memory.buffer);
  const len = view.getUint32(out, true);
  const text = new TextDecoder().decode(new Uint8Array(exports.memory.buffer, out + 4, len));
  return JSON.parse(text);
}

self.onmessage = async (event) => {
  const { wasmUrl, request } = event.data;
  try {
    if (!wasm) await load(wasmUrl);
    if (request) {
      self.postMessage({ answer: run(request) });
    } else {
      self.postMessage({ ready: true });
    }
  } catch (err) {
    // A trap (for example a very deep recursion) can leave the module's
    // memory in a bad state; start fresh next time.
    wasm = null;
    self.postMessage({ crash: String(err && err.message ? err.message : err) });
  }
};
