// The keyword switch on lesson pages: shows every code example with English
// keywords and built-in names (fn, let, print, .length) or Arabic ones (دالة,
// ليكن, اطبع, .طول). The conversion is
// done by Elomar-Lang itself (elomar_fmt::convert, in the same
// WebAssembly module and Web Worker as the playground), so strings and
// comments stay as they are. The choice is remembered on this device.
(function () {
  const bar = document.getElementById("kw-switch");
  if (!bar) return;
  const base = bar.dataset.base;
  const main = document.querySelector("main");
  const KEY = "elomar-keywords";
  const buttons = [...bar.querySelectorAll("button[data-to]")];
  let worker = null;
  let waiting = null;

  // Elomar code blocks, and inline code in the text (not in tables, which
  // list keywords in both spellings, nor in the exercises).
  function targets() {
    return [...main.querySelectorAll("code")].filter((c) => {
      if (c.closest("table, .exercises, .playground, #kw-switch")) return false;
      const pre = c.closest("pre");
      if (!pre) return true;
      const cls = c.className + " " + pre.className + " " + (pre.parentElement ? pre.parentElement.className : "");
      return /language-elomar/.test(cls);
    });
  }

  function mark(to) {
    for (const b of buttons) b.setAttribute("aria-pressed", String(b.dataset.to === to));
    document.documentElement.dataset.keywords = to;
  }

  function convert(to) {
    const els = targets();
    if (to === "en") {
      for (const el of els) if (el.dataset.orig !== undefined) el.textContent = el.dataset.orig;
      mark(to);
      return;
    }
    for (const el of els) if (el.dataset.orig === undefined) el.dataset.orig = el.textContent;
    if (!worker) {
      worker = new Worker(base + "assets/playground-worker.js");
      worker.onmessage = (event) => {
        const answer = event.data.answer;
        if (!answer || !answer.sources || !waiting) return;
        waiting.els.forEach((el, i) => (el.textContent = answer.sources[i]));
        mark(waiting.to);
        waiting = null;
      };
    }
    waiting = { els, to };
    worker.postMessage({
      wasmUrl: base + "playground/elomar.wasm",
      request: { keywords: to, sources: els.map((el) => el.dataset.orig) },
    });
  }

  for (const b of buttons) {
    b.addEventListener("click", () => {
      try {
        localStorage.setItem(KEY, b.dataset.to);
      } catch (_) {}
      convert(b.dataset.to);
    });
  }

  let saved = "en";
  try {
    saved = localStorage.getItem(KEY) || "en";
  } catch (_) {}
  mark("en");
  if (saved === "ar") convert("ar");
})();
