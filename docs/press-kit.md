# Press kit

Texts and images for telling people about Elomar-Lang. Adjust them to the
place you post in, and keep the project status honest: early development,
not production-ready.

## Links

| What | Link |
|---|---|
| Repository | https://github.com/abobasel86/elomar-lang |
| Tutorial (English and Arabic) | https://abobasel86.github.io/elomar-lang/ |
| Playground (runs in the browser) | https://abobasel86.github.io/elomar-lang/en/playground.html |
| Editor extension (Open VSX) | https://open-vsx.org/extension/abobasel86/elomar-lang |
| Downloads | https://github.com/abobasel86/elomar-lang/releases/latest |

## Images

- Icon: [`editor/vscode/images/icon.png`](../editor/vscode/images/icon.png)
- Social preview, 1280×640: [`docs/images/social-preview.png`](images/social-preview.png).
  Set it once in the repository's **Settings → General → Social preview**,
  so links to the repository show it.

## One line

- English: A simple, typed programming language with AI built in. Write
  code in Arabic or English.
- العربية: لغة برمجة بسيطة، فيها الذكاء الاصطناعي جزء من اللغة، وتكتب بها
  بالعربية أو الإنجليزية.

## Hacker News (Show HN)

Title:

> Show HN: Elomar-Lang – a typed language where AI calls are checked like any other call

Text:

> Elomar-Lang is a small, statically typed language I'm building, with AI
> as part of the language rather than a library. You write
> `ai.infer<Ticket>(model, instructions, input)` and the answer is checked
> against the `Ticket` type before your code sees it; a bad answer is an
> `Err` value you must handle. Model calls, network and file access are
> effects in function signatures, and the permissions (allowed hosts, call
> budget) live in `elomar.toml`. An offline mock model makes tests free.
>
> It also accepts Arabic names and gives errors in Arabic or English, which
> is the reason I started it.
>
> What exists: an interpreter in Rust, OpenAI and Anthropic providers,
> agents with MCP tools and approval callbacks, a standard library (files,
> JSON, HTTP client and server, SQLite, regex, processes), packages with a
> git-based registry, a language server and VS Code extension, a bilingual
> tutorial and a WebAssembly playground.
>
> It's early (v0.3) and not production-ready. I'd love feedback on the
> design of typed AI calls and effects.
>
> https://github.com/abobasel86/elomar-lang

## Reddit (r/ProgrammingLanguages)

Title:

> Elomar-Lang: typed AI inference and visible effects, with Arabic support

Text: the Hacker News text above works here too. Lead with a question for
discussion, for example: "How would you type a call whose result comes
from a model?"

## X / LinkedIn (English)

> I'm building Elomar-Lang: a simple, typed programming language with AI
> built in.
>
> • `ai.infer<Ticket>(...)` returns a checked `Ticket`, or an error you must handle
> • Model, network and file access are visible in types
> • Write code and read errors in Arabic or English
> • Try it in your browser, no install
>
> Open source, early stage, feedback welcome:
> https://github.com/abobasel86/elomar-lang

## منشور بالعربية (لينكدإن، إكس، مجموعات المبرمجين)

> أعمل على **Elomar-Lang**، لغة برمجة مفتوحة المصدر، بسيطة، والذكاء الاصطناعي
> جزء منها لا مكتبة خارجية.
>
> • تكتب أسماء المتغيرات والدوال بالعربية، وتظهر رسائل الأخطاء بالعربية أو الإنجليزية
> • تطلب من النموذج نوعًا محددًا، واللغة تتحقق من إجابته قبل أن يراها برنامجك
> • الوصول إلى الشبكة والملفات والذكاء الاصطناعي ظاهر في تعريف كل دالة، ومضبوط بصلاحيات
> • دروس كاملة بالعربية، وساحة تجربة تعمل في المتصفح دون تثبيت
>
> المشروع في بدايته (الإصدار 0.3)، وأرحب بآرائكم وتجاربكم:
> https://github.com/abobasel86/elomar-lang
>
> الدروس: https://abobasel86.github.io/elomar-lang/
