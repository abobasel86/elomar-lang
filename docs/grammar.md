# Elomar-Lang — القواعد النحوية والدلالية للنواة (المراحل 1 و2 و3)

> **هذا الملف هو المرجع الملزم** لما ينفذه المفسّر الحالي. عند أي تعارض مع
> `Elomar-Lang_Master_Specification_AR.md` يُعتمد هذا الملف، ويُحدَّث مع كل
> تغيير في `crates/`. كل قاعدة هنا لها اختبار في `crates/*/src/**` أو
> `tests/fail/` أو `examples/`.

الإصدار: `0.1.0-draft.2` · الحالة: منفذ ومختبر.

---

## 1. البنية المعجمية (Lexical)

| العنصر | القاعدة |
|---|---|
| الترميز | UTF-8 |
| التعليقات | `// ...` حتى نهاية السطر، و`/* ... */` غير متداخلة؛ التعليق الممتد عبر أسطر يُعامل كنهاية سطر |
| المعرفات | `(XID_Start \| "_") XID_Continue*` وفق Unicode UAX #31، مع تطبيع **NFC**. الأسماء العربية مسموحة |
| أعداد صحيحة | `0`، `42`، `1_000_000` → `Int` (64 بت). العدد الأكبر من السعة خطأ `E0005` |
| أعداد عشرية | `3.14`، `2.5e3`، `1e-3` → `Float`. `1..5` نطاق وليس `1.` |
| مدد زمنية | `500ms`، `10s`، `2m`، `1h` → `Duration`. أي لاحقة أخرى ملتصقة بعدد خطأ `E0006` |
| نصوص | `"..."` في سطر واحد، والهروب: `\n \t \r \0 \" \\ \u{XXXX}` |
| كلمات محجوزة | `let var fn return if else while for in break continue true false type enum match use pub async await trait impl self as div` |
| أسماء الـ Prelude | `Some None Ok Err` ودوال القسم 7 — ليست كلمات محجوزة لكن لا يجوز إعادة تعريفها |

### 1.1 نهاية السطر

يُبقي الـ Lexer نهاية السطر **فاصلَ جمل** فقط إذا تحققت الشروط الثلاثة:

1. آخر رمز قبلها: معرّف، أو قيمة حرفية، أو `true`/`false`/`self`، أو `)` `]` `}`، أو `return`/`break`/`continue`.
2. لسنا داخل `( )` أو `[ ]` مفتوحة.
3. أول رمز بعدها ليس `.` ولا `else` ولا نهاية الملف.

والأسطر الفارغة المتتالية تُدمج. الفاصلة المنقوطة `;` فاصل صريح اختياري.

```elomar
let total = price *
    quantity              // يكمل: السطر السابق انتهى بـ *
let names = list
    .sorted()             // يكمل: السطر يبدأ بـ .
if ok {
} else {                  // else في السطر نفسه أو التالي — كلاهما صحيح
}
let a = 1; let b = 2      // جملتان في سطر واحد تحتاجان ;
```

قوس الكتلة `{` يجب أن يكون في سطر الرأس (`if`/`while`/`for`/`fn`/`match`).

---

## 2. النحو (EBNF)

`SEP = NEWLINE | ";"`. القوسان `{ }` في EBNF تعني التكرار.

```ebnf
program     = { SEP } , { item , ( SEP | EOF ) , { SEP } } ;
item        = use_decl | [ "pub" ] , ( type_decl | enum_decl | const_decl | trait_decl ) | fn_decl | impl_decl ;
trait_decl  = "trait" , IDENT , "{" , { fn_sig , SEP } , "}" ;     (* التواقيع بلا أجسام *)
impl_decl   = "impl" , [ path , "for" ] , type , "{" , { [ "pub" ] , fn_decl , SEP } , "}" ;
fn_sig      = [ "async" ] , "fn" , IDENT , "(" , "self" , { "," , param } , ")" , [ "->" , type ] , [ effects ] ;
bound_list  = "<" , IDENT , [ ":" , path , { "+" , path } ] , { "," , ... } , ">" ;   (* `fn f<T: A + B>` *)
path        = IDENT , { "." , IDENT } ;

use_decl    = "use" , IDENT , { "." , IDENT } ;
type_decl   = "type" , IDENT , [ "<" , IDENT , { "," , IDENT } , ">" ] , "=" , type ;
enum_decl   = "enum" , IDENT , [ "<" , IDENT , { "," , IDENT } , ">" ] , "{" , [ variant , { ( "," | NEWLINE ) , variant } ] , "}" ;
variant     = IDENT , [ "(" , type , { "," , type } , ")" ] ;
const_decl  = "let" , IDENT , [ ":" , type ] , "=" , expr ;
fn_decl     = [ "pub" ] , [ "async" ] , "fn" , IDENT , [ "<" , IDENT , { "," , IDENT } , ">" ] ,
              "(" , [ param , { "," , param } , [ "," ] ] , ")" ,
              [ "->" , type ] , [ effects ] , block ;
param       = IDENT , ":" , type ;
effects     = "!" , "{" , [ IDENT , { "," , IDENT } ] , "}" ;

type        = path , [ "<" , type , { "," , type } , ">" ]   (* `students.Student` *)
            | "{" , [ field_ty , { ( "," | NEWLINE ) , field_ty } ] , "}"
            | "fn" , "(" , [ type , { "," , type } ] , ")" , [ "->" , type ] ;   (* `fn(Int) -> Bool` *)
field_ty    = IDENT , ":" , type ;

block       = "{" , { SEP } , { stmt , ( SEP | before "}" ) , { SEP } } , "}" ;
stmt        = ( "let" | "var" ) , IDENT , [ ":" , type ] , "=" , expr
            | place , ( "=" | "+=" | "-=" | "*=" | "/=" ) , expr
            | "if" , expr , block , [ "else" , ( block | if_stmt ) ]
            | "while" , expr , block
            | "for" , IDENT , "in" , expr , block
            | "return" , [ expr ] | "break" | "continue"
            | block
            | expr ;
place       = IDENT , { "." , IDENT | "[" , expr , "]" } ;

expr        = binary ;                       (* الأسبقية في القسم 3 *)
unary       = ( "-" | "!" | "await" ) , unary | postfix ;
postfix     = primary , { "." , IDENT
                        | "[" , expr , "]"
                        | "?"
                        | [ type_args ] , "(" , [ arg , { "," , arg } , [ "," ] ] , ")" } ;
type_args   = "<" , type , { "," , type } , ">" ;   (* بقاعدة فك الغموض 3.1 *)
arg         = [ IDENT , ":" ] , expr ;
primary     = INT | FLOAT | DURATION | STRING | "true" | "false" | IDENT
            | "(" , expr , ")"
            | "[" , [ expr , { "," , expr } , [ "," ] ] , "]"
            | "[" , ( ":" | expr , ":" , expr , { "," , expr , ":" , expr } , [ "," ] ) , "]"
            | "{" , [ field , { ( "," | NEWLINE ) , field } ] , "}"
            | "match" , expr , "{" , { arm , ( "," | SEP ) } , "}"
            | lambda ;
lambda      = ( "||" | "|" , [ lparam , { "," , lparam } ] , "|" ) , ( block | expr ) ;
lparam      = IDENT , [ ":" , type ] ;
field       = IDENT , [ ":" , expr ] ;           (* { name } اختصار لـ { name: name } *)
arm         = pattern , "=>" , ( block | expr ) ;
pattern     = "_" | IDENT | IDENT , "(" , pattern , ")"
            | path , "." , IDENT , [ "(" , pattern , { "," , pattern } , ")" ]   (* `lib.Level.High` *)
            | [ "-" ] , ( INT | FLOAT ) | STRING | "true" | "false" ;
```

- الجملة التي تنتهي بـ `}` لا تحتاج فاصلًا بعدها.
- في **موضع الجملة** تبدأ `{` كتلةً؛ في **موضع التعبير** تبدأ سجلًا. فرع `match` وجسم الدالة المجهولة اللذان يبدآن بـ `{` كتلة؛ لإرجاع سجل اكتب `( { ... } )`.
- `if` جملة وليست تعبيرًا؛ للاختيار بين قيمتين استخدم `match` على `Bool`.

---

## 3. الأسبقية

| من الأعلى | العمليات | التجميع |
|---|---|---|
| 1 | `.f` `f()` `a[i]` `x?` | يسار |
| 2 | `-x` `!x` `await x` | — |
| 3 | `*` `/` `div` `%` | يسار |
| 4 | `+` `-` | يسار |
| 5 | `..` `..=` | غير تجميعي |
| 6 | `<` `<=` `>` `>=` | يسار |
| 7 | `==` `!=` | يسار |
| 8 | `&&` | يسار، قصير الدارة |
| 9 | `\|\|` | يسار، قصير الدارة |

### 3.1 فك الغموض بين `<` ومعاملات النوع

بعد اسم أو `a.b`، يحاول المحلل قراءة `<T, ...>`؛ إن نجح وتلاه `(` مباشرة فهو
استدعاء بمعاملات نوع (`ai.infer<Ticket>(x)` أو `first<Int>(xs)`)، وإلا فهو المقارنة `<`.
تقبل معاملات النوع `ai.infer` و`json.parse` والدوال العامة؛ وفي غيرها تُرفض بـ `E0103`.

### 3.2 الشريط `|`

`|` لا يظهر إلا حول معاملات الدالة المجهولة (`|x| x * 2`)، و`||` في موضع القيمة
دالة مجهولة بلا معاملات (`|| 1`)، وبين قيمتين هو "أو" المنطقية.

---

## 4. نظام الأنواع

| النوع | القيم | ملاحظات |
|---|---|---|
| `Int` | أعداد صحيحة 64 بت | التجاوز خطأ تنفيذي `R0002` |
| `Float` | IEEE-754 64 بت | يُطبع `85.0` لا `85` |
| `Bool` | `true` `false` | الشروط تقبل `Bool` فقط |
| `String` | نص Unicode | `+` للدمج؛ لا فهرسة بالأرقام |
| `Unit` | لا قيمة | نوع الدالة بلا `->` |
| `List<T>` | `[1, 2]` | عناصر متجانسة؛ `+` للدمج |
| `Option<T>` | `Some(x)` `None` | |
| `Result<T, E>` | `Ok(x)` `Err(e)` | |
| `Range` | `a..b` `a..=b` | للحلقات |
| `Duration` | `500ms` `10s` `2m` `1h` | `+` و`-` والمقارنة؛ `.millis()` و`.seconds()` |
| `Future<T>` | ناتج استدعاء `async fn` أو `ai.infer` | يُستخرج `T` بـ `await` |
| `Map<K, V>` | `["a": 1]` و`[:]` | المفاتيح `String` أو `Int`؛ الترتيب بحسب المفتاح |
| `Set<T>` | `set([1, 2])` | العناصر `String` أو `Int` بلا تكرار، مرتبة؛ تُطبع `set([1, 2])` |
| أنواع معدودة | `Shape.Circle(1.0)` `Priority.High` | اسمية: `enum` بالحالات نفسها نوع مختلف |
| `AIError` | `AIError.Timeout("...")` | نوع معدود مدمج؛ كل حالة تحمل رسالة |
| `FsError` | `NotFound` · `PermissionDenied` · `PolicyDenied` · `InvalidData` · `Other` | لـ `std.fs`؛ كل حالة تحمل رسالة |
| `JsonError` | `InvalidJson` · `TypeMismatch` | لـ `std.json`؛ كل حالة تحمل رسالة |
| `{ f: T, ... }` | `{ f: v }` | **بنيوي**: الحقول نفسها بالأنواع نفسها = النوع نفسه |

- `type X = ...` اسم مستعار، والأسماء المستعارة الدائرية خطأ `E1006`.
- **الاستنتاج المحلي** بالتوحيد (Unification): `let xs = []` ثم `xs.push(1)` يجعلها `List<Int>`.
  الدوال يجب أن تعلن أنواع معاملاتها؛ نوع الإرجاع الافتراضي `Unit`.
- **لا تحويل ضمني**، باستثناءين محددين:
  1. **الترقية العددية:** في `+ - * / %` والمقارنات، إذا كان أحد الطرفين `Float` تُرقّى `Int`.
  2. **الأعداد الحرفية:** العدد الصحيح الحرفي في موضع يُتوقع فيه `Float` يصبح عشريًا (`var t: Float = 0`).
- `/` تعطي **دائمًا** `Float`؛ `div` قسمة صحيحة تقطع نحو الصفر و`%` باقيها (`Int` فقط لـ `div`).
- القسمة على صفر (`/` `div` `%` صحيحة أو عشرية) خطأ `R0001`.

### 4.1 دلالة القيمة

الإسناد والتمرير ينسخان القيمة منطقيًا (التنفيذ: عدّ مراجع مع نسخ عند الكتابة).
التعديل عبر `x = ...` و`x[i] = ...` و`x.f = ...` و`x.push(...)`... مسموح فقط إذا كان
جذر المسار متغير `var` محليًا؛ المعاملات و`let` والثوابت غير قابلة للتعديل (`E1003`).

### 4.2 الأسماء والنطاقات

- لكل كتلة نطاق. تعريف الاسم مرتين في النطاق نفسه `E1002`؛ حجب متغير من نطاق خارجي تحذير `W1001`.
- البحث: المتغيرات المحلية ← ثوابت الملف ← الدوال ← الـ Prelude.
- ثوابت الملف (`let` على مستوى الملف) تُقيَّم بالترتيب قبل `main`، ولا ترى إلا ما قبلها.
- الدوال ترى كل الدوال وكل الثوابت بغض النظر عن الترتيب.
- الدوال ليست قيمًا بعد (`E0103`).

### 4.3 `match`

- الأنماط: `_`، اسم ربط، `Some(p)`/`Ok(p)`/`Err(p)`/`None`، أعداد ونصوص و`true`/`false`.
- **الشمول إلزامي** (`E2009`): `Option` يحتاج `Some(_)` و`None`، و`Result` يحتاج `Ok(_)` و`Err(_)`،
  و`Bool` يحتاج `true` و`false`، وغيرها يحتاج `_` أو اسم ربط.
- كتعبير: كل الفروع بنوع واحد. كجملة: لا يُشترط توافق أنواع الفروع.

### 4.4 الإرجاع

الدالة ذات نوع إرجاع غير `Unit` يجب أن ينتهي كل مسار فيها بـ `return` (`E2007`).
يُعد المسار منتهيًا إذا: كان `return`، أو `if/else` كل فروعه منتهية، أو `match` كل
فروعه كتل منتهية، أو `while true` بلا `break`.

---

## 4.5 `async` و`await`

- `async fn f() -> T` عند استدعائها تُرجع `Future<T>` **دون تنفيذ جسمها**.
- `await e` تنفذ الـ Future عند أول انتظار وتُرجع `T`؛ الانتظار مرة ثانية للقيمة نفسها يعيد النتيجة المحفوظة دون إعادة التنفيذ.
- `await` مسموحة داخل `async fn` فقط (`E2013`)، وعلى قيمة `Future` فقط (`E2014`).
- `Future` يُحسب ولا يُنتظر في جملة مستقلة تحذير `W1003` لأنه لن يُنفَّذ.
- `async fn main()` مسموحة؛ والبيئة تنتظر ناتجها تلقائيًا.
- **لا يوجد تنفيذ متوازٍ** في هذه المرحلة: المنفّذ (Executor) تسلسلي. المهام المتوازية والإلغاء في v0.3.
- تُحسب تأثيرات الدالة `async` عند موضع استدعائها.

## 4.6 الوحدة `std.ai`

```elomar
use std.ai

type Ticket = { category: String, priority: String, summary: String }

pub async fn classify(message: String) -> Result<Ticket, AIError> !{ai} {
    return await ai.infer<Ticket>(
        model: "support.classifier",          // اسم منطقي من elomar.toml
        instructions: "Classify the request.",
        input: message,                        // بيانات غير موثوقة، لا تعليمات
        timeout: 10s                           // اختياري
    )
}
```

| القاعدة | الرمز |
|---|---|
| استخدام `ai` يتطلب `use std.ai` | `E1001` |
| `ai.infer<T>(model: String, instructions: String, input, timeout: Duration)` → `Future<Result<T, AIError>>`؛ المعاملات بالموضع أو بالاسم، و`timeout` اختياري | `E2002` / `E2010` |
| `T` إلزامي ويجب أن يكون له تمثيل JSON: `String` و`Int` و`Float` و`Bool` و`List<T>` و`Option<T>` والسجلات منها؛ والقاعدة نفسها على نوع `input` | `E2008` / `E2015` |
| الاستدعاء يُحدث التأثير `ai` | `E3001` / `E3002` |

**مسار التنفيذ:** فحص السياسة (`max_calls_per_run`) ← البحث عن النموذج ← استدعاء المزود ← تحليل JSON ←
**التحقق من مطابقة `T`** (الحقول الإلزامية، وغياب الحقول الزائدة، والأنواع) ← `Ok(T)` أو `Err(AIError)`.
المطابقة البنيوية لا تعني صحة المعلومة واقعيًا. حين يكون `T` هو `String` يُقبل النص الحر.

**حالات `AIError`** (كل منها تحمل نص الرسالة): `Authentication` · `ProviderUnavailable` · `RateLimited` · `Timeout` · `Network` ·
`UnsupportedCapability` · `InvalidStructuredOutput` · `PolicyDenied` · `BudgetExceeded` · `ContextLimitExceeded` · `Cancelled`.
كل فشل في الاستدعاء قيمة `Err` يعالجها البرنامج، وليس خطأ تنفيذ يوقفه:

```elomar
match await classify(message) {
    Ok(t) => print(t.summary)
    Err(AIError.Timeout(_)) => print("try again later")
    Err(e) => print(e.kind(), e.message())   // e.kind() اسم الحالة نصًا
}
```

النوع المعدود الذي حالاته أسماء فقط (بلا قيم) يمكن أن يكون جزءًا من `T` أو من `input`، ويُرسل إلى النموذج قيمةً نصية
من حالاته (`{"type": "string", "enum": [...]}`)؛ والإجابة بقيمة خارج الحالات ← `InvalidStructuredOutput`.

### 4.6.1 `elomar.toml` والمزود الوهمي (Mock)

يُقرأ أقرب `elomar.toml` في مجلد الملف أو المجلدات الأعلى منه. النموذج `mock` متاح دائمًا دون إعداد، ويُرجع قيمة بديلة
من النوع المطلوب: `"mock"` و`0` و`0.0` و`false` و`[]` و`None`.

```toml
[ai.models."support.classifier"]
provider = "mock"          # المزودات السحابية لم تُنفذ بعد: تُرجع ProviderUnavailable

[[ai.models."support.classifier".mock]]   # تُجرَّب القواعد بالترتيب
input_contains = "password"               # اختياري؛ دونه تنطبق القاعدة على كل مدخل
response = '{"category": "account", "priority": "high", "summary": "..."}'

[[ai.models."support.classifier".mock]]
input_contains = "crash"
delay_ms = 15000           # زمن استجابة محاكى؛ إن تجاوز timeout ← Timeout (لا انتظار فعلي)
response = '{}'

[[ai.models."support.classifier".mock]]
error = "RateLimited"      # أي نوع من AIError.kind
message = "retry later"

[ai.policy]
max_calls_per_run = 10     # بعده ← BudgetExceeded
default_timeout_ms = 30000
```

إن لم تنطبق أي قاعدة يُرجع المزود قيمة بديلة. الأخطاء في الملف (مثل نوع خطأ غير معروف أو مفتاح مكتوب خطأً
في قاعدة) توقف التشغيل قبل البدء برمز خروج 2.

### 4.6.2 مزود OpenAI والصلاحيات

```toml
[ai.models."support.classifier"]
provider = "openai"
model = "..."                      # معرّف نموذج OpenAI (إلزامي، ولا يُكتب في الشيفرة)
api_key_env = "OPENAI_API_KEY"     # اسم متغير البيئة؛ الافتراضي OPENAI_API_KEY
# base_url = "https://gateway.example.com/v1"   # اختياري؛ https إلزامي إلا لـ localhost

[permissions]
network_hosts = ["api.openai.com"] # منع افتراضي: أي مضيف غير مذكور يُرفض بـ PolicyDenied
```

| القاعدة | النتيجة |
|---|---|
| استدعاء نموذج سحابي (أي مزود غير `mock`) يُحدث `net` إضافة إلى `ai`. إن لم يكن اسم النموذج نصًا حرفيًا يُفترض أنه سحابي متى وُجد نموذج سحابي في الإعداد | `E3001` / `E3002` وقت الترجمة |
| المضيف (أو `host:port`) غير موجود في `network_hosts` | `Err` بنوع `PolicyDenied` دون أي اتصال، ولا يُحتسب ضمن `max_calls_per_run` |
| متغير البيئة غير معرّف أو فارغ | `Authentication` دون أي اتصال |
| HTTP 401/403 → `Authentication` · 429 → `RateLimited` (أو `BudgetExceeded` عند نفاد الرصيد) · 400 لطول السياق → `ContextLimitExceeded` · 400/422 أخرى → `UnsupportedCapability` · 5xx و404 → `ProviderUnavailable` · انقضاء المهلة → `Timeout` · فشل الاتصال → `Network` · رفض النموذج → `PolicyDenied` · قطع الإجابة → `ContextLimitExceeded` | قيمة `Err` |
| `base_url` بـ `http` لمضيف غير محلي، أو نموذج بلا `model` | خطأ إعداد قبل التشغيل (رمز خروج 2) |

**تفاصيل التنفيذ:** واجهة Chat Completions مع Structured Outputs بوضع `strict`؛ الأنواع غير السجلية تُغلَّف بـ `{ "value": ... }`
ثم تُفك، و`String` يُطلب نصًا حرًا. `instructions` في رسالة النظام مع تنبيه بأن رسالة المستخدم بيانات لا تعليمات، و`input`
في رسالة المستخدم. إعادة التوجيه (Redirects) لا تُتبع، والمفتاح يُقرأ عند كل استدعاء ويُخفى (`sk-***`) في رسائل الخطأ.

## 4.7 الأنواع المعدودة (`enum`)

```elomar
enum Shape {
    Circle(Float)
    Rect(Float, Float)
    Empty
}

fn area(s: Shape) -> Float {
    return match s {
        Shape.Circle(r) => 3.14 * r * r
        Shape.Rect(w, h) => w * h
        Shape.Empty => 0
    }
}
```

- الحالات تُفصل بفاصلة أو سطر جديد؛ لكل حالة قيم موضعية اختيارية.
- الإنشاء والأنماط بالاسم الكامل `Shape.Circle(r)`؛ الحالة بلا قيم تُكتب دون أقواس (`E2002` إن خالفت).
- الحالة غير الموجودة `E1010` مع اقتراح أقرب اسم.
- `match` على نوع معدود يجب أن يغطي كل حالاته أو يحتوي `_` (`E2009` مع قائمة الناقص).
- الأنواع المعدودة **اسمية**، ويمكن أن تكون **تعاودية** (`enum Tree { Leaf, Node(Tree, Int, Tree) }`).
- المساواة `==` بنيوية؛ ولا ترتيب (`<`) بين قيمها بعد.
- القيمة تُطبع `Shape.Circle(1.0)` أو `Shape.Empty`.

## 4.8 القاموس `Map<K, V>`

```elomar
var stock = ["apple": 3, "pear": 0]
stock["kiwi"] = 5            // يضيف المفتاح أو يحدّثه
stock["apple"] += 1          // المفتاح يجب أن يكون موجودًا (R0011)
print(stock.get("plum"))     // None — القراءة الآمنة
for entry in stock {         // سجلات { key, value } بترتيب المفاتيح
    print(entry.key, entry.value)
}
var empty: Map<String, Int> = [:]
```

- المفاتيح `String` أو `Int` فقط (`E2018`). كل القيم من نوع واحد.
- `m[k]` تُرجع القيمة أو خطأ تنفيذ `R0011` إن غاب المفتاح؛ `m.get(k)` تُرجع `Option<V>`.
- دلالة القيمة كالقوائم: النسخ مستقل، والتعديل يتطلب `var`.
- القاموس ليس من أنواع JSON المقبولة في `ai.infer` بعد.

## 4.8.1 المجموعة `Set<T>`

```elomar
var tags = set(["ai", "web", "ai"])    // set(["ai", "web"])
tags.add("rust")
print(tags.contains("web"), tags.length, tags.remove("x"))   // true 3 false
let both = tags.intersection(set(["web", "mobile"]))          // set(["web"])
```

- تُنشأ بالدالة `set(list)` (لا صياغة حرفية خاصة)، والعناصر `String` أو `Int` فقط (`E2018`).
- `contains` و`add`* و`remove`* (تُرجع هل كان العنصر موجودًا) و`is_empty` و`to_list` و`union` و`intersection` و`difference` و`is_subset` و`.length`.
- `for x in s` بالترتيب، ودلالة القيمة كالقوائم، والمساواة لا تعتمد على ترتيب الإدخال.

## 4.9 المعامل `?`

```elomar
fn sum(a: String, b: String) -> Result<Int, String> {
    return Ok(parse(a)? + parse(b)?)
}
```

- على `Result<T, E>`: يُرجع `T`، أو يُنهي الدالة فورًا بـ `Err(e)`. يجب أن تُرجع الدالة `Result<_, E>` بنوع الخطأ نفسه (`E2001`).
- على `Option<T>`: يُرجع `T`، أو يُنهي الدالة بـ `None`. يجب أن تُرجع الدالة `Option<_>`.
- خارج دالة تُرجع النوع المناسب (ومنها `main`) `E2016`؛ وعلى قيمة ليست `Result` أو `Option` `E2017`.
- في `async fn` يعمل على نوع الإرجاع المعلن (ما داخل `Future`).

## 4.10 الملفات: `std.fs`

```elomar
use std.fs

match fs.read_text("data/students.json") {
    Ok(text) => print(text.length)
    Err(FsError.NotFound(_)) => print("no data yet")
    Err(e) => print(e.kind(), e.message())
}
```

| الدالة | النوع | الصلاحية |
|---|---|---|
| `fs.read_text(path)` | `Result<String, FsError>` (UTF-8؛ تُحذف علامة BOM) | `filesystem_read` |
| `fs.write_text(path, text)` | `Result<Unit, FsError>` (ينشئ الملف أو يستبدل محتواه) | `filesystem_write` |
| `fs.append_text(path, text)` | `Result<Unit, FsError>` (ينشئ الملف إن غاب) | `filesystem_write` |
| `fs.exists(path)` | `Result<Bool, FsError>` | `filesystem_read` |
| `fs.list_dir(path)` | `Result<List<String>, FsError>` (أسماء مرتبة) | `filesystem_read` |

- كل دوال `fs` تُحدث التأثير `fs`؛ والدالة `pub` التي تستخدمها يجب أن تصرّح بـ `!{fs}`.
- لا تُنشأ المجلدات تلقائيًا، ولا يوجد حذف ملفات بعد.
- كل فشل قيمة `Err(FsError...)`، لا خطأ تنفيذ.

### 4.10.1 صلاحيات الملفات

```toml
[permissions]
filesystem_read = ["data", "output"]   # ملفات أو مجلدات
filesystem_write = ["output"]          # القراءة لا تمنح الكتابة، والعكس
```

- **منع افتراضي:** دون هذه القوائم يُرفض كل وصول بـ `FsError.PolicyDenied` دون لمس الملف.
- **المجلد الأساس:** المسارات النسبية في البرنامج وفي القوائم تبدأ من مجلد `elomar.toml`، أو مجلد البرنامج إن لم يوجد الملف (لا من مجلد التشغيل الحالي).
- **لا هروب:** يُحوَّل المسار إلى شكله الحقيقي (canonical) قبل المقارنة، فلا يتجاوز `..` أو الروابط الرمزية المجلد المسموح.
- رسائل `NotFound` و`PermissionDenied` بصياغة ثابتة على كل أنظمة التشغيل.

## 4.11 JSON: `std.json`

```elomar
use std.json

type Student = { name: String, grades: List<Int> }

match json.parse<List<Student>>(text) {
    Ok(students) => print(students.length)
    Err(JsonError.TypeMismatch(m)) => print(m)   // "$[0].grades[1]: expected Int, found \"A\""
    Err(e) => print(e.message())
}
print(json.stringify(["a": 1]))   // {"a":1}
print(json.pretty(students))      // بمسافات بادئة
```

- `json.parse<T>(text) -> Result<T, JsonError>`: `InvalidJson` إن لم يكن النص JSON صالحًا، و`TypeMismatch` إن لم يطابق `T`
  (بالقواعد نفسها المستخدمة لإجابات `ai.infer`: الحقول الإلزامية، ورفض الحقول الزائدة، ومسار الحقل المخالف).
- `json.stringify(value)` و`json.pretty(value)` → `String`.
- الأنواع المقبولة: أنواع JSON في 4.6 إضافة إلى `Map<String, T>` (كائن JSON بمفاتيح حرة). المخالف `E2015`.
- `std.json` خالية من التأثيرات.
- السجلات الناتجة عن `json.parse` تُطبع حقولها بالترتيب الأبجدي، لأن أنواع السجلات بنيوية لا تحفظ ترتيب التعريف.

## 4.11.1 HTTP: `std.http`

```elomar
use std.http

fn main() {
    match http.get("https://api.example.com/todos/1") {
        Ok(resp) => print(resp.status, resp.body)
        Err(HttpError.PolicyDenied(m)) => print("not allowed:", m)
        Err(e) => print(e.kind(), e.message())
    }
    let r = http.post("https://api.example.com/todos", "{\"title\": \"x\"}")
}
```

| الدالة | النوع |
|---|---|
| `http.get(url)` | `Result<{ status: Int, body: String }, HttpError>` |
| `http.post(url, body[, content_type])` | النوع نفسه؛ `content_type` افتراضيًا `application/json` |

- كل استدعاء هو تأثير `net`، والمضيف يجب أن يكون في `[permissions] network_hosts` (`host` أو `host:port`، بلا أحرف بدل) وإلا `HttpError.PolicyDenied`.
- `https` إلزامي إلا لـ `localhost` و`127.0.0.1`؛ والعنوان غير الصالح `InvalidUrl`.
- لا تُتبع إعادة التوجيه (3xx) لأنها قد تقود إلى مضيف غير مسموح؛ يحصل البرنامج على الاستجابة نفسها.
- حالة الخطأ مثل 404 أو 500 **ليست** `Err`: هي استجابة بـ `status` الخاص بها. `Err` لفشل الاتصال فقط: `Timeout` (30 ثانية) و`Network` و`TooLarge` (أكبر من 10 MB أو ليست نصًا).
- لتحويل الجسم إلى نوع: `json.parse<Todo>(resp.body)`.

## 4.11.2 قواعد البيانات: `std.db` (SQLite)

```elomar
use std.db

type Student = { id: Int, name: String, grade: Int, email: Option<String> }

fn top(conn: Db) -> Result<List<Student>, DbError> {
    return db.query<Student>(conn, "SELECT * FROM students WHERE grade >= :min", { min: 60 })
}

fn main() {
    match db.open("data/school.db") {
        Ok(conn) => {
            print(db.execute(conn, "CREATE TABLE IF NOT EXISTS students (id INTEGER PRIMARY KEY, name TEXT, grade INTEGER, email TEXT)"))
            print(top(conn))
        }
        Err(e) => print(e.kind(), e.message())
    }
}
```

| الدالة | النوع |
|---|---|
| `db.open(path)` | `Result<Db, DbError>`؛ `":memory:"` قاعدة في الذاكرة |
| `db.execute(conn, sql[, params])` | `Result<Int, DbError>`: عدد الصفوف المتأثرة؛ بلا معاملات يُسمح بعدة جمل مفصولة بـ `;` |
| `db.query<T>(conn, sql[, params])` | `Result<List<T>, DbError>` |
| `db.query_one<T>(conn, sql[, params])` | `Result<Option<T>, DbError>`: أول صف إن وُجد |

- كل استدعاء هو تأثير `db`. ملف القاعدة يحتاج إذن `filesystem_write` لمجلده في `elomar.toml` (وإلا `DbError.PolicyDenied`)؛ و`":memory:"` مسموح دائمًا.
- **المعاملات** سجل تربط حقوله العناصر المسماة في SQL: `{ min: 60 }` يربط `:min`، فلا يُبنى SQL بدمج النصوص (لا حقن SQL). الحقول `Int` و`Float` و`String` و`Bool` و`Option` منها (`None` يصبح `NULL`)؛ والحقل غير المذكور في SQL `DbError.Sql`.
- **الصف** سجل تُطابق حقوله الأعمدة بالاسم، أو قيمة واحدة لعمود واحد (`db.query<Int>(conn, "SELECT COUNT(*) FROM t")`). العمود `NULL` يحتاج `Option<T>`، و`Bool` يُقرأ من 0/1، و`Float` يقبل الأعداد الصحيحة؛ وغير ذلك `DbError.TypeMismatch`.
- `DbError`: `PolicyDenied` و`Open` و`Sql` و`Constraint` (مثل `UNIQUE`) و`TypeMismatch`. نوع الصف أو المعاملات غير المناسب `E2021`، ونسيان نوع الصف `E2008`.
- قيمة `Db` مقبض: نسخها يشير إلى الاتصال نفسه. للمعاملات المتعددة: `db.execute(conn, "BEGIN")` ثم `"COMMIT"` أو `"ROLLBACK"`.

## 4.12 الوحدات متعددة الملفات

```text
school/
├── elomar.toml
└── src/
    ├── main.elo        ← البرنامج الرئيسي (يحتوي main)
    ├── students.elo    ← use app.students
    └── grading.elo     ← use app.grading
```

```elomar
// src/students.elo
use app.grading

pub type Student = { name: String, grades: List<Int> }

pub fn level(s: Student) -> grading.Level {
    return grading.level_for(average(s))
}

fn round1(x: Float) -> Float {      // خاصة: لا تُرى خارج هذا الملف
    return (x * 10).round() / 10
}
```

```elomar
// src/main.elo
use app.students
use app.grading

fn main() {
    let s: students.Student = { name: "Lina", grades: [95] }
    match students.level(s) {
        grading.Level.Excellent => print("excellent")
        _ => print("keep going")
    }
}
```

- `use app.a.b` يحمّل الملف `src/a/b.elo` من مجلد المشروع (مجلد `elomar.toml`، أو مجلد البرنامج إن لم يوجد)؛ وإن لم يوجد
  مجلد `src/` يُبحث بجانب `elomar.toml`. الاسم المختصر هو آخر جزء (`b`).
- الوصول بالاسم المؤهَّل: `students.level(s)` و`students.Student` و`grading.Level.Excellent` (في التعابير والأنماط والأنواع).
- أو باستيراد أسماء محددة: `use app.students.{Student, level}` ثم `level(s)` و`Student` مباشرة (ومنها الأنواع المعدودة: `use app.grading.{Level}` ثم `Level.Excellent`).
  الاسم المستورد الذي يتعارض مع عنصر في الملف نفسه أو مع استيراد آخر `E1002`، وغير الموجود `E1008`، وغير `pub` `E1012`.
- **الخصوصية افتراضية:** لا يُرى من ملف آخر إلا ما عُلّم بـ `pub` (`pub fn` و`pub type` و`pub enum` و`pub let`)؛ وإلا `E1012`
  مع تلميح بمكان إضافة `pub`. العنصر غير الموجود `E1008` مع اقتراح أقرب اسم.
- **الاستيرادات خاصة بكل ملف:** `use std.ai` أو `use app.x` في ملف لا تنطبق على ملف آخر.
- داخل الوحدة تُستخدم عناصرها بأسمائها المختصرة (`average(s)`)؛ ويمكن للوحدات أن يستورد بعضها بعضًا، ولو بشكل دائري.
- الدوال `pub` يجب أن تصرّح بتأثيراتها (`E3002`)، فلا يُفاجأ من يستخدم الوحدة باتصال شبكي أو كتابة ملف.
- رسائل الأخطاء تشير إلى الملف الصحيح، مع سطر `:::` للمواضع في ملفات أخرى.
- الوحدة غير الموجودة `E1011` مع المسار المتوقع.
- ثوابت `let` في الوحدات تُقيَّم قبل ثوابت الملف الرئيسي.

## 4.14 الدوال العامة (Generics)

```elomar
fn first<T>(xs: List<T>) -> Option<T> {
    if xs.is_empty() {
        return None
    }
    return Some(xs[0])
}

let n: Option<Int> = first([1, 2])        // T = Int، يُستنتج من المعاملات
let s = first<String>([])                 // أو يُكتب صراحة
```

- معاملات النوع تُكتب بعد اسم الدالة، وتُستنتج عند كل استدعاء من المعاملات أو من النوع المتوقع، أو تُعطى صراحة `f<Int>(...)`.
- داخل الدالة يكون `T` نوعًا مجهولًا: يُمرَّر ويُخزَّن ويُقارن بـ `==` ويُطبع، لكن لا تُطبَّق عليه عمليات حسابية ولا `<` (`E2005`)، ولا يُعامل كنوع محدد (`E2001`).
- عدد معاملات النوع الصريحة الخاطئ `E2002`؛ ومعاملات نوع لدالة غير عامة `E0103`.

### الأنواع والأنواع المعدودة العامة

```elomar
type Pair<A, B> = { left: A, right: B }

enum Tree<T> {
    Leaf
    Node(Tree<T>, T, Tree<T>)
}

fn swap<A, B>(p: Pair<A, B>) -> Pair<B, A> {
    return { left: p.right, right: p.left }
}

let t: Tree<Int> = Tree.Leaf
let names = Tree.Node(Tree.Leaf, "Lina", Tree.Leaf)   // Tree<String> يُستنتج
```

- `Pair<Int, String>` يعني السجل بعد التعويض، فالسجلات تبقى بنيوية.
- الأنواع المعدودة العامة اسمية: `Tree<Int>` و`Tree<String>` نوعان مختلفان، ونوع الحالة يُستنتج من قيمها أو من السياق، ونوع القيم في الأنماط يتبع نوع ما يُطابَق.
- عند الاستخدام يجب ذكر كل معاملات النوع (`Tree` وحده `E1006`). المعامل المكرر `E1002`.

## 4.15 الدوال المجهولة والدوال كقيم

```elomar
fn double(x: Int) -> Int {
    return x * 2
}

fn adder(n: Int) -> fn(Int) -> Int {
    return |x| x + n                       // تحتفظ بنسخة من n
}

let xs = [3, 1, 2]
xs.map(|x| x * 10)                         // [30, 10, 20]
xs.map(double)                             // الدالة المسماة قيمة أيضًا
xs.fold(0, |sum, x| sum + x)               // 6
let add5 = adder(5)
print(add5(1))                             // 6
let f = |a: Int, b: Int| {
    return a * b
}
```

- النوع `fn(Int, String) -> Bool` (و`fn()` تعني إرجاع `Unit`). أنواع المعاملات تُكتب، أو تُستنتج من السياق (`xs.map(|x| ...)`) أو من الجسم.
- الجسم تعبير أو كتلة؛ داخل الكتلة يُرجع `return` من الدالة المجهولة نفسها، و`?` كذلك.
- **الالتقاط بالقيمة:** تأخذ الدالة المجهولة نسخة من المتغيرات المرئية عند إنشائها؛ تغيير المتغير لاحقًا لا يؤثر فيها، وتعديل متغير ملتقط داخلها `E1013` (أرجع القيمة الجديدة أو استخدم `fold`).
- **التأثيرات:** تأثيرات جسم الدالة المجهولة (أو الدالة المسماة المستخدمة كقيمة) تُحسب على الدالة التي تُنشئها، كأن الجسم نُفّذ هناك؛ واستدعاء قيمة دالة لا يضيف تأثيرًا.
- دوال القوائم: `map` و`filter` و`any` و`all` و`find` و`count` و`fold(init, f)` و`sort_by(f)` (المفتاح `Int` أو `Float` أو `String`، والترتيب مستقر).
- الدوال المدمجة مثل `print` ليست قيمًا بعد (`E0103`)؛ اكتب `|x| print(x)`. ومساواة الدوال `==` بالهوية لا بالسلوك.
- استدعاء قيمة ليست دالة `E2004`، وعدد معاملات خاطئ `E2002`.

## 4.16 الواجهات `trait` والدوال التابعة `impl`

```elomar
trait Describe {
    fn describe(self) -> String
}

type Student = { name: String, grade: Int }

impl Describe for Student {
    fn describe(self) -> String {
        return self.name + ": " + to_string(self.grade)
    }
}

impl Describe for Int {
    fn describe(self) -> String {
        return "the number " + to_string(self)
    }
}

impl Student {                         // دوال تابعة بلا واجهة
    fn passed(self) -> Bool {
        return self.grade >= 60
    }
}

fn report<T: Describe>(items: List<T>) {   // أي نوع ينفذ Describe
    for x in items {
        print(x.describe())
    }
}

let s: Student = { name: "Lina", grade: 91 }
print(s.describe(), s.passed())
report([1, 2])
```

- الواجهة تذكر تواقيع الدوال فقط؛ أول معامل `self`، و`Self` يعني النوع المنفِّذ (`fn scaled(self, k: Float) -> Self`).
- `impl Trait for Type` يجب أن ينفذ كل دوال الواجهة بالتوقيع نفسه: الناقصة `E1014`، والمختلفة `E2001`، والزائدة `E1008` (ضعها في `impl Type` مستقلة).
- التنفيذ متاح لأي نوع: السجلات والأنواع المعدودة والأنواع الأساسية مثل `Int` و`List<Int>`. السجلات بنيوية، فـ `impl` على `Student` تنطبق على كل سجل بالحقول نفسها.
- القيد `<T: A + B>` يسمح باستدعاء دوال `A` و`B` على قيم `T`، وكل استدعاء للدالة العامة يجب أن يعطي نوعًا ينفذها (`E2019`)؛ وإن تعذر معرفة النوع `E2020`.
- التنفيذ المكرر لواجهة على النوع نفسه، أو دالتان تابعتان بالاسم نفسه للنوع نفسه، `E1002`.
- الدوال التابعة متاحة حيثما كان النوع متاحًا، ولها تأثيرات كأي دالة؛ وتأثيرات التنفيذ المستخدم عبر قيد تُحسب على من يستدعي الدالة العامة.
- غير مدعوم بعد: الدوال الافتراضية في الواجهة، و`impl` لنوع عام (`impl<T> ... for Tree<T>`)، واستخدام دالة ذات قيود كقيمة (`E0103`).

## 4.13 المشاريع والاختبارات

`elomar new myapp` ينشئ مشروعًا يعمل وتنجح اختباراته:

```text
myapp/
├── elomar.toml               ← [package] و[permissions] (كلها مغلقة افتراضيًا)
├── src/main.elo              ← elomar run src/main.elo
├── src/greeting.elo          ← وحدة: use app.greeting
└── tests/greeting_test.elo   ← elomar test
```

```elomar
// tests/greeting_test.elo
use app.greeting

fn test_greets_by_name() {
    assert_eq(greeting.greet("Lina"), "Hello, Lina!")
}
```

- **الاكتشاف:** كل ملف `.elo` داخل `tests/` (ومجلداته الفرعية) ملف اختبار؛ وكل دالة فيه اسمها يبدأ بـ `test_` وبلا معاملات اختبار.
- **النجاح والفشل:** يفشل الاختبار إن وقع خطأ تنفيذ (مثل فشل `assert` أو `assert_eq`) أو أرجع `Err(...)`؛ ويجوز أن يكون `async`.
- **العزل:** كل اختبار يعمل في مفسّر جديد بميزانية AI جديدة، ولا تُعرض مخرجاته إلا إن فشل.
- **الوصول للكود:** ملفات الاختبار تستخدم عناصر المشروع المعلّمة بـ `pub` عبر `use app.<module>`، وتنطبق عليها صلاحيات `elomar.toml` ونماذجه.
- `elomar test [PATH] [--filter TEXT]`: المسار مجلد المشروع (الافتراضي المجلد الحالي) أو ملف اختبار واحد؛ ورمز الخروج 1 عند أي فشل.
- `elomar new` لا يكتب فوق مجلد موجود غير فارغ.
- `elomar fmt [PATHS] [--check]` ينسّق الملفات بأسلوب واحد: 4 مسافات لكل مستوى، ومسافات حول العوامل الثنائية، و`f(x, y)` و`{ a: 1 }` و`0..10` و`!{ai, net}`، وحذف `;` في نهاية السطر، ودمج الأسطر الفارغة المتتالية. يحتفظ بالتعليقات وبفواصل الأسطر التي كتبها المؤلف، ويتحقق أن الناتج يُنتج الرموز نفسها قبل الكتابة.

## 5. التأثيرات (Effects)

التأثيرات المعروفة: `io` `fs` `net` `ai` `db` `py`. المنفذ حاليًا: `io` (`print` و`input`) و`ai` (`ai.infer`)
و`net` (`ai.infer` على نموذج سحابي، انظر 4.6.2) و`fs` (`std.fs`، انظر 4.10).

| القاعدة | الرمز |
|---|---|
| الدالة الخاصة بلا تصريح: تُستنتج تأثيراتها من جسمها ومن تستدعيه (نقطة ثابتة تشمل الاستدعاء الذاتي) | — |
| الدالة `pub` التي لها تأثيرات يجب أن تصرّح بها | `E3002` |
| أي دالة صرّحت بـ `!{...}` يجب أن يشمل تصريحها كل تأثيراتها الفعلية | `E3001` |
| اسم تأثير غير معروف | `E3003` |
| قيمة ثابت الملف يجب أن تكون بلا تأثيرات | `E3004` |
| `main` لا تحتاج تصريحًا | — |

---

## 6. التنفيذ

- يبدأ البرنامج بتقييم ثوابت الملف بالترتيب ثم استدعاء `main()`، التي لا تأخذ معاملات ولا تُرجع قيمة (`E2011`).
- عمق الاستدعاء الأقصى 3000؛ بعده `R0005`.
- `print(a, b, ...)` يطبع القيم مفصولة بمسافة ثم سطرًا جديدًا. النصوص تُطبع بلا علامات تنصيص في المستوى الأعلى وبها داخل القوائم والسجلات.

---

## 7. الـ Prelude

### 7.1 الدوال

| الدالة | النوع | تأثير |
|---|---|---|
| `print(x, ...)` | أي عدد من القيم → `Unit` | `io` |
| `input()` / `input(prompt)` | `String` → `String` (سطر بلا نهايته؛ نص فارغ عند نهاية الإدخال) | `io` |
| `to_float(n)` | `Int\|Float` → `Float` | |
| `to_int(x)` | `Int\|Float` → `Int` (يحذف الكسر؛ `R0007` لـ NaN/∞) | |
| `to_string(x)` | أي قيمة → `String` | |
| `parse_int(s)` / `parse_float(s)` | `String` → `Option<Int>` / `Option<Float>` | |
| `assert(cond)` / `assert(cond, msg)` | `Bool`, `String` → `Unit` (`R0004`) | |
| `assert_eq(actual, expected)` | قيمتان من النوع نفسه → `Unit` (`R0004` مع عرض القيمتين) | |
| `min(a, b)` / `max(a, b)` | عددان → `Int` إن كانا صحيحين وإلا `Float` | |
| `set(list)` | `List<T>` → `Set<T>` (العناصر `String` أو `Int`) | |
| `abs(x)` | `Int\|Float` → النوع نفسه | |

### 7.2 الخصائص والدوال التابعة

| النوع | الأعضاء |
|---|---|
| كل الأنواع | `.to_string()` |
| `List<T>` | `.length` · `push(T)`* · `pop() -> Option<T>`* · `insert(Int, T)`* · `remove(Int) -> T`* · `contains(T)` · `index_of(T) -> Option<Int>` · `is_empty()` · `first()` / `last() -> Option<T>` · `reverse()` · `sorted()` (أعداد أو نصوص) · `slice(start, end)` · `sum()` (أعداد) · `join(sep)` (`List<String>`) · `map(fn(T) -> U)` · `filter` / `any` / `all` / `count(fn(T) -> Bool)` · `find(fn(T) -> Bool) -> Option<T>` · `fold(U, fn(U, T) -> U)` · `sort_by(fn(T) -> K)` |
| `String` | `.length` (حروف مرئية) · `upper()` · `lower()` · `trim()` · `contains(s)` · `starts_with(s)` · `ends_with(s)` · `split(sep) -> List<String>` · `chars() -> List<String>` · `replace(a, b)` · `repeat(n)` · `parse_int()` · `parse_float()` · `is_empty()` |
| `Int` | `to_float()` · `abs()` |
| `Float` | `to_int()` · `round()` · `floor()` · `ceil()` (→ `Int`) · `abs()` · `sqrt()` |
| `Option<T>` | `is_some()` · `is_none()` · `unwrap_or(T) -> T` |
| `Result<T,E>` | `is_ok()` · `is_err()` · `unwrap_or(T) -> T` |
| `Map<K,V>` | `.length` · `get(K) -> Option<V>` · `contains_key(K)` · `remove(K) -> Option<V>`* · `keys() -> List<K>` · `values() -> List<V>` · `is_empty()` |
| `Set<T>` | `.length` · `contains(T)` · `add(T)`* · `remove(T) -> Bool`* · `is_empty()` · `to_list()` · `union(Set<T>)` · `intersection(Set<T>)` · `difference(Set<T>)` · `is_subset(Set<T>) -> Bool` |
| `AIError` · `FsError` · `JsonError` | `kind() -> String` (اسم الحالة) · `message() -> String` |

\* يعدّل القائمة، ويتطلب أن تكون في متغير `var`.

---

## 8. رموز التشخيص

كل رسالة متاحة بالعربية والإنجليزية (`elomar --lang ar ...` أو `ELOMAR_LANG=ar`).

### أخطاء معجمية ونحوية (`E0xxx`)

| الرمز | المعنى |
|---|---|
| `E0001` | حرف غير متوقع |
| `E0002` | نص غير مغلق |
| `E0003` | تسلسل هروب غير صالح |
| `E0004` | تعليق متعدد الأسطر غير مغلق |
| `E0005` | عدد صحيح أو مدة أكبر من 64 بت |
| `E0006` | لاحقة عدد غير صالحة (المسموح `ms` و`s` و`m` و`h` بعد عدد صحيح) |
| `E0101` | رمز غير متوقع (المتوقع كذا) |
| `E0102` | المتوقع تعبير |
| `E0103` | ميزة غير مدعومة بعد (معاملات النوع لدالة غير عامة، الدوال المدمجة كقيم، الدالة ذات القيود كقيمة) |
| `E0104` | هدف إسناد غير صالح |
| `E0105` | `var` على مستوى الملف |
| `E0107` | المتوقع نمط |
| `E0108` | المتوقع نوع |

### أخطاء الأسماء (`E1xxx`)

| الرمز | المعنى |
|---|---|
| `E1001` | اسم غير معروف (مع اقتراح أقرب اسم) |
| `E1002` | تعريف مكرر، أو اسم محجوز/مدمج |
| `E1003` | تعديل قيمة غير معرّفة بـ `var` |
| `E1004` | `break`/`continue` خارج حلقة |
| `E1005` | لا توجد `main` |
| `E1006` | نوع غير معروف، أو اسم مستعار دائري، أو عدد معاملات نوع خاطئ |
| `E1007` | حقل غير موجود |
| `E1008` | دالة تابعة غير موجودة، أو دالة غير موجودة في وحدة |
| `E1009` | وحدة غير معروفة في `use` |
| `E1010` | حالة غير موجودة في نوع معدود |
| `E1011` | وحدة `use app.*` غير موجودة |
| `E1012` | عنصر غير `pub` مستخدم من ملف آخر |
| `E1013` | تعديل متغير ملتقط داخل دالة مجهولة |
| `E1014` | `impl` لواجهة ينقصها دالة من دوال الواجهة |

### أخطاء الأنواع (`E2xxx`)

| الرمز | المعنى |
|---|---|
| `E2001` | عدم توافق الأنواع |
| `E2002` | عدد معاملات خاطئ |
| `E2003` | الشرط ليس `Bool` |
| `E2004` | القيمة ليست دالة (أو `length` استُدعيت كدالة) |
| `E2005` | العامل لا ينطبق على هذه الأنواع |
| `E2006` | القيمة لا تُفهرس بـ `[]` |
| `E2007` | مسار لا يُرجع قيمة |
| `E2008` | تعذّر استنتاج النوع |
| `E2009` | `match` غير شامل |
| `E2010` | خطأ في المعاملات المسماة |
| `E2011` | توقيع `main` غير صحيح |
| `E2012` | القيمة لا يُتكرر عليها |
| `E2013` | `await` خارج `async fn` |
| `E2014` | `await` على قيمة ليست `Future` |
| `E2015` | نوع ليس له تمثيل JSON في `ai.infer` (ناتجًا أو مدخلًا) |
| `E2016` | `?` في دالة لا تُرجع `Result`/`Option` المناسب |
| `E2017` | `?` على قيمة ليست `Result` ولا `Option` |
| `E2018` | مفتاح `Map` ليس `String` ولا `Int` |
| `E2019` | نوع لا ينفذ الواجهة المطلوبة في قيد `<T: Trait>` |
| `E2020` | تعذر معرفة النوع المطلوب لقيد واجهة في استدعاء |
| `E2021` | نوع صف أو معاملات غير مناسب لـ `std.db` |

### أخطاء التأثيرات (`E3xxx`)

| الرمز | المعنى |
|---|---|
| `E3001` | تأثير غير مصرّح به |
| `E3002` | دالة عامة بلا تصريح تأثيرات |
| `E3003` | تأثير غير معروف |
| `E3004` | ثابت ملف ذو تأثيرات |

### تحذيرات (`Wxxxx`)

| الرمز | المعنى |
|---|---|
| `W1001` | حجب متغير من نطاق خارجي |
| `W1002` | قيمة محسوبة لا تُستخدم (مع تلميح `==` بدل `=`) |
| `W1003` | `Future` لا يُنتظر، فلن يُنفَّذ |

### أخطاء التنفيذ (`Rxxxx`)

| الرمز | المعنى |
|---|---|
| `R0001` | قسمة على صفر |
| `R0002` | تجاوز سعة `Int` |
| `R0003` | فهرس خارج الحدود |
| `R0004` | فشل `assert` |
| `R0005` | استدعاءات متداخلة كثيرة جدًا |
| `R0006` | معامل غير صالح (جذر عدد سالب، تكرار نص بعدد سالب) |
| `R0007` | تحويل عشري إلى `Int` غير ممكن |
| `R0008` | استخدام ثابت قبل تهيئته |
| `R0009` | خطأ داخلي في المفسّر (يُبلَّغ عنه كعلة) |
| `R0010` | انتظار `Future` أثناء تنفيذه |
| `R0011` | مفتاح غير موجود في `Map` عند `m[k]` |

---

## 9. غير المنفذ بعد (مخطط)

مزودات سحابية أخرى، وتطبيق `[permissions]` على الملفات والعمليات الفرعية (تُتجاهل حاليًا) ·
`ai.generate` و`ai.stream` و`ai.embed` · حذف الملفات وإنشاء المجلدات والدوال التابعة للأنواع المعدودة ·
الدوال الافتراضية في الواجهات و`impl` للأنواع العامة والمهام المتوازية والإلغاء (v0.3) · الاستيفاء النصي.
