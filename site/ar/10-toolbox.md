---
layout: default
title: صندوق الأدوات والخطوات التالية
lang: ar
slug: 10-toolbox
---

# 10. صندوق الأدوات والخطوات التالية

في المكتبة القياسية وحدة لمعظم الأعمال اليومية. استوردها بـ `use std.<الاسم>`.

## الرياضيات والأعداد العشوائية والوقت

```elomar
use std.math
use std.random
use std.time

fn main() {
    print(math.pi.round_to(4), math.sqrt(2), math.pow(2, 8))
    print(random.int(1, 6), random.choice(["أحمر", "أخضر"]))

    let البداية = time.make(2026, 1, 1)
    print(time.format(البداية))             // 2026-01-01T00:00:00Z
    print(time.date(البداية).weekday)       // 4 (الخميس؛ الاثنين = 1)
}
```

الأوقات أعداد `Int` بالملّي ثانية منذ 1970 (بتوقيت UTC). و`time.parse` تقرأ نصًا
بصيغة ISO 8601 وتعيد `Option`:

```elomar
use std.time

fn main() {
    match time.parse("2026-03-21T10:30:00+03:00") {
        Some(t) => print(time.format(t))   // 2026-03-21T07:30:00Z
        None => print("ليس تاريخًا")
    }
    print(time.today())
}
```

ويمكن كتابة التاريخ بالشكل الذي تريده، وتحريكه بالأيام أو الأشهر:

```elomar
use std.time

fn main() {
    let t = time.make(2026, 1, 31, 9, 30)
    print(time.format_as(t, "DD/MM/YYYY HH:mm"))           // 31/01/2026 09:30
    print(time.format_as(time.add_months(t, 1), "DD/MM"))  // 28/02: آخر يوم في الشهر
    print(time.days_between(t, time.add_days(t, 10)))      // 10
}
```

## حدود الوقت

`timeout` توقف العمل الذي يطول أكثر من اللازم. `await timeout(work, 2s)` تعطي
`Some(الناتج)`، أو `None` إن انتهى الوقت:

```elomar
async fn count_forever() -> Int {
    var n = 0
    while true {
        n += 1
    }
    return n
}

async fn main() {
    match await timeout(count_forever(), 200ms) {
        Some(n) => print("انتهى: {n}")
        None => print("توقف بعد 200 ملي ثانية")
    }
}
```

## الأنماط النصية والترميز والتشفير

```elomar
use std.regex
use std.encoding
use std.crypto

fn main() {
    print(regex.find_all("[0-9]+", "الغرف 12 و7"))
    print(regex.replace(r"(\w+)@", "راسل lina@example.com", "***@"))
    print(encoding.base64_encode("مرحبا"))
    print(encoding.url_encode("a b&c"))
    print(crypto.sha256("hello"))
    print(crypto.uuid())
}
```

اكتب الأنماط **نصوصًا خامًا** `r"..."`: داخلها تبقى الشرطة المائلة و`{` كما هي،
فـ `r"\d{2,4}"` هو النمط تمامًا. (في النص العادي يبدأ `{` إدراجًا، وتحتاج الشرطة
المائلة إلى المضاعفة.)

## حفظ كلمات المرور {#passwords}

لا تحفظ كلمة المرور نفسها ولا `sha256` لها: من يقرأ قاعدة البيانات يستطيع أن يجرّب عليها مليارات التخمينات في
الثانية. احفظ ما ترجعه `crypto.hash_password` (منذ 1.19)، وتحقق عند الدخول بـ `crypto.verify_password`:

```elomar
use std.crypto

fn main() {
    let stored = crypto.hash_password("كلمة سر طويلة")
    print(stored.starts_with("$argon2id$"))
    print(crypto.verify_password("كلمة سر طويلة", stored))
    print(crypto.verify_password("تخمين خاطئ", stored))
}
```

تستخدم خوارزمية Argon2id، وهي مصممة لتكون بطيئة ومستهلكة للذاكرة على من يخمّن، مع ملح عشوائي جديد في كل مرة،
فتعطي كلمة المرور نفسها نتيجة مختلفة في كل تجزئة. احفظ النص كاملًا: فيه الملح والإعدادات، وتقرؤه مكتبات Argon2
في اللغات الأخرى.

## سطر الأوامر

```elomar
use std.os

fn main() {
    let args = os.args()
    if args.length == 0 {
        print("الاستخدام: elomar run tool.elo <الاسم>")
        os.exit(2)
    }
    print("أهلًا يا {args[0]}")
    print(os.env("HOME"))
}
```

`elomar run tool.elo لينا` يمرر `["لينا"]`، و`os.exit(2)` ينهي البرنامج برمز الخروج 2.

## تشغيل برامج أخرى

```elomar
use std.process

fn main() {
    match process.run("git", ["log", "--oneline", "-3"]) {
        Ok(out) => print(out.stdout)
        Err(e) => print(e.kind(), e.message())
    }
}
```

يجب ذكر البرامج في `elomar.toml` (`commands = ["git"]` تحت `[permissions]`). وهي تعمل
بلا shell، فلا تُعامل المعاملات كأوامر أبدًا.

## مكتبات Python

```text
py.call<Float>("statistics", "mean", [1, 2, 3])
```

`std.py` تستدعي دوال Python بنتائج لها أنواع، بعد السماح بذلك بـ `python = true`.

## إلى أين بعد ذلك

- [السرعة والتوازي](11-speed.html): كيف تستخدم كل أنوية المعالج، وكيف تبقى برامجك سريعة.
- [مرجع اللغة](https://github.com/abobasel86/elomar-lang/blob/main/docs/grammar.md)
  يذكر كل دالة ونوع ورمز خطأ.
- [الأمثلة](https://github.com/abobasel86/elomar-lang/tree/main/examples)
  برامج كاملة تُختبر مع كل تعديل.
- الأسئلة والأفكار والأخطاء مرحب بها في
  [صفحة المشكلات](https://github.com/abobasel86/elomar-lang/issues).
