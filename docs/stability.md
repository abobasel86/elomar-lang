# Stability: what 1.0 promises

Elomar-Lang 1.0 makes one promise: **a program that works with 1.0 keeps
working with every later 1.x version**, with the same results. You can
upgrade `elomar` without rewriting anything.

The specification ([`grammar.md`](grammar.md)) is frozen at `1.0.0-rc.1`
from version 0.9: until 1.0, it changes only to fix mistakes.

## What stays the same in every 1.x

- **The language:** its syntax, keywords, types, type rules, effects and
  what every expression and statement does.
- **The standard library:** every module and function documented in
  `grammar.md` (`std.fs`, `std.json`, `std.csv`, `std.time`, `std.ai`, …),
  with the same parameters and results.
- **Error codes:** `E…`, `W…` and `R…` keep their meaning, so programs and
  tools that look for a code keep working.
- **The command line:** the commands (`run`, `check`, `build`, `test`,
  `fmt`, `new`, `repl`, `add`, …) and their options.
- **Project files:** the keys of `elomar.toml` and the format of
  `elomar.lock`.
- **Editors:** `elomar lsp` and `elomar dap` keep speaking the standard
  protocols.

## What may change in a 1.x version

- **New things are added** in ways that do not break existing programs: new
  functions, modules, methods, commands and options. A new keyword is added
  only if no program could already use it as a name.
- **New warnings** (`W…`) may appear. Warnings never stop a program.
- **Messages** may be reworded or made clearer, in English and Arabic.
  Tools should rely on codes, not on the wording.
- **Speed and memory use** may change (they are meant to improve).
- **Bug fixes:** if the interpreter does something different from
  `grammar.md`, it is fixed to match it, and the change is listed in the
  changelog.
- **Security fixes**, even if a program relied on the hole.

## Removing something

Nothing documented is removed in 1.x. Something that should go is first
marked deprecated with a warning for at least one minor version, and is
only removed in 2.0.

## Not covered

- The Rust crates inside the repository (`elomar-parser`, …): they are not
  a public library.
- The browser playground and the exercises on the website.
- Section 9 of `grammar.md` ("not implemented yet"): planned features may
  arrive in a form different from what is described.

---

## بالعربية: ما تعد به النسخة 1.0

تعد Elomar-Lang 1.0 بوعد واحد: **البرنامج الذي يعمل مع 1.0 يبقى يعمل مع كل إصدار 1.x
بعده**، وبالنتائج نفسها. يمكنك تحديث `elomar` دون إعادة كتابة أي شيء.

والمواصفة ([`grammar.md`](grammar.md)) مجمّدة عند `1.0.0-rc.1` منذ الإصدار 0.9: لا تتغير
قبل 1.0 إلا لتصحيح خطأ.

### ما يبقى ثابتًا في كل إصدارات 1.x

- **اللغة:** قواعد الكتابة، والكلمات المحجوزة، والأنواع وقواعدها، والتأثيرات، وما تفعله كل
  عبارة وجملة.
- **المكتبة القياسية:** كل وحدة ودالة موثقة في `grammar.md`، بالمعاملات والنتائج نفسها.
- **رموز الأخطاء:** `E…` و`W…` و`R…` تحتفظ بمعانيها.
- **سطر الأوامر:** الأوامر (`run` و`check` و`build` و`test` و`fmt` و`new` و`repl` و`add`…)
  وخياراتها.
- **ملفات المشروع:** مفاتيح `elomar.toml` وصيغة `elomar.lock`.
- **المحررات:** `elomar lsp` و`elomar dap` يبقيان على البروتوكولين القياسيين.

### ما قد يتغير في إصدار 1.x

- **إضافات جديدة** لا تكسر البرامج الموجودة: دوال ووحدات وأوامر وخيارات جديدة. ولا تُضاف
  كلمة محجوزة جديدة إلا إن لم يكن ممكنًا أن يستخدمها برنامج اسمًا.
- **تحذيرات جديدة** (`W…`)؛ والتحذير لا يوقف البرنامج أبدًا.
- **صياغة الرسائل** قد تتحسن بالعربية والإنجليزية؛ وعلى الأدوات أن تعتمد على الرموز لا النص.
- **السرعة واستهلاك الذاكرة** (والمقصود تحسينهما).
- **إصلاح الأخطاء:** إن خالف المفسّر ما في `grammar.md` يُصلَح ليوافقه، ويُذكر ذلك في سجل التغييرات.
- **إصلاحات الأمان**، حتى لو اعتمد برنامج على الثغرة.

### الحذف

لا يُحذف شيء موثق في 1.x. ما يجب أن يزول يُعلَّم أولًا بتحذير لإصدار فرعي واحد على الأقل،
ولا يُحذف إلا في 2.0.

### خارج هذا الوعد

مكتبات Rust الداخلية في المستودع، وساحة التجربة والتمارين في الموقع، والقسم 9 من
`grammar.md` ("غير المنفذ بعد").
