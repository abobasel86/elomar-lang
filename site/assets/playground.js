// The playground page: an editor, a Run button and the output.
// Programs run in a Web Worker (playground-worker.js) with a time limit.
(function () {
  const root = document.getElementById("playground");
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
  const examples = document.getElementById("pg-examples");

  const EXAMPLES = {
    hello: {
      en: "Hello",
      ar: "مرحبا",
      code: `fn main() {
    let name = input("What is your name? ")
    print("Hello, {name}!")
    print("مرحبًا يا {name}!")
}
`,
      input: "Lina",
    },
    numbers: {
      en: "Lists and closures",
      ar: "القوائم والدوال المجهولة",
      code: `fn main() {
    let grades = [91, 72, 55, 88, 64]
    print("sorted:", grades.sorted())
    print("passed:", grades.filter(|g| g >= 60))
    print("average:", (grades.sum() / grades.length).round_to(1))
    for item in grades.enumerate() {
        print("{item.index + 1}. {item.value}")
    }
}
`,
    },
    types: {
      en: "Types and errors",
      ar: "الأنواع والأخطاء",
      code: `enum Shape {
    Circle(Float)
    Rect(Float, Float)
}

fn area(s: Shape) -> Float {
    return match s {
        Shape.Circle(r) => 3.14159 * r * r
        Shape.Rect(w, h) => w * h
    }
}

fn parse_age(text: String) -> Result<Int, String> {
    return match text.parse_int() {
        Some(n) => Ok(n)
        None => Err("'{text}' is not a number")
    }
}

fn main() {
    print(area(Shape.Circle(1.0)), area(Shape.Rect(2.0, 3.0)))
    print(parse_age("42"), parse_age("abc"))
}
`,
    },
    arabic: {
      en: "Arabic names",
      ar: "أسماء عربية",
      code: `fn التقدير(الدرجة: Int) -> String {
    if الدرجة >= 90 {
        return "ممتاز"
    } else if الدرجة >= 70 {
        return "جيد"
    }
    return "يحتاج إلى تحسين"
}

fn main() {
    let الطلاب = ["لينا": 95, "عمر": 72, "سارة": 60]
    for الاسم in الطلاب.keys() {
        print("{الاسم}: {التقدير(الطلاب[الاسم])}")
    }
}
`,
    },
    ai: {
      en: "AI (offline mock)",
      ar: "الذكاء الاصطناعي (نموذج تجريبي)",
      code: `use std.ai

type Summary = { title: String, points: List<String> }

async fn main() {
    // "mock" answers offline with a value of the requested type.
    match await ai.infer<Summary>(
        model: "mock",
        instructions: "Summarize the text",
        input: "Elomar-Lang has AI built into the language."
    ) {
        Ok(s) => print("title:", s.title, "points:", s.points.length)
        Err(e) => print("failed:", e.kind())
    }
}
`,
    },
    toolbox: {
      en: "Toolbox",
      ar: "صندوق الأدوات",
      code: `use std.math
use std.time
use std.regex
use std.crypto

fn main() {
    print(math.pi.round_to(5), math.sqrt(2))
    print(time.format(time.now()))
    print(regex.find_all(r"\\d+", "rooms 12, 7 and 30"))
    print(crypto.sha256("elomar"))
    print(crypto.uuid())
}
`,
    },
  };

  for (const [key, ex] of Object.entries(EXAMPLES)) {
    const opt = document.createElement("option");
    opt.value = key;
    opt.textContent = ex[lang];
    examples.appendChild(opt);
  }

  function loadExample(key) {
    const ex = EXAMPLES[key];
    editor.value = ex.code;
    inputs.value = ex.input || "";
  }

  // Programs can be shared in the address: #code=<base64 of UTF-8>.
  function fromHash() {
    const m = location.hash.match(/^#code=(.+)$/);
    if (!m) return false;
    try {
      const bytes = Uint8Array.from(atob(decodeURIComponent(m[1])), (c) => c.charCodeAt(0));
      editor.value = new TextDecoder().decode(bytes);
      return true;
    } catch (_) {
      return false;
    }
  }

  if (!fromHash()) loadExample("hello");

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
