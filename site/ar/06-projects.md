---
layout: default
title: المشاريع والوحدات والاختبارات
lang: ar
slug: 06-projects
---

# 6. المشاريع والوحدات والاختبارات

## مشروع جديد

```text
elomar new school
cd school
elomar run src/main.elo
elomar test
```

هذا ينشئ:

```text
school/
├── elomar.toml               الإعدادات والصلاحيات
├── src/main.elo              البرنامج
├── src/greeting.elo          وحدة
└── tests/greeting_test.elo   الاختبارات
```

## الوحدات (Modules)

كل ملف في `src/` وحدة. ولا يُستخدم من ملف آخر إلا ما عُلّم بـ `pub`:

```elomar
// src/grading.elo
pub fn level(grade: Int) -> String {
    return match grade >= 90 {
        true => "ممتاز"
        false => "استمر"
    }
}
```

```elomar
// src/main.elo
use app.grading

fn main() {
    print(grading.level(95))
}
```

ويمكنك استيراد الأسماء مباشرة: `use app.grading.{level}` ثم استدعاء `level(95)`.

ويمكن أن تحمل وحدتك اسم وحدة قياسية: ملفك `src/csv.elo` تستورده بـ `use app.csv`، فيستدعي
`csv.parse(...)` دالتك أنت. لكن الملف الواحد لا يستطيع أن يستورد `std.csv` و`app.csv` معًا،
لأن `csv` سيعني حينها شيئين.

## الاختبارات

كل ملف `.elo` في مجلد `tests/` يحتوي اختبارات: دوال تبدأ أسماؤها بـ `test_`:

```elomar
// tests/grading_test.elo
use app.grading

fn test_high_grades_are_excellent() {
    assert_eq(grading.level(95), "ممتاز")
}

fn test_low_grades() {
    assert(grading.level(10) != "ممتاز")
}
```

`elomar test` يشغلها كلها ولا يعرض إلا ما فشل. ولتشغيل بعضها فقط:
`elomar test --filter high`.

## التنسيق

`elomar fmt` يعيد كتابة ملفاتك بأسلوب موحد (4 مسافات، ومسافات حول العوامل، وغير ذلك).
و`elomar fmt --check` يكتفي بذكر الملفات التي تحتاج تنسيقًا، وهذا مفيد في CI.

## مشاركة برنامجك

`elomar build` يصنع من برنامجك ملفًا تنفيذيًا واحدًا، بداخله وحداته وحزمه و`elomar.toml`.
أرسله لأي شخص: يعمل دون تثبيت Elomar-Lang.

```text
elomar build src/main.elo -o grades.exe
grades.exe لينا عمر            المعاملات تصل إلى البرنامج (os.args())
```

الملف التنفيذي للنظام الذي بُني عليه: ابنِ على Windows لـ Windows، وعلى Linux لـ Linux.
والمسارات في البرنامج، والمجلدات في `[permissions]`، تبدأ من المجلد الذي يُشغَّل منه.

## الحزم

استخدم كودًا كتبه غيرك:

```text
elomar search text            ابحث عن حزم في السجل العام
elomar add slugify            أضف حزمة إلى elomar.toml ونزّلها
elomar add tools --git https://github.com/user/tools --tag v1.0.0
elomar update                 انتقل إلى أحدث الإصدارات المسموحة
```

الملف `elomar.lock` يسجل الإصدارات بدقة، فيبني الجميع الشيء نفسه. ولمشاركة حزمتك
راجع [سجل الحزم](https://github.com/abobasel86/elomar-registry).
