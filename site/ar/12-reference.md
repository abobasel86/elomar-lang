---
layout: default
title: المرجع
lang: ar
slug: 12-reference
---

# 12. المرجع

كل دالة مدمجة، وكل دالة تابعة لكل نوع، وكل دوال الوحدات القياسية في صفحة واحدة. تعرضها الدروس وهي تعمل، ويذكر
[مرجع اللغة](https://github.com/abobasel86/elomar-lang/blob/main/docs/grammar.md) كل تفصيل ورمز خطأ. الدالة المعلَّمة
بـ * تغيّر القيمة، فيجب أن تكون القيمة في متغير `var`.

## الكلمات المحجوزة {#keywords}

لكل كلمة محجوزة كتابة عربية، والكتابتان كلمة واحدة: يجوز المزج بينهما في الملف نفسه، ولا تصلح أي منهما اسمًا.

| الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|
| `let` | `ليكن` | `type` | `نوع` |
| `var` | `متغير` | `enum` | `تعداد` |
| `fn` | `دالة` | `match` | `طابق` |
| `return` | `أرجع` | `use` | `استخدم` |
| `if` | `إذا` | `pub` | `عام` |
| `else` | `وإلا` | `async` | `غير_متزامن` |
| `while` | `طالما` | `await` | `انتظر` |
| `for` | `لكل` | `trait` | `سمة` |
| `in` | `في` | `impl` | `نفذ` |
| `break` | `توقف` | `self` | `هذا` |
| `continue` | `تابع` | `as` | `بصفة` |
| `true` | `صح` | `div` | `قسمة` |
| `false` | `خطأ` |  | |

- تُقبل الكلمة بهمزة أو دونها (`أرجع` و`ارجع`، `إذا` و`اذا`)، و`فى` و`داله`، وتُتجاهل الحركات والشدة والتطويل (`نفّذ`).
- يبدأ البرنامج من `main`، أو من `رئيسية` إن لم تكن فيه `main`.
- `elomar fmt --keywords ar` يكتب كل الكلمات في الملف بالعربية، و`--keywords en` بالإنجليزية، ومعها الدوال والأنواع
  والدوال التابعة المدمجة (`print` تصير `اطبع`، و`.length` تصير `.طول`)، إلا حيث يكون في برنامجك شيء خاص بالاسم نفسه.
- زر **«دالة ليكن إذا»** فوق كل درس يعرض الأمثلة بالعربية، وزر **fn ⇄ دالة** في صفحة التجربة يحوّل برنامجك.
- وللدوال التابعة لكل نوع أسماء عربية أيضًا ([في الأسفل](#arabic-methods))، وكذلك الوحدات القياسية ([الجدول](#arabic-modules)).

## أسماء عربية للدوال والأنواع المدمجة {#arabic-names}

للدوال المدمجة والمُنشئات والأنواع الأساسية أسماء عربية أيضًا: `اطبع("مرحبًا")` هي `print("مرحبًا")`،
و`قائمة<نص>` هي `List<String>`. وهي غير محجوزة: إن كان في برنامجك `نص` خاص به (متغير أو دالة أو نوع)،
فهو المستعمل حيث يظهر.

| الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|
| `print` | `اطبع` | `min` | `أصغر` |
| `input` | `اقرأ` | `max` | `أكبر` |
| `to_string` | `إلى_نص` | `abs` | `مطلق` |
| `to_int` | `إلى_صحيح` | `all` | `الكل` |
| `to_float` | `إلى_عشري` | `timeout` | `مهلة` |
| `parse_int` | `حلل_صحيح` | `Some` | `قيمة` |
| `parse_float` | `حلل_عشري` | `None` | `لاشيء` |
| `assert` | `تأكد` | `Ok` | `نجاح` |
| `assert_eq` | `تأكد_تساوي` | `Err` | `فشل` |
| `set` | `مجموعة` |  | |

| الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|
| `Int` | `صحيح` | `Set` | `مجموعة` |
| `Float` | `عشري` | `Map` | `قاموس` |
| `Bool` | `منطقي` | `Option` | `اختياري` |
| `String` | `نص` | `Result` | `نتيجة` |
| `List` | `قائمة` | `Duration` | `مدة` |

## أسماء عربية للدوال التابعة {#arabic-methods}

للدوال التابعة للأنواع المدمجة وللخاصية `length` أسماء عربية أيضًا: `الطلاب.طول` هي `students.length`،
و`xs.حول(|x| x * 2)` هي `xs.map(|x| x * 2)`. ويختار المدقق الدالة الإنجليزية من نوع القيمة، فـ`.أضف`
هي `push` على القائمة و`add` على المجموعة. وإن كان للسجل حقل بالاسم نفسه، أو لك دالة في `impl` بالاسم
نفسه، فهي المستعملة. و`to_string` هي `إلى_نص` على كل نوع، وكل دالة متوازية `par_X` هي `X_بالتوازي`
(`par_map` هي `حول_بالتوازي`). وتُقبل الأسماء كما تُقبل الكلمات المحجوزة (لا تهم الهمزة ولا `ى` ولا `ة`
ولا الحركات ولا التطويل)، وتذكر الرسائل الأسماء الإنجليزية.

**القائمة** (`List`)

| الإنجليزية | العربية | الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|---|---|
| `length` | `طول` | `sum` | `مجموع` | `min` | `أصغر` |
| `push` | `أضف` | `slice` | `اقتطع` | `max` | `أكبر` |
| `pop` | `أخرج` | `map` | `حول` | `enumerate` | `مرقمة` |
| `insert` | `أدرج` | `filter` | `رشح` | `chunks` | `قطع` |
| `remove` | `احذف` | `any` | `أي` | `flat_map` | `حول_وادمج` |
| `contains` | `يحتوي` | `all` | `الكل` | `group_by` | `جمع_حسب` |
| `index_of` | `موقع` | `find` | `ابحث` | `zip` | `زاوج` |
| `is_empty` | `فارغ` | `count` | `عد` | `windows` | `نوافذ` |
| `first` | `الأول` | `fold` | `اطو` | `sum_by` | `مجموع_حسب` |
| `last` | `الأخير` | `sort_by` | `رتب_حسب` | `min_by` | `أصغر_حسب` |
| `reverse` | `اعكس` | `take` | `خذ` | `max_by` | `أكبر_حسب` |
| `sorted` | `مرتبة` | `skip` | `تخط` |  |  |
| `join` | `اربط` | `unique` | `فريدة` |  |  |

**النص** (`String`)

| الإنجليزية | العربية | الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|---|---|
| `length` | `طول` | `chars` | `حروف` | `lines` | `أسطر` |
| `upper` | `أحرف_كبيرة` | `replace` | `استبدل` | `words` | `كلمات` |
| `lower` | `أحرف_صغيرة` | `repeat` | `كرر` | `trim_start` | `شذب_البداية` |
| `trim` | `شذب` | `parse_int` | `حلل_صحيح` | `trim_end` | `شذب_النهاية` |
| `contains` | `يحتوي` | `parse_float` | `حلل_عشري` | `pad_start` | `املأ_البداية` |
| `starts_with` | `يبدأ_ب` | `is_empty` | `فارغ` | `pad_end` | `املأ_النهاية` |
| `ends_with` | `ينتهي_ب` | `index_of` | `موقع` | `reverse` | `اعكس` |
| `split` | `قسم` | `slice` | `اقتطع` |  |  |

**الأعداد** (`Int, Float`)

| الإنجليزية | العربية | الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|---|---|
| `to_float` | `إلى_عشري` | `to_int` | `إلى_صحيح` | `sqrt` | `جذر` |
| `abs` | `مطلق` | `round` | `قرب` | `round_to` | `قرب_إلى` |
| `pow` | `أس` | `floor` | `قرب_لأسفل` | `to_fixed` | `بخانات` |
| `clamp` | `حصر` | `ceil` | `قرب_لأعلى` |  |  |

**الاختياري** (`Option`)

| الإنجليزية | العربية | الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|---|---|
| `is_some` | `فيه_قيمة` | `map` | `حول` | `ok_or` | `أو_فشل` |
| `is_none` | `بلا_قيمة` | `and_then` | `ثم` | `is_some_and` | `قيمة_تحقق` |
| `unwrap_or` | `القيمة_أو` | `filter` | `رشح` |  |  |
| `unwrap_or_else` | `القيمة_أو_احسب` | `or` | `أو` |  |  |

**النتيجة** (`Result`)

| الإنجليزية | العربية | الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|---|---|
| `is_ok` | `ناجحة` | `unwrap_or_else` | `القيمة_أو_احسب` | `and_then` | `ثم` |
| `is_err` | `فاشلة` | `map` | `حول` | `ok` | `النجاح` |
| `unwrap_or` | `القيمة_أو` | `map_err` | `حول_الفشل` | `err` | `الفشل` |

**القاموس** (`Map`)

| الإنجليزية | العربية | الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|---|---|
| `length` | `طول` | `remove` | `احذف` | `map_values` | `حول_القيم` |
| `get` | `اجلب` | `keys` | `المفاتيح` | `any` | `أي` |
| `get_or` | `اجلب_أو` | `values` | `القيم` | `all` | `الكل` |
| `entries` | `المدخلات` | `is_empty` | `فارغ` | `count` | `عد` |
| `contains_key` | `فيه_مفتاح` | `filter` | `رشح` |  |  |

**المجموعة** (`Set`)

| الإنجليزية | العربية | الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|---|---|
| `length` | `طول` | `to_list` | `إلى_قائمة` | `filter` | `رشح` |
| `contains` | `يحتوي` | `union` | `اتحاد` | `map` | `حول` |
| `add` | `أضف` | `intersection` | `تقاطع` | `any` | `أي` |
| `remove` | `احذف` | `difference` | `فرق` | `all` | `الكل` |
| `is_empty` | `فارغ` | `is_subset` | `جزء_من` | `count` | `عد` |

**المدة والأخطاء** (`Duration, errors`)

| الإنجليزية | العربية | الإنجليزية | العربية | الإنجليزية | العربية |
|---|---|---|---|---|---|
| `millis` | `بالمللي` | `kind` | `النوع` |  |  |
| `seconds` | `بالثواني` | `message` | `الرسالة` |  |  |
## أسماء عربية للوحدات القياسية {#arabic-modules}

للوحدات القياسية ودوالها أسماء عربية أيضًا (منذ 1.17): `استخدم std.ملفات` هي `use std.fs`، و`ملفات.اقرأ_نص(المسار)`
هي `fs.read_text(المسار)`. وبعد استيراد الوحدة بأي كتابة يعمل اسماها كلاهما. وهي غير محجوزة: المتغير المسمى `وقت`
يحجب الوحدة `time` حيث يظهر. في كل خانة الاسم الإنجليزي ثم العربي.

| الوحدة | الدوال |
|---|---|
| `ai` `ذكاء` | `infer` `استنتج` · `run` `شغل` · `embed` `ضمن` · `similarity` `تشابه` · `search` `ابحث` · `stream` `بث` |
| `agent` `وكيل` | `run` `شغل` |
| `fs` `ملفات` | `read_text` `اقرأ_نص` · `write_text` `اكتب_نص` · `append_text` `ألحق_نص` · `exists` `موجود` · `list_dir` `محتويات` · `is_dir` `هل_مجلد` · `size` `حجم` · `create_dir` `أنشئ_مجلد` · `remove` `احذف` · `remove_dir` `احذف_مجلد` · `rename` `أعد_تسمية` · `copy` `انسخ` |
| `json` `جيسون` | `parse` `حلل` · `stringify` `إلى_نص` · `pretty` `منسق` |
| `http` `ويب` | `get` `اجلب` · `post` `أرسل` |
| `db` `بيانات` | `open` `افتح` · `execute` `شغل` · `query` `استعلم` · `query_one` `استعلم_واحدا` |
| `py` `بايثون` | `call` `استدع` |
| `mcp` `أدوات` | `tools` `الأدوات` · `call` `استدع` |
| `math` `رياضيات` | `sqrt` `جذر` · `cbrt` `جذر_تكعيبي` · `pow` `أس` · `exp` `أسي` · `ln` `لو_طبيعي` · `log10` `لو10` · `log2` `لو2` · `sin` `جا` · `cos` `جتا` · `tan` `ظا` · `asin` `جا_عكسي` · `acos` `جتا_عكسي` · `atan` `ظا_عكسي` · `atan2` `ظا_عكسي2` · `hypot` `وتر` · `is_nan` `ليس_عددا` · `pi` `باي` · `e` `هـ` · `tau` `تاو` · `inf` `لانهاية` |
| `random` `عشوائي` | `int` `صحيح` · `float` `عشري` · `bool` `منطقي` · `choice` `اختر` · `shuffle` `اخلط` · `seed` `بذرة` |
| `time` `وقت` | `now` `الآن` · `sleep` `نم` · `format` `نسق` · `parse` `حلل` · `today` `اليوم` · `date` `تاريخ` · `make` `اصنع` · `add_days` `أضف_أياما` · `add_months` `أضف_أشهرا` · `days_between` `أيام_بين` · `format_as` `نسق_بصيغة` |
| `os` `نظام` | `args` `المعطيات` · `env` `بيئة` · `platform` `المنصة` · `exit` `اخرج` |
| `regex` `أنماط` | `is_match` `يطابق` · `find` `ابحث` · `find_all` `ابحث_الكل` · `captures` `التقط` · `replace` `استبدل` · `split` `قسم` |
| `encoding` `ترميز` | `base64_encode` `إلى_قاعدة64` · `base64_decode` `من_قاعدة64` · `hex_encode` `إلى_ست_عشري` · `hex_decode` `من_ست_عشري` · `url_encode` `ترميز_رابط` · `url_decode` `فك_رابط` |
| `crypto` `تشفير` | `sha256` `بصمة` · `hmac_sha256` `توقيع` · `uuid` `معرف_فريد` · `token` `رمز_سري` |
| `server` `خادم` | `serve` `شغل` · `text` `نص` · `html` `صفحة` · `json` `جيسون` · `redirect` `حول` |
| `process` `عملية` | `run` `شغل` |
| `csv` `جدول` | `parse` `حلل` · `parse_records` `حلل_سجلات` · `stringify` `إلى_نص` |

الصلاحيات في `!{...}`: `io` `إدخال_إخراج` · `fs` `ملفات` · `net` `شبكة` · `ai` `ذكاء` · `db` `بيانات` · `py` `بايثون` · `mcp` `أدوات` · `process` `عملية`. مثلًا `دالة احفظ(س: نص) !{ملفات}`.
أما حقول السجلات التي ترجعها الوحدات (`status` و`body` و`year`) وأنواع الأخطاء (`FsError`) فتبقى بالإنجليزية.

## الدوال المدمجة

| الدالة | عملها |
|---|---|
| `print(x, ...)` | تطبع القيم في سطر واحد بينها مسافات |
| `input()` / `input(prompt)` | تقرأ سطرًا؛ و`""` عند نهاية الإدخال |
| `to_string(x)` | أي قيمة نصًّا |
| `to_int(x)` / `to_float(x)` | العدد `Int` (يُحذف الكسر) أو `Float` |
| `parse_int(s)` / `parse_float(s)` | النص عددًا: `Option<Int>` / `Option<Float>` |
| `min(a, b)` / `max(a, b)` / `abs(x)` | الأصغر، الأكبر، القيمة المطلقة |
| `set(list)` | مجموعة `Set` من عناصر القائمة |
| `assert(cond[, msg])` / `assert_eq(actual, expected)` | توقف البرنامج إن فشل التحقق (للاختبارات) |
| `all([futures])` | تنتظر عدة استدعاءات `async`: `await all([...])` |
| `timeout(future, limit)` | `await timeout(work, 2s)`: `Some(النتيجة)`، أو `None` إن انتهى الوقت |

## كل الأنواع

| الدالة | ما تعطيه |
|---|---|
| `to_string()` | القيمة نصًّا كما تعرضها `print` |

## القائمة `List<T>`

| الدالة | عملها |
|---|---|
| `.length` · `is_empty()` | عدد العناصر؛ هل هي فارغة |
| `push(x)`* · `pop()`* | تضيف في الآخر؛ تحذف الأخير: `Option<T>` |
| `insert(i, x)`* · `remove(i)`* | تضيف في الموضع `i`؛ تحذف عنصر الموضع `i` وتعطيه |
| `contains(x)` · `index_of(x)` | هل فيها `x`؛ وأين: `Option<Int>` |
| `first()` · `last()` | `Option<T>` |
| `take(n)` · `skip(n)` · `slice(start, end)` | أول `n`، الكل عدا أول `n`، جزء منها |
| `reverse()` · `sorted()` · `unique()` | معكوسة؛ مرتبة (أعداد أو نصوص)؛ كل عنصر مرة واحدة |
| `sum()` · `min()` · `max()` | للأعداد (و`min` و`max` للنصوص أيضًا): `max()` تعطي `Option<T>` |
| `join(sep)` | قائمة `List<String>` نصًّا واحدًا |
| `map(f)` · `filter(f)` | كل عنصر عبر `f`؛ العناصر التي تعطي لها `f` القيمة `true` |
| `any(f)` · `all(f)` · `count(f)` · `find(f)` | هل يحقق بعضها أو كلها الشرط؛ كم عنصرًا؛ أولها: `Option<T>` |
| `fold(init, f)` | تجمع العناصر بدءًا من `init`: `f(acc, item)` |
| `sort_by(f)` · `min_by(f)` · `max_by(f)` | مرتبة بالمفتاح `f(item)` (ترتيب مستقر)؛ العنصر ذو أصغر مفتاح أو أكبره |
| `sum_by(f)` | مجموع `f(item)` (`Int` أو `Float`) |
| `flat_map(f)` | القوائم التي تعطيها `f` موصولة في قائمة واحدة |
| `group_by(f)` | قاموس `Map` من كل مفتاح (`Int` أو `String`) إلى عناصره |
| `enumerate()` | سجلات `{ index, value }` |
| `zip(other)` | أزواج `(a, b)` بطول القائمة الأقصر |
| `chunks(n)` · `windows(n)` | أجزاء من `n` عناصر؛ كل `n` عناصر متجاورة |
| `par_map` · `par_filter` · `par_fold` · `par_any` · `par_all` · `par_find` · `par_count` · `par_sort_by` · `par_min_by` · `par_max_by` · `par_sum_by` · `par_flat_map` · `par_group_by` | مثل نظيراتها بلا `par_`، على عدة أنوية ([الدرس 11](11-speed.html)) |

## النص `String`

| الدالة | عملها |
|---|---|
| `.length` · `is_empty()` | عدد الحروف المرئية؛ هل هو `""` |
| `upper()` · `lower()` | بالأحرف الكبيرة؛ بالصغيرة |
| `trim()` · `trim_start()` · `trim_end()` | بلا مسافات حوله، في أوله، في آخره |
| `contains(s)` · `starts_with(s)` · `ends_with(s)` · `index_of(s)` | البحث؛ و`index_of` تعطي `Option<Int>` |
| `slice(start, end)` · `chars()` · `reverse()` | جزء، الحروف، معكوسًا (بالحروف المرئية) |
| `split(sep)` · `lines()` · `words()` | قائمة `List<String>` بالفاصل `sep`، بالأسطر، بالمسافات |
| `replace(a, b)` · `repeat(n)` | كل `a` تصبح `b`؛ مكررًا `n` مرات |
| `pad_start(n[, fill])` · `pad_end(n[, fill])` | مكمَّلًا إلى `n` حروف بمسافات أو بـ`fill` |
| `parse_int()` · `parse_float()` | عددًا: `Option` |

## الأعداد `Int` و`Float`

| الدالة | عملها |
|---|---|
| `abs()` · `pow(n)` · `clamp(min, max)` | القيمة المطلقة؛ القوة؛ حصره بين `min` و`max` (للنوعين) |
| `to_float()` | العدد `Int` بصيغة `Float` |
| `to_int()` · `round()` · `floor()` · `ceil()` | العدد `Float` بصيغة `Int`: بحذف الكسر، للأقرب، للأسفل، للأعلى |
| `sqrt()` · `round_to(digits)` · `to_fixed(digits)` | الجذر التربيعي؛ مقرَّبًا إلى `digits` منازل؛ نصًّا بـ`digits` منازل بالضبط |

## المدة `Duration`

تُكتب `500ms` و`10s` و`2m` و`1h`.

| الدالة | ما تعطيه |
|---|---|
| `millis()` · `seconds()` | المدة بالميلي ثانية (`Int`)، وبالثواني (`Float`) |

## `Option<T>` و`Result<T, E>`

| الدالة | عملها |
|---|---|
| `is_some()` · `is_none()` · `is_some_and(f)` | هل توجد قيمة (تحقق `f`) |
| `is_ok()` · `is_err()` | هل هي `Ok` أو `Err` |
| `unwrap_or(x)` · `unwrap_or_else(f)` | القيمة، أو `x`، أو ما تعطيه `f` |
| `map(f)` · `and_then(f)` | القيمة بعد تغييرها بـ`f`؛ و`f` تعطي `Option` أو `Result` آخر |
| `filter(f)` · `or(other)` | (`Option`) `None` إلا إن تحققت `f`؛ هذه أو `other` |
| `ok_or(e)` | (`Option`) بصيغة `Result`، و`e` مكان `None` |
| `map_err(f)` · `ok()` · `err()` | (`Result`) الخطأ بعد تغييره بـ`f`؛ القيمة أو الخطأ بصيغة `Option` |

## القاموس `Map<K, V>` والمجموعة `Set<T>`

| الدالة | عملها |
|---|---|
| `m[k]` · `get(k)` · `get_or(k, default)` | قيمة المفتاح: يتوقف البرنامج إن غاب؛ `Option<V>`؛ أو `default` |
| `contains_key(k)` · `remove(k)`* | هل المفتاح موجود؛ تحذفه: `Option<V>` |
| `keys()` · `values()` · `entries()` | قوائم المفاتيح والقيم وسجلات `{ key, value }` |
| `.length` · `is_empty()` | (للنوعين) عدد العناصر؛ هل هو فارغ |
| `contains(x)` · `add(x)`* · `remove(x)`* | (`Set`) هل فيها `x`؛ تضيف؛ تحذف: `Bool` |
| `union(s)` · `intersection(s)` · `difference(s)` · `is_subset(s)` | (`Set`) عمليات المجموعات |
| `to_list()` | (`Set`) العناصر قائمة `List` مرتبة |
| `filter(f)` · `any(f)` · `all(f)` · `count(f)` | (للنوعين) بالدالة `f(key, value)` للقاموس و`f(item)` للمجموعة؛ و`filter` تعطي النوع نفسه |
| `map_values(f)` | (`Map`) كل قيمة بعد تغييرها بـ`f` |
| `map(f)` | (`Set`) كل عنصر عبر `f`، مجموعةً (عناصرها `Int` أو `String`) |

## الأخطاء

`AIError` و`FsError` و`JsonError` و`HttpError` و`DbError` و`PyError` و`McpError` و`ProcessError` تعدادات، وكل حالة
فيها تحمل رسالة.

| الدالة | ما تعطيه |
|---|---|
| `kind()` · `message()` | اسم الحالة، مثل `"NotFound"`؛ والرسالة |

## الوحدات القياسية

استخدم الوحدة بـ`use std.<الاسم>`. الوحدات التي تتعامل مع العالم الخارجي تحتاج إذنًا في `elomar.toml`
([الدرس 7](07-data.html)).

| الوحدة | الدوال |
|---|---|
| `std.math` | `math.sqrt` · `math.cbrt` · `math.pow` · `math.exp` · `math.ln` · `math.log10` · `math.log2` · `math.sin` · `math.cos` · `math.tan` · `math.asin` · `math.acos` · `math.atan` · `math.atan2` · `math.hypot` · `math.is_nan`؛ والقيم `math.pi` و`math.e` و`math.tau` و`math.inf` |
| `std.random` | `random.int(min, max)` · `random.float()` · `random.bool()` · `random.choice(list)` · `random.shuffle(list)` · `random.seed(n)` |
| `std.time` | `time.now()` (بالميلي ثانية منذ 1970) · `time.today()` · `time.sleep(d)` · `time.format(t)` · `time.format_as(t, pattern)` · `time.parse(text)` · `time.date(t)` · `time.make(y, m, d, ...)` · `time.add_days(t, n)` · `time.add_months(t, n)` · `time.days_between(a, b)` |
| `std.os` | `os.args()` · `os.env(name)` · `os.platform()` · `os.exit(code)` |
| `std.fs` | `fs.read_text` · `fs.write_text` · `fs.append_text` · `fs.exists` · `fs.list_dir` · `fs.is_dir` · `fs.size` · `fs.create_dir` · `fs.remove` · `fs.remove_dir` · `fs.rename` · `fs.copy` (كلها تعطي `Result<_, FsError>`) |
| `std.json` | `json.parse<T>(text)` · `json.stringify(value)` · `json.pretty(value)` |
| `std.csv` | `csv.parse(text)` · `csv.parse_records(text)` · `csv.stringify(rows)` |
| `std.db` | `db.open(path)` · `db.execute(conn, sql[, params])` · `db.query<T>(conn, sql[, params])` · `db.query_one<T>(conn, sql[, params])` (SQLite) |
| `std.http` | `http.get(url)` · `http.post(url, body[, content_type])` |
| `std.server` | `server.serve(address, handler[, max_requests])` · `server.text(status, body)` · `server.html(status, body)` · `server.json(status, value)` · `server.redirect(url)` |
| `std.regex` | `regex.is_match` · `regex.find` · `regex.find_all` · `regex.captures` · `regex.replace` · `regex.split` (النمط أولًا ثم النص) |
| `std.encoding` | `encoding.base64_encode` · `encoding.base64_decode` · `encoding.hex_encode` · `encoding.hex_decode` · `encoding.url_encode` · `encoding.url_decode` |
| `std.crypto` | `crypto.sha256(text)` · `crypto.hmac_sha256(key, message)` · `crypto.uuid()` · `crypto.token(bytes)` |
| `std.process` | `process.run(command, args[, timeout])` |
| `std.py` | `py.call<T>(module, function, args[, timeout])` |
| `std.ai` | `ai.infer<T>(model, instructions, input[, timeout])` · `ai.run<T>(model, instructions, input, tools, ...)` · `ai.stream(model, instructions, input, on_text[, timeout])` · `ai.embed(model, text[, timeout])` · `ai.similarity(a, b)` · `ai.search(model, query, texts[, timeout])` |
| `std.agent` | `agent.run<T>(model, instructions, input, tools, mcp, approve, ...)` |
| `std.mcp` | `mcp.tools(server)` · `mcp.call(server, tool, args)` |

## الأوامر

| الأمر | عمله |
|---|---|
| `elomar run file.elo` | يفحص البرنامج ويشغّله |
| `elomar check file.elo` | يفحصه فقط |
| `elomar repl` | اكتب شيفرة وشاهد النتيجة فورًا |
| `elomar fmt` | ينسّق الشيفرة بالأسلوب القياسي؛ و`--keywords ar` أو `en` يكتب كل الكلمات المحجوزة والأسماء المدمجة بلغة واحدة |
| `elomar test` | يشغّل اختبارات المشروع |
| `elomar new name` | ينشئ مشروعًا |
| `elomar new name --ai PROVIDER` | ينشئ مشروعًا يستدعي نموذجًا: `mock` أو `ollama` أو `openai` أو `anthropic` |
| `elomar doctor` | يفحص الإعداد: مفاتيح الذكاء الاصطناعي والصلاحيات والخوادم ومعرّفات النماذج؛ وPython وخوادم MCP والحزم والمجلدات |
| `elomar upgrade [--check]` | ينتقل إلى أحدث إصدار (أو يخبر فقط إن كان هناك أحدث) |
| `elomar build file.elo` | يصنع ملفًا تنفيذيًا |
| `elomar add` · `elomar remove` · `elomar update` · `elomar search` · `elomar publish` | الحزم ([الدرس 6](06-projects.html)) |
| `elomar lsp` · `elomar dap` | خادم اللغة والمنقّح، للمحررات |
| `elomar --lang ar ...` | الرسائل بالعربية (أو اضبط `ELOMAR_LANG=ar`) |
