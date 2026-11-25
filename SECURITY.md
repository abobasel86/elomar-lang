# Security

Elomar-Lang runs programs with permissions (`[permissions]` in
`elomar.toml`): files, network hosts, Python, programs and servers are
denied unless listed, and AI keys stay in environment variables. A way
around these limits is a security problem.

## Reporting a problem

Please report it privately, not in a public issue:

- on GitHub, use **Report a vulnerability** on the
  [Security tab](https://github.com/abobasel86/elomar-lang/security), or
- open a minimal issue that says only that you have a security report and
  how to reach you, without the details.

Include the version (`elomar --version`), a small program or project that
shows the problem, and what it lets a program do. You will get an answer
within a week. Once a fix is released, the report is credited in the
changelog unless you prefer otherwise.

## Supported versions

Fixes go into the newest 1.x release. Upgrade with `elomar upgrade`.

## What is in scope

- A program doing something its `elomar.toml` does not allow (reading or
  writing files, reaching hosts, starting programs, running Python).
- An API key reaching an error message, a log, a file, or a host other
  than its provider.
- A crash or hang of the checker, the formatter or the language server on
  some input (please also say how you found it; the fuzzer is in
  `crates/elomar-fuzz`).
- The installers or `elomar upgrade` installing something other than the
  release they checked.

`std.py` and `process.run` run with the user's full rights by design, once
`python = true` or a command is listed; that alone is not a vulnerability.

---

## بالعربية

أبلغ عن أي ثغرة أمنية سرًا لا في مسألة عامة: عبر **Report a vulnerability** في
[تبويب Security](https://github.com/abobasel86/elomar-lang/security)، أو افتح مسألة قصيرة تقول فقط إن لديك بلاغًا أمنيًا وكيف
نتواصل معك. أرفق الإصدار (`elomar --version`) وبرنامجًا صغيرًا يُظهر المشكلة. ستصلك إجابة خلال أسبوع، وتُصلح الثغرات في أحدث
إصدار 1.x.
