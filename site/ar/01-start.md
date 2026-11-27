---
layout: default
title: البداية
lang: ar
slug: 01-start
---

# 1. البداية

## التثبيت

على **Windows** الصق هذا السطر في PowerShell:

```powershell
irm https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.ps1 | iex
```

وعلى **Linux** أو **macOS** الصقه في الطرفية:

```bash
curl -fsSL https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.sh | sh
```

ينزّل المثبّت آخر إصدار لنظامك، ويتحقق منه ببصمات الإصدار، ويضع `elomar` في مجلد خاص به (`.elomar/bin` في مجلدك الرئيسي)
ويضيف هذا المجلد إلى `PATH`. افتح نافذة طرفية جديدة بعده. ولاحقًا ينقلك `elomar upgrade` إلى أحدث إصدار.

أو بمدير حزم:

```powershell
# Windows باستخدام Scoop
scoop bucket add elomar https://github.com/abobasel86/elomar-lang
scoop install elomar
```

```bash
# macOS أو Linux باستخدام Homebrew
brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
brew install elomar
```

وبعدها ينقلك `scoop update elomar` أو `brew upgrade elomar` إلى أي إصدار جديد.

أو ثبّته يدويًا:

1. افتح [صفحة آخر إصدار](https://github.com/abobasel86/elomar-lang/releases/latest)
   ونزّل الملف المناسب لنظامك:

   | النظام | الملف |
   |---|---|
   | Windows | `elomar-<version>-x86_64-pc-windows-msvc.zip` |
   | Windows على معالجات ARM | `elomar-<version>-aarch64-pc-windows-msvc.zip` |
   | Linux | `elomar-<version>-x86_64-unknown-linux-gnu.tar.gz` |
   | Linux على معالجات ARM (مثل Raspberry Pi) | `elomar-<version>-aarch64-unknown-linux-gnu.tar.gz` |
   | macOS (معالجات Apple) | `elomar-<version>-aarch64-apple-darwin.tar.gz` |
   | macOS (معالجات Intel) | `elomar-<version>-x86_64-apple-darwin.tar.gz` |

2. فك الضغط وضع `elomar` (أو `elomar.exe` على Windows) في مجلد موجود في `PATH`.
   على macOS قد يحتاج التشغيل الأول إلى الأمر `xattr -d com.apple.quarantine elomar`.
3. تأكد أنه يعمل:

   ```text
   elomar --version
   ```

## برنامجك الأول

أنشئ ملفًا اسمه `hello.elo`:

```elomar
fn main() {
    print("مرحبًا بالعالم!")
    print("Hello, world!")
}
```

ثم شغّله:

```text
elomar run hello.elo
```

كل برنامج يبدأ من الدالة `fn main()`، و`print` تطبع قيمها ثم تنتقل إلى سطر جديد.

### بكلمات عربية

لكل كلمة محجوزة كتابة عربية: `دالة` هي `fn`، و`ليكن` هي `let`، و`إذا` هي `if`، و`وإلا` هي `else`.
ويجوز أن يبدأ البرنامج من `رئيسية` بدل `main`:

```elomar
دالة رئيسية() {
    ليكن الاسم = "لينا"
    إذا الاسم.طول > 3 {
        اطبع("مرحبًا يا {الاسم}")
    } وإلا {
        اطبع("أهلًا")
    }
}
```

و`اطبع` هي `print`، و`.طول` هي `.length`: للدوال والأنواع والدوال التابعة أسماء عربية أيضًا ([كلها هنا](12-reference.html#arabic-names)).
اكتب بالعربية أو بالإنجليزية أو امزج بينهما، فالبرنامج واحد. الدروس تستعمل الكلمات الإنجليزية،
وكل الكلمات العربية في [جدول الكلمات المحجوزة](12-reference.html#keywords). وفوق كل درس زر
**«دالة ليكن إذا»** يعرض كل الأمثلة بالكلمات العربية، وفي [صفحة التجربة](playground.html) زر **fn ⇄ دالة**
يحوّل برنامجك.

## جرّب في السطر التفاعلي

الأمر `elomar repl` يفتح سطرًا تفاعليًا: تكتب سطرًا فترى نتيجته فورًا، دون إنشاء ملف.
وهو أسرع طريقة لتجربة فكرة:

```text
elomar repl
> 2 + 3 * 4
14
> let الاسم = "لينا"
> "أهلًا يا {الاسم}!"
"أهلًا يا لينا!"
> [3, 1, 2].sorted()
[1, 2, 3]
```

المتغيرات والدوال التي تكتبها تبقى للأسطر التالية. `:help` يعرض الأوامر، و`:quit` للخروج.

## الحوار مع المستخدم

`input` تعرض سؤالًا وتعيد ما كتبه المستخدم:

```elomar
fn main() {
    let الاسم = input("ما اسمك؟ ")
    print("تشرفنا يا {الاسم}!")
}
```

`{الاسم}` داخل النص تُستبدل بقيمة المتغير. هذا يسمى **إدراج القيم في النصوص**،
ويمكنك وضع أي تعبير بين القوسين.

## عندما يكون هناك خطأ

تفحص Elomar-Lang برنامجك كله قبل تشغيله. جرّب هذا:

```text
fn main() {
    let العمر = 30
    print(العمر + "سنة")
}
```

```text
elomar --lang ar run hello.elo
خطأ[E2005]: لا يمكن تطبيق العامل `+` على `Int` و`String`
```

لم يُنفَّذ شيء، لأن الخطأ اكتُشف أولًا. الخيار `--lang ar` يعرض الرسائل بالعربية
(أو اضبط متغير البيئة `ELOMAR_LANG=ar` مرة واحدة).

## المحرر

في VS Code أو VSCodium أو Cursor، ابحث عن **Elomar-Lang** في قائمة الإضافات وثبّتها. ستحصل على تلوين الكود، وعرض الأخطاء أثناء الكتابة،
ومعلومات عند مرور المؤشر، وإكمال تلقائي، وزر **Run**.

ولتجد خطأً، انقر يسار رقم السطر لتضع نقطة توقف (دائرة حمراء)، ثم اضغط **F5**. يتوقف البرنامج
عندها، فترى قيمة كل متغير، وتتقدم سطرًا سطرًا (F10)، أو تدخل إلى دالة (F11)، أو تكمل (F5).
وإن فشل البرنامج يتوقف على السطر الذي فشل فيه، فترى السبب.

## الأوامر التي ستستخدمها

| الأمر | وظيفته |
|---|---|
| `elomar run file.elo` | فحص البرنامج ثم تشغيله |
| `elomar repl` | اكتب كودًا وشاهد نتيجته فورًا |
| `elomar check file.elo` | الفحص فقط |
| `elomar build file.elo` | صنع ملف تنفيذي لمشاركته |
| `elomar fmt file.elo` | تنسيق الكود بالأسلوب الموحد |
| `elomar new myapp` | إنشاء مشروع مع اختبارات |
| `elomar new myapp --ai ollama` | إنشاء مشروع يستدعي نموذجًا (`mock` أو `ollama` أو `openai` أو `anthropic`) |
| `elomar doctor` | فحص إعداد المشروع: مفاتيح الذكاء الاصطناعي ونماذجه، والصلاحيات، وPython، وخوادم MCP، والحزم |
| `elomar upgrade` | الانتقال إلى أحدث إصدار |
| `elomar test` | تشغيل اختبارات المشروع |
