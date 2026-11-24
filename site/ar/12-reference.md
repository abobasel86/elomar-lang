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
| `std.ai` | `ai.infer<T>(model, instructions, input[, timeout])` · `ai.run<T>(model, instructions, input, tools, ...)` |
| `std.agent` | `agent.run<T>(model, instructions, input, tools, mcp, approve, ...)` |
| `std.mcp` | `mcp.tools(server)` · `mcp.call(server, tool, args)` |

## الأوامر

| الأمر | عمله |
|---|---|
| `elomar run file.elo` | يفحص البرنامج ويشغّله |
| `elomar check file.elo` | يفحصه فقط |
| `elomar repl` | اكتب شيفرة وشاهد النتيجة فورًا |
| `elomar fmt` | ينسّق الشيفرة بالأسلوب القياسي |
| `elomar test` | يشغّل اختبارات المشروع |
| `elomar new name` | ينشئ مشروعًا |
| `elomar build file.elo` | يصنع ملفًا تنفيذيًا |
| `elomar add` · `elomar remove` · `elomar update` · `elomar search` · `elomar publish` | الحزم ([الدرس 6](06-projects.html)) |
| `elomar lsp` · `elomar dap` | خادم اللغة والمنقّح، للمحررات |
| `elomar --lang ar ...` | الرسائل بالعربية (أو اضبط `ELOMAR_LANG=ar`) |
