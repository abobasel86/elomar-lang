---
layout: default
title: البداية
lang: ar
slug: 01-start
---

# 1. البداية

## التثبيت

1. افتح [صفحة آخر إصدار](https://github.com/abobasel86/elomar-lang/releases/latest)
   ونزّل الملف المناسب لنظامك:

   | النظام | الملف |
   |---|---|
   | Windows | `elomar-<version>-x86_64-pc-windows-msvc.zip` |
   | Linux | `elomar-<version>-x86_64-unknown-linux-gnu.tar.gz` |
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

في VSCodium وCursor وغيرهما من المحررات التي تستخدم Open VSX، ابحث عن **Elomar-Lang**
في قائمة الإضافات. وفي Visual Studio Code ثبّت الإضافة من صفحة الإصدار نفسها
(الملف `elomar-lang-<version>.vsix`، ثم **Extensions ← … ← Install from VSIX**). ستحصل على تلوين الكود، وعرض الأخطاء أثناء الكتابة،
ومعلومات عند مرور المؤشر، وإكمال تلقائي، وزر **Run**.

## الأوامر التي ستستخدمها

| الأمر | وظيفته |
|---|---|
| `elomar run file.elo` | فحص البرنامج ثم تشغيله |
| `elomar repl` | اكتب كودًا وشاهد نتيجته فورًا |
| `elomar check file.elo` | الفحص فقط |
| `elomar fmt file.elo` | تنسيق الكود بالأسلوب الموحد |
| `elomar new myapp` | إنشاء مشروع مع اختبارات |
| `elomar test` | تشغيل اختبارات المشروع |
