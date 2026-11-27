// The playground page: an editor, a Run button and the output.
// Programs run in a Web Worker (playground-worker.js) with a time limit.
(function () {
  const root = document.getElementById("pg-root");
  if (!root) return;
  const lang = root.dataset.lang === "ar" ? "ar" : "en";
  const T = JSON.parse(root.dataset.text);
  const base = root.dataset.base;
  const wasmUrl = base + "playground/elomar.wasm";
  const workerUrl = base + "assets/playground-worker.js";
  const TIME_LIMIT_MS = 10000;

  const editor = document.getElementById("pg-code");
  const inputs = document.getElementById("pg-input");
  const output = document.getElementById("pg-output");
  const status = document.getElementById("pg-status");
  const runButton = document.getElementById("pg-run");
  const shareButton = document.getElementById("pg-share");
  const keywordsButton = document.getElementById("pg-keywords");
  const examples = document.getElementById("pg-examples");

  // The examples and the gallery under the playground: site/_data/gallery.json.
  const EXAMPLES = {};
  for (const ex of JSON.parse(root.dataset.gallery)) EXAMPLES[ex.key] = ex;

  for (const [key, ex] of Object.entries(EXAMPLES)) {
    const opt = document.createElement("option");
    opt.value = key;
    opt.textContent = ex.title[lang];
    examples.appendChild(opt);
  }

  // Code with Arabic keywords reads right to left, as Arabic does.
  const ARABIC_KEYWORD = /(^|[^\p{L}\p{N}_])(دالة|ليكن|متغير|إذا|لكل|نوع|استخدم)(?![\p{L}\p{N}_])/u;
  function setDirection() {
    editor.dir = ARABIC_KEYWORD.test(editor.value) ? "rtl" : "ltr";
  }

  function loadExample(key) {
    const ex = EXAMPLES[key];
    editor.value = ex.code;
    setDirection();
    inputs.value = ex.input || "";
  }

  // Programs can be shared in the address: #code=<base64 of UTF-8>, or
  // #example=<key> for a program of the gallery.
  function fromHash() {
    const ex = location.hash.match(/^#example=([\w-]+)$/);
    if (ex && EXAMPLES[ex[1]]) {
      loadExample(ex[1]);
      return true;
    }
    const m = location.hash.match(/^#code=(.+)$/);
    if (!m) return false;
    try {
      const bytes = Uint8Array.from(atob(decodeURIComponent(m[1])), (c) => c.charCodeAt(0));
      editor.value = new TextDecoder().decode(bytes);
      setDirection();
      return true;
    } catch (_) {
      return false;
    }
  }

  if (!fromHash()) loadExample("hello");

  // A gallery link on this page loads its program and goes up to the editor.
  window.addEventListener("hashchange", () => {
    if (fromHash()) {
      output.textContent = "";
      root.scrollIntoView({ behavior: "smooth" });
      run();
    }
  });

  examples.addEventListener("change", () => {
    if (examples.value) loadExample(examples.value);
    examples.value = "";
  });

  let worker = null;
  let timer = null;
  let ready = false;

  function startWorker() {
    worker = new Worker(workerUrl);
    ready = false;
    worker.onmessage = (event) => {
      const msg = event.data;
      if (msg.ready) {
        ready = true;
        setStatus("");
        return;
      }
      if (msg.answer && msg.answer.sources) {
        keywordsAnswer(msg.answer.sources[0]);
        return;
      }
      clearTimeout(timer);
      runButton.disabled = false;
      if (msg.crash) {
        show("", T.crashed + "\n" + msg.crash, false);
        return;
      }
      ready = true;
      const a = msg.answer;
      show(a.output, a.errors, a.ok);
    };
    worker.postMessage({ wasmUrl });
  }

  function setStatus(text) {
    status.textContent = text;
  }

  function show(out, errors, ok) {
    output.textContent = "";
    if (out) {
      const pre = document.createElement("span");
      pre.textContent = out;
      output.appendChild(pre);
    }
    if (errors) {
      const err = document.createElement("span");
      err.className = "pg-errors";
      err.textContent = (out && !out.endsWith("\n") ? "\n" : "") + errors;
      output.appendChild(err);
    }
    if (!out && !errors) output.textContent = T.noOutput;
    setStatus(ok ? T.done : T.failed);
  }

  function run() {
    runButton.disabled = true;
    output.textContent = "";
    setStatus(ready ? T.running : T.loading);
    const request = {
      source: editor.value,
      lang,
      inputs: inputs.value === "" ? [] : inputs.value.split("\n"),
    };
    timer = setTimeout(() => {
      worker.terminate();
      startWorker();
      runButton.disabled = false;
      show("", T.timeout, false);
    }, TIME_LIMIT_MS + (ready ? 0 : 20000));
    worker.postMessage({ wasmUrl, request });
  }

  runButton.addEventListener("click", run);
  editor.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      run();
    } else if (e.key === "Tab" && !e.shiftKey) {
      e.preventDefault();
      const s = editor.selectionStart;
      editor.setRangeText("    ", s, editor.selectionEnd, "end");
    }
  });

  // fn ⇄ دالة: writes every keyword and built-in name of the program in
  // Arabic (print is اطبع, .length is .طول), or, when it
  // already is, in English (elomar_fmt::convert in the worker).
  let keywordsTo = null;
  function askKeywords(to) {
    keywordsTo = to;
    keywordsButton.disabled = true;
    worker.postMessage({ wasmUrl, request: { keywords: to, sources: [editor.value] } });
  }
  function keywordsAnswer(text) {
    ready = true;
    if (keywordsTo === "ar" && text === editor.value) {
      askKeywords("en");
      return;
    }
    keywordsButton.disabled = false;
    editor.value = text;
    setDirection();
  }
  keywordsButton.addEventListener("click", () => askKeywords("ar"));

  shareButton.addEventListener("click", async () => {
    const bytes = new TextEncoder().encode(editor.value);
    let bin = "";
    bytes.forEach((b) => (bin += String.fromCharCode(b)));
    const url = location.href.split("#")[0] + "#code=" + encodeURIComponent(btoa(bin));
    history.replaceState(null, "", url);
    try {
      await navigator.clipboard.writeText(url);
      setStatus(T.copied);
    } catch (_) {
      setStatus(T.linkReady);
    }
  });

  setStatus(T.loading);
  startWorker();
})();
