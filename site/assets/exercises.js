// Interactive exercises at the end of a lesson. Each answer is checked by
// Elomar-Lang itself, compiled to WebAssembly (crates/elomar-playground,
// `check_exercise`), in the same Web Worker as the playground, so a long or
// endless program never freezes the page. Answers and progress are kept in
// this browser's localStorage only.
(function () {
  const root = document.getElementById("exercises");
  const dataNode = document.getElementById("ex-data");
  if (!root || !dataNode) return;
  const lang = root.dataset.lang === "ar" ? "ar" : "en";
  const slug = root.dataset.slug;
  const base = root.dataset.base;
  const ALL = JSON.parse(dataNode.textContent);
  const exercises = ALL[slug] || [];
  const TIME_LIMIT_MS = 10000;

  const T = {
    en: {
      check: "Check",
      hint: "Hint",
      solution: "Show solution",
      reset: "Start over",
      loading: "Loading Elomar-Lang…",
      checking: "Checking…",
      correct: "Correct, well done!",
      wrong: "Not yet: {n} of {m} checks passed.",
      expected: "expected",
      got: "got",
      nothing: "(nothing)",
      input: "input",
      errors: "The program has errors:",
      noteMain:
        "Remove `fn main`: in this exercise the checker adds its own `main`, which calls your function.",
      noteSignature:
        "Your code is fine on its own, but the checker cannot call your function. Keep its name and parameters as in the starting code.",
      timeout: "Stopped: the program ran longer than 10 seconds. Is there a loop that never ends?",
      crashed: "The program stopped unexpectedly (for example, too deep a recursion).",
      confirmSolution: "Replace your code with the solution?",
      confirmReset: "Replace your code with the starting code?",
      solved: "{n} of {m} solved",
      done: "✓ solved",
      hintN: "Hint {n}:",
    },
    ar: {
      check: "تحقّق",
      hint: "تلميح",
      solution: "اعرض الحل",
      reset: "ابدأ من جديد",
      loading: "جارٍ تحميل Elomar-Lang…",
      checking: "جارٍ التحقق…",
      correct: "إجابة صحيحة، أحسنت!",
      wrong: "ليس بعد: نجح {n} من {m} اختبارات.",
      expected: "المتوقع",
      got: "الناتج",
      nothing: "(لا شيء)",
      input: "المدخلات",
      errors: "في البرنامج أخطاء:",
      noteMain:
        "احذف `fn main`: في هذا التمرين يضيف المصحّح دالة `main` خاصة به تستدعي دالتك.",
      noteSignature:
        "كودك سليم وحده، لكن المصحّح لا يستطيع استدعاء دالتك. أبقِ اسمها ومعاملاتها كما في الكود الأولي.",
      timeout: "توقف: استغرق البرنامج أكثر من 10 ثوانٍ. هل فيه حلقة لا تنتهي؟",
      crashed: "توقف البرنامج بشكل غير متوقع (مثلًا: استدعاء ذاتي عميق جدًا).",
      confirmSolution: "أستبدل كودك بالحل؟",
      confirmReset: "أستبدل كودك بالكود الأولي؟",
      solved: "حللت {n} من {m}",
      done: "✓ محلول",
      hintN: "تلميح {n}:",
    },
  }[lang];

  const fill = (text, values) => text.replace(/\{(\w+)\}/g, (_, k) => values[k]);

  // ---- Saved work (this browser only; may be unavailable) ----
  const KEY = "elomar.exercise.";
  function load(id) {
    try {
      return JSON.parse(localStorage.getItem(KEY + id)) || {};
    } catch (_) {
      return {};
    }
  }
  function save(id, patch) {
    try {
      localStorage.setItem(KEY + id, JSON.stringify(Object.assign(load(id), patch)));
    } catch (_) {}
  }
  const isSolved = (id) => load(id).solved === true;

  // ---- Lesson list: mark lessons whose exercises are all solved ----
  function markLessons() {
    document.querySelectorAll(".toc li").forEach((li) => {
      const a = li.querySelector("a");
      const m = a && a.getAttribute("href").match(/([\w-]+)\.html$/);
      const list = m && ALL[m[1]];
      li.classList.toggle("solved", !!(list && list.length && list.every((e) => isSolved(e.id))));
    });
    const n = exercises.filter((e) => isSolved(e.id)).length;
    document.getElementById("ex-progress").textContent =
      "· " + fill(T.solved, { n, m: exercises.length });
  }

  // ---- The checker: one worker, one request at a time ----
  let worker = null;
  let ready = false;
  let current = null; // { resolve, timer }
  const queue = [];

  function startWorker() {
    worker = new Worker(base + "assets/playground-worker.js");
    ready = false;
    worker.onmessage = (event) => {
      const msg = event.data;
      if (msg.ready) {
        ready = true;
        return;
      }
      ready = true;
      finish(msg.crash ? { crash: msg.crash } : msg.answer);
    };
    worker.postMessage({ wasmUrl: base + "playground/elomar.wasm" });
  }

  function finish(answer) {
    if (!current) return;
    clearTimeout(current.timer);
    const done = current.resolve;
    current = null;
    done(answer);
    next();
  }

  function next() {
    if (current || !queue.length) return;
    const job = queue.shift();
    current = {
      resolve: job.resolve,
      timer: setTimeout(() => {
        worker.terminate();
        startWorker();
        finish({ timeout: true });
      }, TIME_LIMIT_MS + (ready ? 0 : 20000)),
    };
    worker.postMessage({ wasmUrl: base + "playground/elomar.wasm", request: job.request });
  }

  function check(ex, source) {
    if (!worker) startWorker();
    return new Promise((resolve) => {
      queue.push({
        resolve,
        request: { source, lang, check: { mode: ex.mode, cases: ex.cases } },
      });
      next();
    });
  }

  // ---- Rendering ----
  function el(tag, cls, text) {
    const node = document.createElement(tag);
    if (cls) node.className = cls;
    if (text !== undefined) node.textContent = text;
    return node;
  }

  // Text with `code` in backticks.
  function rich(text) {
    const p = el("p");
    text.split("`").forEach((part, i) => {
      if (i % 2) {
        const c = el("code", null, part);
        c.dir = "ltr";
        p.appendChild(c);
      } else {
        p.appendChild(document.createTextNode(part));
      }
    });
    return p;
  }

  function codeBlock(text, cls) {
    const pre = el("pre", cls, text);
    pre.dir = "ltr";
    return pre;
  }

  function showResult(box, ex, answer) {
    box.textContent = "";
    if (answer.timeout || answer.crash) {
      box.appendChild(el("p", "ex-bad", answer.timeout ? T.timeout : T.crashed));
      return false;
    }
    if (answer.note === "main") {
      box.appendChild(rich(T.noteMain)).className = "ex-bad";
      return false;
    }
    if (answer.errors) {
      if (answer.note === "signature") box.appendChild(rich(T.noteSignature)).className = "ex-bad";
      else box.appendChild(el("p", "ex-bad", T.errors));
      box.appendChild(codeBlock(answer.errors, "ex-errors"));
      return false;
    }
    const passed = answer.cases.filter((c) => c.pass).length;
    box.appendChild(
      el(
        "p",
        answer.ok ? "ex-good" : "ex-bad",
        answer.ok ? T.correct : fill(T.wrong, { n: passed, m: answer.cases.length }),
      ),
    );
    const list = el("ul", "ex-cases");
    answer.cases.forEach((c, i) => {
      const spec = ex.cases[i];
      const li = el("li", c.pass ? "pass" : "fail");
      li.appendChild(el("span", "ex-mark", c.pass ? "✓" : "✗"));
      const body = el("div");
      if (ex.mode === "function") {
        const call = el("code", null, spec.expr);
        call.dir = "ltr";
        body.appendChild(call);
      } else if (spec.inputs.length) {
        body.appendChild(document.createTextNode(T.input + ": "));
        const inp = el("code", null, spec.inputs.join(" / "));
        inp.dir = "ltr";
        body.appendChild(inp);
      }
      if (!c.pass) {
        const table = el("div", "ex-compare");
        table.appendChild(el("span", "ex-label", T.expected));
        table.appendChild(codeBlock(spec.expect || T.nothing));
        table.appendChild(el("span", "ex-label", T.got));
        table.appendChild(codeBlock(c.got || T.nothing));
        body.appendChild(table);
        if (c.error) body.appendChild(codeBlock(c.error, "ex-errors"));
      }
      li.appendChild(body);
      list.appendChild(li);
    });
    box.appendChild(list);
    return answer.ok;
  }

  const list = document.getElementById("ex-list");
  exercises.forEach((ex, index) => {
    const saved = load(ex.id);
    const card = el("article", "ex");
    card.id = ex.id;

    const head = el("h3", null, index + 1 + ". " + ex.title[lang] + " ");
    const badge = el("span", "ex-done", T.done);
    head.appendChild(badge);
    card.appendChild(head);
    card.appendChild(rich(ex.task[lang]));

    const editor = el("textarea", "ex-code");
    editor.value = saved.code || ex.starter;
    editor.dir = "ltr";
    editor.spellcheck = false;
    editor.setAttribute("autocapitalize", "off");
    editor.setAttribute("autocomplete", "off");
    editor.setAttribute("aria-label", ex.title[lang]);
    editor.rows = Math.max(6, editor.value.split("\n").length + 2);
    card.appendChild(editor);

    const bar = el("div", "ex-bar");
    const checkBtn = el("button", "ex-check", "▶ " + T.check);
    const hintBtn = el("button", null, T.hint);
    const solBtn = el("button", null, T.solution);
    const resetBtn = el("button", null, T.reset);
    const status = el("span", "ex-status");
    status.setAttribute("role", "status");
    [checkBtn, hintBtn, solBtn, resetBtn].forEach((b) => {
      b.type = "button";
      bar.appendChild(b);
    });
    bar.appendChild(status);
    card.appendChild(bar);

    const hints = el("ol", "ex-hints");
    card.appendChild(hints);
    const result = el("div", "ex-result");
    result.setAttribute("aria-live", "polite");
    card.appendChild(result);

    const refresh = () => card.classList.toggle("solved", isSolved(ex.id));
    refresh();

    let shown = 0;
    const showHint = () => {
      const all = ex.hints[lang];
      if (shown >= all.length) return;
      const li = rich(all[shown]);
      const item = el("li");
      while (li.firstChild) item.appendChild(li.firstChild);
      hints.appendChild(item);
      shown += 1;
      hintBtn.disabled = shown >= all.length;
    };
    hintBtn.addEventListener("click", showHint);

    let saveTimer = null;
    editor.addEventListener("input", () => {
      clearTimeout(saveTimer);
      saveTimer = setTimeout(() => save(ex.id, { code: editor.value }), 400);
    });

    const run = async () => {
      checkBtn.disabled = true;
      status.textContent = ready ? T.checking : T.loading;
      save(ex.id, { code: editor.value });
      const answer = await check(ex, editor.value);
      checkBtn.disabled = false;
      status.textContent = "";
      if (showResult(result, ex, answer)) {
        save(ex.id, { solved: true });
        refresh();
        markLessons();
      }
    };
    checkBtn.addEventListener("click", run);

    editor.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        run();
      } else if (e.key === "Tab" && !e.shiftKey) {
        e.preventDefault();
        editor.setRangeText("    ", editor.selectionStart, editor.selectionEnd, "end");
      }
    });

    const replace = (code, question) => {
      if (editor.value !== code && !confirm(question)) return;
      editor.value = code;
      editor.rows = Math.max(6, code.split("\n").length + 2);
      save(ex.id, { code });
      result.textContent = "";
    };
    solBtn.addEventListener("click", () => replace(ex.solution, T.confirmSolution));
    resetBtn.addEventListener("click", () => replace(ex.starter, T.confirmReset));

    list.appendChild(card);
  });

  markLessons();
  // Load the checker only when someone starts working on an exercise.
  const warm = () => {
    if (!worker) startWorker();
  };
  root.addEventListener("focusin", warm, { once: true });
  root.addEventListener("pointerdown", warm, { once: true });
})();
