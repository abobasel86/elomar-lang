# Elomar-Lang — القواعد النحوية والدلالية للنواة (المراحل 1 و2 و3)

> **هذا الملف هو المرجع الملزم** لما ينفذه المفسّر الحالي. عند أي تعارض مع
> `Elomar-Lang_Master_Specification_AR.md` يُعتمد هذا الملف، ويُحدَّث مع كل
> تغيير في `crates/`. كل قاعدة هنا لها اختبار في `crates/*/src/**` أو
> `tests/fail/` أو `examples/`.

الإصدار: `1.0.0` · الحالة: **مستقرة**. يحكمها [وعد الاستقرار](stability.md): البرنامج الذي يعمل
مع 1.0 يبقى يعمل مع كل إصدار 1.x.

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
| نصوص | `"..."` في سطر واحد، والهروب: `\n \t \r \0 \" \\ \{ \} \u{XXXX}` |
| نصوص خام | `r"\d+"` و`r#"قال "نعم""#`: بلا هروب ولا إدراج، فكل حرف يبقى كما هو. مناسبة للتعابير النمطية ومسارات Windows. بـ `#` يمكن أن يحتوي النص على `"` |
| إدراج القيم | `"مرحبا {name}، عمرك {age + 1}"`: أي تعبير بين `{` و`}` يُحسب ويُعرض كما يعرضه `print`، والنتيجة `String`. لقوس حرفي اكتب `\{` (مثلًا JSON: `"\{\"a\": 1}"`). داخل التعبير يمكن استعمال نصوص وأقواس (`"{m["k"]}"`). لا يُقبل نمط `match` بنص فيه إدراج |
| كلمات محجوزة | `let var fn return if else while for in break continue true false type enum match use pub async await trait impl self as div`، وكتابة كل منها بالعربية (§1.0.1) |
| أسماء الـ Prelude | `Some None Ok Err` ودوال القسم 7 — ليست كلمات محجوزة لكن لا يجوز إعادة تعريفها |

### 1.0.1 الكلمات المحجوزة بالعربية (منذ 1.13)

لكل كلمة محجوزة كتابة عربية أيضًا. الكتابتان رمز واحد، ويجوز المزج بينهما في
الملف نفسه، وكلتاهما محجوزة فلا تصلح اسمًا.

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

تُقبل الكلمة العربية بهمزة أو دونها على الألف (`أرجع` أو `ارجع`، `إذا` أو `اذا`)،
وبـ `ى` مكان `ي` في آخرها (`فى`)، وبـ `ه` مكان `ة` (`داله`)، وتُتجاهل الحركات
والشدة والتطويل (`نفّذ`، `دالـــة`). ويجوز أن يبدأ البرنامج من `رئيسية` بدل `main`
إن لم تكن فيه `main`. ويكتب `elomar fmt --keywords ar` كل الكلمات بالعربية،
و`--keywords en` بالإنجليزية؛ ودون الخيار تبقى كل كلمة كما كُتبت. ومنذ 1.16 يكتب
بتلك اللغة أيضًا الأسماء المدمجة في §1.0.2 و§1.0.3، إلا حيث يعني الاسم الآخر شيئًا
مختلفًا (لأن اسمًا من البرنامج يحجبه هناك).

### 1.0.2 أسماء عربية للدوال والأنواع المدمجة (منذ 1.14)

للدوال المدمجة والمُنشئات والأنواع الأساسية أسماء عربية أيضًا. وهي، بخلاف الكلمات المحجوزة،
**غير محجوزة**: يحوّل المحلل كل اسم منها إلى اسمه الإنجليزي، إلا حيث يظهر للبرنامج شيء يحمل
الاسم نفسه (دالة أو نوع أو ثابت أو معامل أو متغير أو معامل دالة مجهولة أو اسم في نمط)، فيحجب
الاسمَ المدمج هناك كما يحجب `print`. وتُقبل الأسماء كما تُقبل الكلمات العربية (لا تهم الهمزة ولا `ى`
ولا `ة` ولا الحركات ولا التطويل). وتذكر الرسائل الأسماء الإنجليزية.

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
| `HttpRequest` | `طلب_ويب` | `HttpResponse` | `رد_ويب` |
| `Db` | `قاعدة_بيانات` | `Range` | `مدى` |
| `Future` | `مستقبل` | `Unit` | `فراغ` |
| `AIError` | `خطأ_ذكاء` | `FsError` | `خطأ_ملفات` |
| `JsonError` | `خطأ_جيسون` | `HttpError` | `خطأ_ويب` |
| `DbError` | `خطأ_بيانات` | `PyError` | `خطأ_بايثون` |
| `McpError` | `خطأ_أدوات` | `ProcessError` | `خطأ_عملية` |

### 1.0.3 أسماء عربية للدوال التابعة (منذ 1.15)

للدوال التابعة للأنواع المدمجة وللخاصية `length` أسماء عربية أيضًا (`.طول` هي `.length`، و`.أضف` هي
`.push` على القائمة و`.add` على المجموعة)، وجدولها كاملًا في
[المرجع](https://abobasel86.github.io/elomar-lang/ar/12-reference.html#arabic-methods). وهي غير محجوزة،
ويحلّها مدقق الأنواع من نوع القيمة: يأتي أولًا حقل السجل الذي يحمل الاسم نفسه ثم الدالة المعرّفة في `impl`،
وإلا صار الاسم العربي الدالةَ الإنجليزية لذلك النوع. وتُقبل كما تُقبل الكلمات العربية، وتذكر الرسائل الأسماء
الإنجليزية.

### 1.0.4 أسماء عربية للوحدات القياسية (منذ 1.17)

للوحدات القياسية ودوالها وثوابتها وللصلاحيات أسماء عربية (`use std.ملفات` هي `use std.fs`، و`ملفات.اقرأ_نص` هي
`fs.read_text`، و`!{ملفات}` هي `!{fs}`)، وجدولها كاملًا في
[المرجع](https://abobasel86.github.io/elomar-lang/ar/12-reference.html#arabic-modules).
ويحلّها المحلل: `use std.X` بأي من اللغتين يستورد الوحدة، وبعدها تسمّيها أي الكتابتين (إلا إن حجبها اسم من البرنامج)،
والدالة بعد اسم وحدة مستوردة يُبحث عنها في جدول تلك الوحدة. وتُقبل كما تُقبل الكلمات العربية.

### 1.0.5 `std` والأخطاء وسجلات الوحدات (منذ 1.18)

يجوز أن تُكتب `std` هكذا: `قياسي`. ولبقية الأنواع المدمجة أسماء عربية (في §1.0.2)، وكذلك لحالات الأخطاء المدمجة
(`خطأ_ملفات.غير_موجود` هي `FsError.NotFound`)، ويحلّها المحلل بعد اسم نوع الخطأ. ولحقول السجلات التي ترجعها
الوحدات أسماء عربية يحلّها المدقق على تلك السجلات وحدها (بحسب حقولها)، عند قراءة الحقل وعند كتابة سجل من ذلك
النوع. الجداول في [المرجع](https://abobasel86.github.io/elomar-lang/ar/12-reference.html#arabic-errors).

### 1.1 نهاية السطر

يُبقي الـ Lexer نهاية السطر **فاصلَ جمل** فقط إذا تحققت الشروط الثلاثة:

1. آخر رمز قبلها: معرّف، أو قيمة حرفية، أو `true`/`false`/`self`، أو `)` `]` `}`، أو `return`/`break`/`continue`.
2. لسنا داخل `( )` أو `[ ]` مفتوحة.
3. أول رمز بعدها ليس `.` ولا `else` ولا نهاية الملف، ولا عاملًا ثنائيًا لا يبدأ به تعبير: `||` `&&` `+` `*` `/` `%` `div` `==` `!=` `<` `<=` `>` `>=` `..` `..=`. (`-` و`!` يمكن أن يبدأ بهما تعبير، فيبدآن جملة جديدة.)

والأسطر الفارغة المتتالية تُدمج. الفاصلة المنقوطة `;` فاصل صريح اختياري.

```elomar
let total = price *
    quantity              // يكمل: السطر السابق انتهى بـ *
let names = list
    .sorted()             // يكمل: السطر يبدأ بـ .
let ok = a > 0
    || b > 0              // يكمل: السطر يبدأ بـ ||
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
| `String` | نص Unicode | `+` للدمج، و`"{x}"` للإدراج؛ لا فهرسة بالأرقام |
| `Unit` | لا قيمة | نوع الدالة بلا `->` |
| `List<T>` | `[1, 2]` | عناصر متجانسة؛ `+` للدمج |
| `(A, B, ...)` | `(1, "a")` | صف (tuple) من قيمتين أو أكثر؛ `t.0` و`t.1` للوصول، و`let (a, b) = t` للتفريق، و`(x, _)` كنمط. يُقارن ويُرتَّب عنصرًا عنصرًا |
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
- الدوال المسماة قيم يمكن تمريرها (§4.15)، وكذلك الدوال المدمجة مثل `print` و`to_string` (منذ 1.9).

### 4.3 `match`

- فرع يبدأ بـ `return` أو `break` أو `continue` اختصار لكتلة: `None => return 0` تعني `None => { return 0 }`.
- فحص التغطية يشمل الأنماط المتداخلة: `Ok(Some(x))` و`Ok(None)` و`Err(e)` معًا تغطي `Result<Option<T>, E>`، ورسالة `E2009` تذكر الحالة الناقصة بدقة (مثل `Ok(None)`).
- الأنماط: `_`، اسم ربط، `Some(p)`/`Ok(p)`/`Err(p)`/`None`، صفوف `(p, q)`، أعداد ونصوص و`true`/`false`، وبدائل بـ `|` (`"Fri" | "Sat" =>`، ولا تربط أسماء: `E2024`)، ونطاقات أعداد صحيحة `90..=100` و`0..10` (النطاق الفارغ `E2025`).
- **الشمول إلزامي** (`E2009`): `Option` يحتاج `Some(_)` و`None`، و`Result` يحتاج `Ok(_)` و`Err(_)`،
  و`Bool` يحتاج `true` و`false`، وغيرها يحتاج `_` أو اسم ربط.
- كتعبير: كل الفروع بنوع واحد. كجملة: لا يُشترط توافق أنواع الفروع.

### 4.3.1 الصفوف (Tuples) والتفريق

```elomar
fn min_max(xs: List<Int>) -> (Int, Int) {
    return (xs.min().unwrap_or(0), xs.max().unwrap_or(0))
}

fn main() {
    let (lo, hi) = min_max([4, 9, 2])          // التفريق؛ `_` لتجاهل عنصر
    let pair = (1, "one")
    print(pair.0, pair.1)
    for (name, score) in ["Lina": 91] {        // على قاموس: المفتاح والقيمة
        print(name, score)
    }
    for (i, w) in [(1, "a"), (2, "b")] {       // على قائمة صفوف: العناصر
        print(i, w)
    }
}
```

`(x)` بقوس واحد ليس صفًا بل تجميع. الصفوف لا تعبر إلى JSON ولا إلى إجابات الذكاء الاصطناعي؛ استخدم سجلًا هناك.

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
- `all(List<Future<T>>) -> Future<List<T>>`: `await all([...])` ينتظر كل العمليات ويعيد نتائجها **بالترتيب نفسه**.
  استدعاءات `ai.infer` التي تُمرَّر مباشرة (مثل `await all(texts.map(|t| ai.infer<Ticket>(...)))`) تُرسل **معًا في الوقت نفسه**
  عند مزودي OpenAI وAnthropic، بعد فحص الصلاحيات وحد الاستدعاءات لكل منها بالترتيب.
  واستدعاءات `async fn` **المعزولة** المتجاورة في القائمة تُنفذ **معًا على أنوية المعالج** (منذ 1.2)، والنتيجة هي نفسها
  لو نُفذت واحدة بعد أخرى. الاستدعاء معزول إن لم يطبع ولم يقرأ إدخالًا، ولم يستخدم ملفات أو قاعدة بيانات أو عمليات أو
  Python أو MCP أو نموذج ذكاء اصطناعي أو أرقامًا عشوائية، هو وكل دالة قد يستدعيها، ولم تحمل معاملاته دوال غير معزولة
  أو عمليات `Future`؛ ويجوز له الحساب وطلبات `std.http`. إن فشل أحدها يُعاد خطأ أولها في القائمة وتُوقف التي بعده،
  و`timeout` حول `all` تسري عليها. غير ذلك يُنفذ واحدًا بعد آخر، وكذلك كل شيء في ساحة التجربة بالمتصفح وتحت المنقّح.
  `ELOMAR_PARALLEL=0` يُلغي التنفيذ المتوازي.
- `xs.par_map(f)` (منذ 1.3) تعطي دائمًا ما تعطيه `xs.map(f)`، لكنها تقسّم القائمة أجزاءً متتالية وتنفذها **على أنوية المعالج**
  إن كانت `f` دالة معزولة بالشروط نفسها (دالة مسماة غير `async`، أو دالة مجهولة يحقق جسمها الشروط ولا تحمل متغيراتها
  دوال غير معزولة)، وإلا نُفذت مثل `map`. إن فشل عنصر يُعاد خطأ أول عنصر يفشل وتُوقف الأجزاء التي بعده.
- `xs.par_filter(f)` (منذ 1.4) تعطي دائمًا ما تعطيه `xs.filter(f)`، وتُنفذ `f` على الأنوية بالشروط نفسها.
- `xs.par_fold(init, f)` (منذ 1.4) حيث `f: fn(T, T) -> T` ونوع `init` هو نوع العناصر: تقسّم القائمة إلى 64 جزءًا متتاليًا
  على الأكثر (أو جزء لكل عنصر في القوائم الأقصر)، فتطوي أولها بدءًا من `init`، وكل جزء بعده بدءًا من أول عناصره،
  ثم تطوي نتائج الأجزاء بالترتيب بـ `f` نفسها. يعتمد التقسيم على طول القائمة وحده، فالنتيجة واحدة على كل جهاز ومع
  التوازي وبدونه، وتساوي `xs.fold(init, f)` إن كانت `f` تجميعية (`f(f(a, b), c) == f(a, f(b, c))`) مثل الجمع
  و`max` ووصل النصوص. تُطوى الأجزاء على الأنوية بشروط `par_map`، والخطأ خطأ أول جزء يفشل.
- `par_count(f)` و`par_sort_by(f)` (منذ 1.5) تعطيان دائمًا ما تعطيه `count(f)` و`sort_by(f)` (الترتيب مستقر)، وتُحسب
  `f` على الأنوية بشروط `par_map`.
- `par_any(f)` و`par_all(f)` و`par_find(f)` (منذ 1.5) تعطي دائمًا ما تعطيه `any` و`all` و`find`: تبحث الأجزاء معًا عن
  أول عنصر تعطي له `f` القيمة `true` (أو `false` في `par_all`)، وتُوقف الأجزاء التي بعد أول جزء يجده. ما بعد ذلك العنصر
  لا يغيّر النتيجة: لا يُعاد خطؤه، ولا تُنتظر حلقة لا تنتهي فيه، كما لو توقفت `find` عنده.
- `par_flat_map(f)` و`par_group_by(f)` و`par_sum_by(f)` و`par_min_by(f)` و`par_max_by(f)` (منذ 1.6) تعطي دائمًا ما تعطيه
  نظيراتها بلا `par_`: تُحسب `f` لكل العناصر على الأنوية بشروط `par_map`، ثم يُكمَل العمل بالترتيب. إن فشل عنصر في
  `par_sum_by` يُعاد المرور على العناصر بالترتيب، فيكون الخطأ خطأ `sum_by` نفسه (فيض الجمع قبل ذلك العنصر إن وقع).
- `timeout(Future<T>, Duration) -> Future<Option<T>>`: `await timeout(f, 2s)` تعطي `Some(ناتج)`، أو `None` إن لم تنتهِ
  العملية في الوقت المحدد؛ عندها **تُوقف** عند جملتها التالية (أو تكرار الحلقة التالي)، وينتهي `time.sleep` بانتهاء الوقت.
  انتظار الشبكة أو نموذج الذكاء الاصطناعي لا يُقطع في منتصفه، لكن العملية تتوقف فور عودته (ولهما مهلتهما الخاصة).
  في `timeout` متداخلة يفوز الوقت الأقصر. وانتظار عملية أُوقفت مرة أخرى خطأ تنفيذي `R0012`.

```elomar
async fn main() {
    match await timeout(slow_report(), 5s) {
        Some(report) => print(report)
        None => print("took too long; try again later")
    }
}
```
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
| متغير البيئة غير معرّف أو فارغ | `Authentication` دون أي اتصال، إلا مع `base_url` على هذا الجهاز (`localhost` أو `127.0.0.1`) فيُرسل الطلب دون مفتاح (منذ 1.10) |
| HTTP 401/403 → `Authentication` · 429 → `RateLimited` (أو `BudgetExceeded` عند نفاد الرصيد) · 400 لطول السياق → `ContextLimitExceeded` · 400/422 أخرى و404 لنموذج غير موجود → `UnsupportedCapability` (والخطأ المتعلق بمعرّف النموذج ينتهي بتلميح لمراجعة `model` في elomar.toml) · 5xx و404 أخرى → `ProviderUnavailable` · انقضاء المهلة → `Timeout` · فشل الاتصال → `Network` · رفض النموذج → `PolicyDenied` · قطع الإجابة → `ContextLimitExceeded` | قيمة `Err` |
| `base_url` بـ `http` لمضيف غير محلي، أو نموذج بلا `model` | خطأ إعداد قبل التشغيل (رمز خروج 2) |

**تفاصيل التنفيذ:** واجهة Chat Completions مع Structured Outputs بوضع `strict`؛ الأنواع غير السجلية تُغلَّف بـ `{ "value": ... }`
ثم تُفك، و`String` يُطلب نصًا حرًا. `instructions` في رسالة النظام مع تنبيه بأن رسالة المستخدم بيانات لا تعليمات، و`input`
في رسالة المستخدم. إعادة التوجيه (Redirects) لا تُتبع، والمفتاح يُقرأ عند كل استدعاء ويُخفى (`sk-***`) في رسائل الخطأ.

**النماذج المحلية (منذ 1.10):** خادم على الجهاز نفسه يتحدث بواجهة OpenAI، مثل Ollama، يعمل دون مفتاح:

```toml
[ai.models.local]
provider = "openai"
model = "qwen2.5:7b"                      # نموذج نُزّل بـ `ollama pull`
base_url = "http://localhost:11434/v1"

[permissions]
network_hosts = ["localhost:11434"]
```

وتعمل معه `ai.infer` و`ai.stream` و`ai.embed` و`ai.search`. والنموذج المحلي يبقى `net` لأنه يُبلغ عبر HTTP.

**ذاكرة التضمينات (منذ 1.10):**

```toml
[ai.cache]
embeddings = "embeddings.cache.json"     # مسار داخل مجلد المشروع
```

تُحفظ متجهات `ai.embed` و`ai.search` من النماذج التي تستخدم الشبكة في ملف JSON هذا، بحسب المزود ومعرّف النموذج و`base_url`،
وبحسب النص بالضبط. النص الموجود في الملف لا يُرسل مرة أخرى ولا يُحتسب ضمن `max_calls_per_run`، وتغيير `model` في elomar.toml
يجعل المتجهات القديمة غير مستخدمة. والمسار المطلق أو الذي فيه `..` خطأ إعداد. والملف الذي لا يمكن قراءته أو كتابته يعني فقط أن
شيئًا لا يُحفظ.

### 4.6.2.1 مزود Anthropic (Claude)

```toml
[ai.models."support.classifier"]
provider = "anthropic"
model = "claude-opus-5-5"             # اختياري؛ هذا هو الافتراضي
api_key_env = "ANTHROPIC_API_KEY"     # الافتراضي ANTHROPIC_API_KEY

[permissions]
network_hosts = ["api.anthropic.com"]
```

القواعد نفسها كما في OpenAI (الصلاحيات، `net`، إخفاء المفتاح، تحويل الأخطاء). الفروق:

- واجهة Messages (`POST /v1/messages`) بإصدار `2026-10-21`. الأنواع تُطلب بـ `output_config.format` (JSON Schema)، والأدوات بـ `strict: true` (أدوات MCP بمخطط الخادم دون `strict`).
- في حلقة الأدوات يُعاد محتوى المساعد كما وصل تمامًا (ومنه كتل التفكير)، وتُجمع نتائج الخطوة الواحدة في رسالة واحدة.
- عند رفض النموذج (`stop_reason: "refusal"`) يُرجع `PolicyDenied`. للنماذج `claude-opus-5-5` و`claude-opus-5` و`claude-sonnet-5-5` و`claude-fable-5-1` و`claude-fable-5-2` يُفعَّل التحويل التلقائي إلى نموذج بديل عند الرفض (`fallbacks: "default"`).
- HTTP 402 → `BudgetExceeded` · 529 (ضغط) → `ProviderUnavailable` · 400 مع «prompt is too long» → `ContextLimitExceeded`.

### 4.6.3 الأدوات: `ai.run`

```elomar
fn find_student(name: String) -> Option<Student> { ... }
fn class_average() -> Float { ... }

async fn ask(q: String) -> Result<Answer, AIError> !{ai} {
    return await ai.run<Answer>(
        model: "assistant",
        instructions: "Answer questions about the class. Use the tools for facts.",
        input: q,
        tools: [find_student, class_average],   // ما يُسمح للنموذج باستدعائه فقط
        max_steps: 5,                           // اختياري، الافتراضي 5
        timeout: 20s                            // اختياري، لكل خطوة
    )
}
```

- يسأل المحرك النموذج؛ فإن طلب استدعاء أداة نفّذ الدالة وأعاد ناتجها إليه، ويتكرر ذلك حتى يجيب. الإجابة النهائية يُتحقق منها مقابل `T` كما في `ai.infer`.
- الأدوات أسماء دوال عادية مذكورة صراحة في `tools` (لا دوال مجهولة ولا عامة ولا `async`)، ومعاملاتها ونتيجتها أنواع JSON؛ ويرى النموذج اسم الدالة وأسماء معاملاتها وأنواعها. أسماء الأدوات حروف لاتينية وأرقام و`_` فقط (قيد واجهات النماذج).
- **التأثيرات:** تأثيرات الأدوات تُحسب على الدالة التي تستدعي `ai.run` (أداة تطبع تعني `io`)، إضافة إلى `ai` (و`net` للنماذج السحابية).
- **الحدود:** كل خطوة استدعاء يُحسب في `max_calls_per_run`؛ وتجاوز `max_steps` يعطي `AIError.BudgetExceeded`. معاملات خاطئة من النموذج أو أداة غير موجودة لا تُنفَّذ، بل يُعاد إليه خطأ ليصحح.
- النموذج الوهمي `mock` يتبع نصًا مكتوبًا: القواعد المطابقة لـ `input` تُستخدم بالترتيب، خطوة لكل قاعدة؛ القاعدة `tool = "find_student"` و`arguments = { name = "Lina" }` تطلب أداة، والقاعدة `response` تجيب، و`{{last_tool_result}}` فيها يُستبدل بآخر ناتج. مزود OpenAI يستخدم Tool Calling بالوضع الصارم.
- أخطاء الأدوات `E2022`: ليست قائمة أسماء، أو دالة مجهولة أو عامة أو `async`، أو اسم غير لاتيني، أو أداة مكررة؛ ونوع معامل بلا JSON `E2015`.

### 4.6.4 الوكلاء وMCP: `agent.run` و`std.mcp`

```toml
# elomar.toml: خوادم MCP المسموح بتشغيلها؛ ذكر الخادم هنا هو الإذن
[mcp.servers.notes]
command = "python"                 # أو npx أو أي برنامج
args = ["mcp/notes_server.py"]
env = { LEVEL = "1" }              # اختياري
timeout_ms = 30000                 # اختياري، لكل طلب
```

```elomar
use std.agent
use std.mcp

// استدعاء مباشر لأدوات خادم MCP (التأثير mcp)
let tools = mcp.tools("notes")                          // Result<List<{ name, description }>, McpError>
let r = mcp.call("notes", "add_note", { text: "hi" })   // Result<String, McpError>

// وكيل: نموذج يستخدم دوالك وأدوات خوادم MCP، بموافقة قبل الأفعال الحساسة
async fn assist(q: String) -> Result<Answer, AIError> !{ai, io, mcp} {
    return await agent.run<Answer>(
        model: "assistant",
        instructions: "Help the user with their notes.",
        input: q,
        tools: [owner_name],                                    // اختياري
        mcp: ["notes"],                                         // اختياري: خوادم MCP
        require_approval: ["notes.add_note", "notes.delete_all"], // اختياري
        approve: |call| !call.tool.contains("delete"),          // fn({ tool, arguments }) -> Bool
        on_step: |step| print(step.tool, step.result),          // fn({ tool, arguments, result, approved })
        max_steps: 8                                            // اختياري، الافتراضي 10
    )
}
```

- `agent.run` مثل `ai.run` (4.6.3) مع: أدوات خوادم MCP المذكورة في `mcp`، والموافقة، وسجل الخطوات. يرى النموذج أداة MCP باسم `server__tool`، ويراها البرنامج `server.tool`.
- **الموافقة:** قبل تنفيذ أداة مذكورة في `require_approval` (أو كل أداة إن لم تُذكر القائمة) تُستدعى `approve` بـ `{ tool, arguments }` (`arguments` نص JSON)؛ إن أرجعت `false` لا تُنفَّذ الأداة ويُبلَّغ النموذج أن المستخدم رفضها. القرار في الشيفرة وليس في نص التعليمات (المواصفة §15.2). أسماء `require_approval` الحرفية يجب أن تكون أدوات الاستدعاء (`E2022`).
- **السجل:** `on_step` تُستدعى بعد كل استدعاء أداة بـ `{ tool, arguments, result, approved }` للتدقيق.
- **الصلاحيات:** لا يُشغَّل إلا خادم مذكور تحت `[mcp.servers]`؛ برنامج الخادم يعمل بصلاحيات المستخدم كاملة (مثل `std.py`). يبدأ الخادم عند أول استخدام في مجلد المشروع ويبقى حتى نهاية التشغيل.
- **التأثيرات:** `mcp.*` وقائمة `mcp` غير الفارغة تعني `mcp`؛ وتأثيرات الأدوات والدوال `approve`/`on_step` تُحسب على المستدعي، إضافة إلى `ai` (و`net` للنماذج السحابية).
- **البروتوكول:** عميل stdio لـ MCP بإصدار 2026-07-28 (بلا مصافحة، و`_meta` في كل طلب)، ويتراجع تلقائيًا إلى `initialize` مع الخوادم الأقدم (2025-11-25 وما قبلها). يدعم `tools/list` و`tools/call`؛ النتيجة نص المحتوى. الأدوات التي تطلب إدخالًا إضافيًا من المستخدم (elicitation) غير مدعومة بعد.
- `McpError`: `NotConfigured` (خادم غير مذكور) و`Unavailable` (تعذّر التشغيل أو توقف الخادم أو انتهت المهلة) و`Protocol` (خطأ JSON-RPC مثل أداة غير موجودة) و`ToolFailed` (الأداة أبلغت `isError`). في `agent.run` يصبح خادم غير مذكور `AIError.PolicyDenied` وتعذّر تشغيله `AIError.ProviderUnavailable`، وأخطاء الأدوات تُعاد إلى النموذج ليتصرف.

### 4.6.5 البث والتضمين: `ai.stream` و`ai.embed` (منذ 1.7)

| الدالة | النوع |
|---|---|
| `ai.stream(model, instructions, input, on_text[, timeout])` | `Future<Result<String, AIError>>`؛ و`on_text: fn(String)` تُستدعى بكل قطعة من الإجابة فور وصولها |
| `ai.embed(model, text[, timeout])` | `Future<Result<List<Float>, AIError>>`: متجه يدل اتجاهه على معنى النص |
| `ai.similarity(a, b)` | `List<Float>`, `List<Float>` → `Float`: جيب تمام الزاوية بين المتجهين، من -1 إلى 1، و0 إن كان أحدهما أصفارًا |
| `ai.search(model, query, texts[, timeout])` (منذ 1.9) | `Future<Result<List<{ text: String, score: Float }>, AIError>>`: النصوص مع قربها من `query`، الأقرب أولًا |

- يمر `ai.stream` و`ai.embed` بفحوص `ai.infer` نفسها (الحد، والنموذج، وإذن الشبكة)، وتأثيرهما `ai` (و`net` للنماذج السحابية).
- `ai.stream`: يبث مزودا OpenAI وAnthropic الإجابة فعلًا (`"stream": true`)، ويبث المزود الوهمي إجابته كلمةً كلمة. تُنفذ
  `on_text` في البرنامج بالترتيب بينما يصل الباقي؛ وإن فشلت أُوقف البث وكان خطؤها خطأ البرنامج. في ساحة التجربة بالمتصفح
  تُجمع القطع أولًا ثم تُعطى لـ`on_text`.
- `ai.embed`: مزود OpenAI يستخدم `/embeddings` بالنموذج المذكور في `model`؛ والمزود الوهمي يعطي متجهًا من 64 رقمًا من
  كلمات النص (الكلمات نفسها تعطي الاتجاه نفسه)، وقاعدة `error` تطابق النص تعطي خطأها؛ ومزود Anthropic لا يصنع تضمينات
  (`UnsupportedCapability`).
- `ai.similarity` حساب خالص بلا تأثيرات، فتُعدّ معزولة وتعمل داخل دوال `par_`.
- `await all([...])` (منذ 1.8) يرسل استدعاءات `ai.embed` إلى كل نموذج معًا: مزود OpenAI يضمّن حتى 256 نصًا في طلب واحد،
  والنتائج بترتيب القائمة، ولكل استدعاء فحوصه و`Result` خاص به، كما لو نُفذت واحدًا بعد الآخر.
- `ai.search` تضمّن السؤال والنصوص معًا كما يفعل `await all` (ويُحتسب كل نص ضمن `max_calls_per_run`)، ثم ترتبها بـ`ai.similarity`،
  والدرجات المتساوية تحفظ ترتيب `texts`. و`texts` الفارغة تعطي `Ok([])` دون أي استدعاء. وإن فشل أي تضمين فالنتيجة أول خطأ.

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
| `fs.is_dir(path)` | `Result<Bool, FsError>` | `filesystem_read` |
| `fs.size(path)` | `Result<Int, FsError>` (بالبايت؛ للملفات فقط) | `filesystem_read` |
| `fs.create_dir(path)` | `Result<Unit, FsError>` (مع المجلدات الأم الناقصة؛ لا خطأ إن وُجد) | `filesystem_write` |
| `fs.remove(path)` | `Result<Unit, FsError>` (ملف فقط) | `filesystem_write` |
| `fs.remove_dir(path)` | `Result<Unit, FsError>` (المجلد **وكل ما فيه**) | `filesystem_write` |
| `fs.rename(from, to)` | `Result<Unit, FsError>` (نقل أو إعادة تسمية) | `filesystem_write` للمسارين |
| `fs.copy(from, to)` | `Result<Unit, FsError>` (ملف فقط) | قراءة `from` وكتابة `to` |

- كل دوال `fs` تُحدث التأثير `fs`؛ والدالة `pub` التي تستخدمها يجب أن تصرّح بـ `!{fs}`.
- `write_text` لا تنشئ المجلدات؛ أنشئها أولًا بـ `fs.create_dir`.
- لا يُحذف مجلد مذكور بنفسه في `[permissions]` (`PolicyDenied`)، بل ما بداخله فقط. و`fs.remove` على مجلد أو `fs.remove_dir` على ملف `Other`.
- `()` طريقة أخرى لكتابة النوع `Unit`، فـ `Result<(), FsError>` هو `Result<Unit, FsError>`.
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
- اتصالات HTTPS (هنا وفي مزودي الذكاء الاصطناعي) تثق بالشهادة إن انتهت سلسلتها إلى الجذور المضمّنة في Elomar أو إلى جذور النظام نفسه
  (منذ 1.8)، فتعمل خلف برامج الحماية وخوادم الشركات الوسيطة التي تفحص HTTPS بجذر خاص بها.
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

## 4.11.3 Python: `std.py`

```elomar
use std.py

type Stats = { mean: Float, median: Float }

fn main() {
    // python/grades.py: def describe(grades, course): ...
    print(py.call<Stats>("grades", "describe", { grades: [91, 72], course: "Math" }))
    print(py.call<Float>("math", "sqrt", 2.0))          // قيمة واحدة = معامل موضعي
    print(py.call<Float>("statistics", "mean", [1, 2, 3], 10s))
}
```

| الدالة | النوع |
|---|---|
| `py.call<T>(module, function, args[, timeout])` | `Result<T, PyError>` |

- يحتاج `python = true` تحت `[permissions]` في `elomar.toml` (وإلا `PyError.PolicyDenied`)، لأن كود Python يعمل بصلاحيات المستخدم كاملة: أذونات Elomar للملفات والشبكة **لا تنطبق** داخله.
- كل استدعاء هو تأثير `py`، ويعمل في عملية Python مستقلة (`python3`، أو `python` على Windows، أو `ELOMAR_PYTHON`) تنتهي بعده؛ فلا حالة مشتركة بين الاستدعاءات.
- الوحدات تُبحث في `<project>/python` ثم مجلد المشروع ثم المكتبات المثبتة (`math` و`statistics` و`numpy`...).
- `args`: السجل يصبح معاملات مسماة (`f(grades=..., course=...)`)، وأي قيمة أخرى معامل موضعي واحد. القيم تعبر بصيغة JSON، والنتيجة يُتحقق منها مقابل `T` كما في `json.parse`.
- ما تطبعه الدالة يظهر بعد الاستدعاء. المهلة الافتراضية 60 ثانية.
- `PyError`: `PolicyDenied` و`Unavailable` (لا يوجد Python) و`Exception` (استثناء Python، مثل `"ValueError: ..."`) و`TypeMismatch` و`Timeout`.

## 4.11.4 الرياضيات والعشوائية والوقت والنظام

```elomar
use std.math
use std.random
use std.time
use std.os

fn main() {
    print(math.pi, math.sqrt(2), math.pow(2, 10))       // Int يُقبل حيث يُتوقع Float
    random.seed(7)                                        // نتائج قابلة للتكرار في الاختبارات
    print(random.int(1, 6), random.choice(["a", "b"]))
    let t = time.make(2026, 11, 20, 9, 30)               // ملّي ثانية منذ 1970-01-01 UTC
    print(time.format(t), time.date(t).weekday)           // 2026-11-20T09:30:00Z 5
    print(os.args(), os.env("HOME"))
}
```

| الوحدة | الدوال | التأثير |
|---|---|---|
| `std.math` | الثوابت `math.pi` `math.e` `math.tau` `math.inf` · `sqrt` `cbrt` `exp` `ln` `log10` `log2` `sin` `cos` `tan` `asin` `acos` `atan` (عدد → `Float`) · `pow(x, y)` `atan2(y, x)` `hypot(x, y)` · `is_nan(x) -> Bool` | — |
| `std.random` | `int(min, max) -> Int` (شامل الطرفين) · `float() -> Float` في [0, 1) · `bool()` · `choice(List<T>) -> Option<T>` · `shuffle(List<T>) -> List<T>` · `seed(Int)` | — |
| `std.time` | `now() -> Int` · `today() -> String` (`YYYY-MM-DD` بتوقيت UTC) · `sleep(Duration)` · `format(Int) -> String` (ISO 8601) · `format_as(Int, pattern) -> String` (`YYYY` `MM` `DD` `HH` `mm` `ss`، وما عداها يبقى كما هو) · `parse(String) -> Option<Int>` · `date(Int) -> { year, month, day, hour, minute, second, weekday }` (الاثنين = 1) · `make(year, month, day[, hour[, minute[, second]]]) -> Int` · `add_days(Int, n) -> Int` · `add_months(Int, n) -> Int` (31 يناير + شهر = آخر فبراير) · `days_between(a, b) -> Int` (أيام التقويم من `a` إلى `b`) | — |
| `std.os` | `args() -> List<String>` · `env(name) -> Option<String>` · `platform() -> String` (`windows`/`linux`/`macos`) · `exit(code)` | `io` (عدا `platform`) |

- `random` عشوائية زائفة (xorshift) تصلح للألعاب والمحاكاة، **لا** لكلمات المرور والمفاتيح.
- `time.parse` يقبل `YYYY-MM-DD` ثم اختياريًا `THH:MM[:SS[.fff]]` ومنطقة زمنية (`Z` أو `+03:00`)؛ بلا منطقة يُعدّ UTC. التاريخ غير الموجود (مثل 2001-02-29) يعطي `None`، و`time.make` بتاريخ غير صالح خطأ تنفيذي `R0006`.
- معاملات البرنامج: `elomar run main.elo a b --flag` ثم `os.args()` تعطي `["a", "b", "--flag"]`. و`os.exit(code)` ينهي البرنامج فورًا برمز الخروج (0–255).

## 4.11.5 التعابير النمطية والترميز والتشفير

```elomar
use std.regex
use std.encoding
use std.crypto

fn main() {
    print(regex.find_all("[0-9]+", "order 12 and 7"))          // ["12", "7"]
    print(regex.replace(r"(\d+)", "order 12", "#$1"))            // order #12
    print(encoding.base64_encode("مرحبا"), encoding.url_encode("a b"))
    print(crypto.sha256("abc"), crypto.uuid())
}
```

| الوحدة | الدوال |
|---|---|
| `std.regex` | `is_match(p, text) -> Bool` · `find(p, text) -> Option<String>` · `find_all(p, text) -> List<String>` · `captures(p, text) -> Option<List<String>>` (العنصر 0 المطابقة كلها ثم المجموعات) · `replace(p, text, with) -> String` (كل المطابقات، و`$1` للمجموعة) · `split(p, text) -> List<String>` |
| `std.encoding` | `base64_encode` · `hex_encode` · `url_encode` (`String -> String`) · `base64_decode` · `hex_decode` · `url_decode` (`String -> Option<String>`؛ `None` إن لم يكن الإدخال صالحًا أو لم تكن النتيجة UTF-8) |
| `std.crypto` | `sha256(text) -> String` (hex) · `hmac_sha256(key, message) -> String` (hex) · `uuid() -> String` (الإصدار 4) · `token(bytes) -> String` (hex، من 1 إلى 1024 بايت) · `hash_password(password) -> String` · `verify_password(password, stored) -> Bool` |

- صيغة التعابير النمطية هي صيغة مكتبة Rust `regex` (بلا رجوع للخلف backreferences، وزمن تنفيذ خطي مضمون). اكتب الأنماط نصوصًا خامًا: `r"\d{2,4}"`، لأن `{` في النص العادي يبدأ إدراجًا والشرطة المائلة تحتاج مضاعفة.
- النمط المكتوب نصًا حرفيًا يُفحص وقت الترجمة (`E2023`)؛ والنمط المبني وقت التشغيل إن كان غير صالح فخطأ تنفيذي `R0006`.
- `uuid` و`token` من مصدر عشوائي آمن يوفره النظام، فتصلح لمعرّفات الجلسات والمفاتيح، بخلاف `std.random`.
- لا تخزّن كلمات المرور بـ `sha256` وحده، بل خزّن ما ترجعه `hash_password` (منذ 1.19): Argon2id بالإعدادات التي
  توصي بها OWASP (‏19 MiB من الذاكرة، ومروران، ومسار واحد) وملح جديد من 16 بايت من المصدر الآمن في كل مرة، بصيغة
  PHC النصية القياسية (`$argon2id$v=19$m=19456,t=2,p=1$...`). و`verify_password` تتحقق من كلمة مرور مقابل هذا النص
  بالإعدادات المكتوبة فيه، والنص الذي ليس تجزئة صالحة يعطي `false`.

## 4.11.6 خادم الويب: `std.server`

```elomar
use std.server

fn handle(req: HttpRequest) -> HttpResponse {
    return match req.path {
        "/" => server.html(200, "<h1>مرحبًا</h1>")
        "/hello" => server.text(200, "Hello, {req.query.get_or("name", "guest")}!")
        "/api/time" => server.json(200, { ok: true })
        _ => server.text(404, "not found")
    }
}

fn main() {
    print(server.serve("127.0.0.1:8080", handle))
}
```

```toml
[permissions]
listen = ["127.0.0.1:8080"]     # منع افتراضي؛ "0.0.0.0:8080" للاستقبال من أجهزة أخرى
```

| الاسم | النوع |
|---|---|
| `HttpRequest` | `{ method: String, path: String, query: Map<String, String>, headers: Map<String, String>, body: String }` |
| `HttpResponse` | `{ status: Int, headers: Map<String, String>, body: String }` |
| `server.serve(address, handler[, max_requests])` | `Result<Unit, HttpError>`؛ تأثير `net`. لا تعود إلا بعد `max_requests` طلبًا (إن أُعطي وكان أكبر من 0) |
| `server.text(status, body)` · `server.html(status, body)` | `HttpResponse` بنوع المحتوى المناسب |
| `server.json(status, value)` | `HttpResponse`؛ أي قيمة تُحوَّل إلى JSON كما في `json.stringify` |
| `server.redirect(url)` | `HttpResponse` بالحالة 303 و`Location` |

- العنوان غير المذكور في `[permissions] listen` يعطي `HttpError.PolicyDenied` دون فتح أي منفذ، وفشل فتح المنفذ `HttpError.Network`.
- الطلبات تُعالج **واحدًا تلو الآخر**، فلا يعمل المعالج مرتين في الوقت نفسه. كل اتصال يحمل طلبًا واحدًا (`Connection: close`).
- `path` بعد فك ترميز URL، و`query` من `?a=1&b=2`، وأسماء `headers` بأحرف صغيرة. الجسم نص UTF-8 بحد أقصى 10 ميغابايت ويحتاج `Content-Length` (الإرسال المجزأ chunked يُرد بـ 411)، والرؤوس بحد أقصى 64 كيلوبايت، ومهلة القراءة 10 ثوانٍ.
- خطأ تنفيذي داخل المعالج يُرد عليه بـ 500، ويُطبع الخطأ، ويستمر الخادم. رؤوس الاستجابة التي تحوي سطرًا جديدًا تُحذف (حماية من حقن الرؤوس)، و`Content-Length` يُحسب تلقائيًا.
- طلب `HEAD` يُمرَّر للمعالج كما هو، ويُرسل الرد بلا جسم.
- لحفظ حالة بين الطلبات استخدم قاعدة بيانات (`std.db`) ومرّر الاتصال بدالة مجهولة: `server.serve(addr, |req| handle(conn, req))`. مثال كامل: `examples/web-api`.

## 4.11.7 تشغيل البرامج: `std.process`

```elomar
use std.process

fn main() {
    match process.run("git", ["log", "--oneline", "-3"]) {
        Ok(out) => print(out.status, out.stdout)
        Err(e) => print(e.kind(), e.message())
    }
}
```

```toml
[permissions]
commands = ["git"]     # منع افتراضي: البرامج المسموح بتشغيلها بالاسم
```

| الدالة | النوع |
|---|---|
| `process.run(command, args[, timeout])` | `Result<{ status: Int, stdout: String, stderr: String }, ProcessError>`؛ تأثير `process` |

- يُشغَّل البرنامج **مباشرة بلا shell**، فكل عنصر في `args` يصل كما هو: `"; rm -rf /"` نص عادي وليس أمرًا.
- المجلد الحالي هو مجلد المشروع، والمهلة الافتراضية 60 ثانية (ثم يُوقف البرنامج: `Timeout`). المخرجات بحد أقصى 16 ميغابايت لكل منهما.
- رمز خروج غير الصفر **ليس** خطأً: هو `status` في النتيجة. أما `ProcessError` فهو `PolicyDenied` (الاسم غير موجود في `commands`) و`NotFound` و`Failed` و`Timeout`.
- البرنامج المسموح يعمل بصلاحيات المستخدم كاملة، وأذونات Elomar للملفات والشبكة لا تنطبق داخله.

## 4.11.8 الجداول: `std.csv`

```elomar
use std.csv

fn main() {
    match csv.parse_records("name,city\nLina,Amman\nعمر,\"إربد, الأردن\"\n") {
        Ok(rows) => {
            for row in rows {
                print(row["name"], row["city"])
            }
        }
        Err(problem) => print("bad CSV:", problem)
    }
    print(csv.stringify([["name", "note"], ["Lina", "a, b"]]))
}
```

| الدالة | النوع |
|---|---|
| `csv.parse(text)` | `Result<List<List<String>>, String>`: كل الصفوف |
| `csv.parse_records(text)` | `Result<List<Map<String, String>>, String>`: الصف الأول أسماء الأعمدة |
| `csv.stringify(rows)` | `List<List<String>> -> String` (سطر لكل صف) |

- الصيغة RFC 4180: الحقل بين علامتي تنصيص يحمل الفواصل والأسطر، و`""` داخله علامة تنصيص. الأسطر تنتهي بـ `\n` أو `\r\n`، وتُحذف علامة BOM.
- الأخطاء نص يذكر رقم السطر: حقل لم يُغلق، أو نص بعد علامة الإغلاق، أو صف عدد حقوله غير عدد الأعمدة في `parse_records`.
- لا تأثير لها: لقراءة ملف استخدم `fs.read_text` ثم `csv.parse`.

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
  ويجوز أن يكون اسم وحدة قياسية (`use app.csv` للملف `src/csv.elo`)؛ ولا يكون `E1002` إلا في ملف يستورد `std.csv` و`app.csv` معًا.
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

### 4.12.1 الحزم (الاعتماديات)

```toml
# elomar.toml
[dependencies]
geometry = { path = "vendor/geometry" }                              # مجلد محلي
textkit = { git = "https://github.com/user/textkit", tag = "v1.0" }  # أو branch / rev
```

```elomar
use geometry.shapes               // src/shapes.elo في الحزمة geometry
use geometry.shapes.{Rect, area}
```

- الحزمة مجلد فيه `elomar.toml` ووحداته في `src/` (أو بجوار `elomar.toml`). تُستخدم بـ `use <package>.<module>` بقواعد `pub` نفسها، وداخل الحزمة يعني `use app.x` وحدتها هي `x`.
- اعتماديات الحزم تُحل أيضًا، والاسم الواحد يجب أن يشير دائمًا إلى المصدر نفسه. اسم الحزمة حروف لاتينية صغيرة وأرقام و`_`، وليس `app` ولا `std`.
- حزم git تُنزَّل بالأمر `git` إلى `~/.elomar/git` (أو `$ELOMAR_HOME/git`)، ويثبّت `elomar.lock` الـ commit الدقيق لكل منها حتى `elomar update`. الفحص في المحرر لا يتصل بالشبكة؛ أما `elomar run` و`elomar test` فينزّلان الناقص.
- الأوامر: `elomar add <name> --path <folder>` أو `--git <url> [--tag|--branch|--rev]`، و`elomar remove <name>`، و`elomar update`.
- خطأ في الاعتماديات (مجلد غير موجود، تعارض مصادر، حزمة لم تُنزَّل) `E1015`؛ ووحدة غير موجودة في حزمة `E1011`.

#### سجل الحزم (Registry)

```toml
# elomar.toml للتطبيق
[registry]                       # اختياري: الافتراضي السجل العام https://github.com/abobasel86/elomar-registry
index = "../my-registry"         # مستودع git أو مجلد محلي (أو متغير البيئة ELOMAR_REGISTRY)

[dependencies]
strings = "1.2"                 # أحدث إصدار يحقق ^1.2 من السجل
other = { version = ">=1, <3" }
```

- السجل **فهرس** وليس خادمًا: مستودع git (أو مجلد) فيه ملف لكل حزمة `packages/<name>.toml` يذكر `description` و`git` وقائمة `[[versions]]`، لكل إصدار `version` و`commit` كامل (40 خانة)، فلا تتغير شيفرة إصدار بعد نشره. `yanked = true` يخفي إصدارًا عن الحلول الجديدة دون كسر ملفات القفل الموجودة.
- متطلبات الإصدار بصيغة Cargo/semver: `"1.2"` تعني `^1.2`، و`"=1.0.0"`، و`">=1, <3"`. يُختار أحدث إصدار مطابق غير مسحوب، ويُسجَّل الإصدار والـ commit في `elomar.lock`؛ ولا يُعاد الحل إلا إذا لم يعد القفل يحقق المتطلب أو عند `elomar update`.
- لكل حزمة إصدار واحد في المشروع: إن طلبت حزمتان إصدارين غير متوافقين فهو خطأ `E1015` يذكر من طلب ماذا.
- الأوامر: `elomar add strings` (يكتب `"^<أحدث إصدار>"`) أو `elomar add strings@1.2`، و`elomar search [text]`.
- النشر: في مجلد الحزمة (مستودع git نظيف، و`version` تحت `[package]`) شغّل `elomar publish --index <نسخة محلية من مستودع السجل> [--git <URL>]`؛ يضيف الإصدار إلى ملف الحزمة في الفهرس، ثم تعمل commit وpush للفهرس (أو pull request). لا يرسل الأمر شيئًا إلى الشبكة بنفسه، ويحذّر إن لم يكن الـ commit مدفوعًا إلى فرع بعيد.

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
- `elomar new NAME --ai PROVIDER` (منذ 1.11)، مع `mock` أو `ollama` أو `openai` أو `anthropic`: مشروع يستدعي فيه `src/tickets.elo`
  `ai.infer<Ticket>`، وتبحث `main` فيه أيضًا بـ`ai.search` (إلا مع `anthropic` الذي لا يصنع تضمينات)، و`elomar.toml` فيه مضبوط
  لذلك المزود (النماذج و`[ai.cache]` و`max_calls_per_run` و`network_hosts`). وتنجح اختباراته دون إنترنت.
- `elomar doctor` (منذ 1.11) يفحص إعداد الذكاء الاصطناعي في المشروع ويقول ما يجب إصلاحه: صحة `elomar.toml`؛ ولكل نموذج: وجود
  متغير بيئة المفتاح (ولا يلزم لخادم على `localhost`)، ووجود مضيفه في `network_hosts`، واستجابة خادمه، ووجود معرّف النموذج بين ما
  يعرضه `GET .../models` (وهو مجاني). لا تُطبع المفاتيح أبدًا، ولا يُسأل إلا المضيفات المسموحة. ورمز الخروج 1 إن وُجدت مشكلة. ومنذ
  1.12 يفحص أيضًا الحزم (منزّلة ومثبّتة)، وPython (حين `python = true`)، وأوامر خوادم MCP و`[permissions] commands`، ومجلدات
  `filesystem_read` و`filesystem_write`.
- `elomar upgrade` (منذ 1.12) يسأل GitHub عن أحدث إصدار، وإن كان أحدث شغّل المثبّت الرسمي (`install.ps1` أو `install.sh`، ويتحقق
  من التنزيل بـ`SHA256SUMS.txt`) في مجلد `elomar` الذي يعمل. `--check` يخبر فقط إن كان هناك أحدث، و`--force` يعيد التثبيت.
  و`elomar` المثبّت بـ`cargo install` يُطلب منه التحديث بالطريقة نفسها.
- `elomar build FILE [-o OUT]` يفحص البرنامج ثم يكتب ملفًا تنفيذيًا واحدًا: نسخة من `elomar` نفسه وبآخرها ملفات البرنامج
  (الملف الرئيسي ووحداته ووحدات حزمه) و`elomar.toml`. عند تشغيله ينفّذ البرنامج مباشرة، وكل المعاملات تصل إلى `os.args()`،
  ولا يحتاج تثبيت Elomar-Lang. البرنامج الذي فيه أخطاء لا يُبنى (رمز الخروج 1). الملف للنظام الذي بُني عليه فقط.
  في البرنامج المبني تبدأ المسارات النسبية وقوائم `[permissions]` من **مجلد التشغيل الحالي**، لا من مجلد المشروع،
  وما يقرؤه البرنامج وقت التشغيل (ملفات البيانات، سكربتات Python، خوادم MCP) لا يُضمَّن فيه.
- `elomar fmt [PATHS] [--check]` ينسّق الملفات بأسلوب واحد: 4 مسافات لكل مستوى، ومسافات حول العوامل الثنائية، و`f(x, y)` و`{ a: 1 }` و`0..10` و`!{ai, net}`، وحذف `;` في نهاية السطر، ودمج الأسطر الفارغة المتتالية. يحتفظ بالتعليقات وبفواصل الأسطر التي كتبها المؤلف، ويتحقق أن الناتج يُنتج الرموز نفسها قبل الكتابة.

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
- الدوال المدمجة قيم أيضًا (منذ 1.9): `xs.map(to_string)` و`xs.fold(0, max)` و`let say = print`. تصير الدالة المدمجة الدالة
  المجهولة `|a, ...| print(a, ...)` بعدد معاملات نوع الدالة المتوقع، أو بعددها المعتاد (اثنان لـ`min` و`max` و`assert_eq`، وواحد
  للباقي). أما `all` و`timeout` فتعملان على المستقبلات وتبقيان استدعاءً فقط (`E0103`). ومساواة الدوال `==` بالهوية لا بالسلوك.
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

- الواجهة تذكر تواقيع الدوال؛ أول معامل `self`، و`Self` يعني النوع المنفِّذ (`fn scaled(self, k: Float) -> Self`). ويمكن أن يكون للدالة **جسم افتراضي** يستخدمه كل `impl` لا يعرّفها، مثل `fn greet(self) -> String { return "Hello, {self.name()}" }`.
- `impl Trait for Type` يجب أن ينفذ كل دوال الواجهة التي بلا جسم افتراضي بالتوقيع نفسه: الناقصة `E1014`، والمختلفة `E2001`، والزائدة `E1008` (ضعها في `impl Type` مستقلة).
- التنفيذ متاح لأي نوع: السجلات والأنواع المعدودة والأنواع الأساسية مثل `Int` و`List<Int>`. السجلات بنيوية، فـ `impl` على `Student` تنطبق على كل سجل بالحقول نفسها.
- القيد `<T: A + B>` يسمح باستدعاء دوال `A` و`B` على قيم `T`، وكل استدعاء للدالة العامة يجب أن يعطي نوعًا ينفذها (`E2019`)؛ وإن تعذر معرفة النوع `E2020`.
- التنفيذ المكرر لواجهة على النوع نفسه، أو دالتان تابعتان بالاسم نفسه للنوع نفسه، `E1002`.
- الدوال التابعة متاحة حيثما كان النوع متاحًا، ولها تأثيرات كأي دالة؛ وتأثيرات التنفيذ المستخدم عبر قيد تُحسب على من يستدعي الدالة العامة.
- `impl` لنوع عام: `impl<T> Stack<T> { fn push(self, x: T) -> Stack<T> { ... } }` و`impl<T> Describe for Box<T>`؛ و`T` يأخذ نوع القيمة عند كل استدعاء.
- `impl<T: A>` (منذ 1.7): قيد على معامل نوع الـ`impl`، يصير قيدًا على كل دوالها، فتستطيع استدعاء دوال `A` على قيم `T`.
  ويُمرَّر الـ`impl` حيث تُطلب واجهته (`fn show<X: Describe>`) ومعه تنفيذات `T`، متداخلًا أيضًا (`Box<Box<Int>>`). إن لم
  ينفذ `T` الواجهة `A` فالخطأ `E2019`.
- الدالة ذات القيود قيمة أيضًا (منذ 1.8): `xs.map(show)` و`let f = show` و`apply(show, 3)`. تُعرف تنفيذات الواجهات حين
  يُعرف النوع في الدالة نفسها؛ وإن لم يُعرف (`let f = show` دون استخدام) فالخطأ `E2020`، فاذكر النوع: `let f: fn(Int) -> String = show`.

## 5. التأثيرات (Effects)

التأثيرات المعروفة: `io` `fs` `net` `ai` `db` `py` `mcp` `process`. ولكلٍّ مصدره: `io` (`print` و`input` و`std.os`)
و`ai` (`ai.infer` و`ai.run` و`agent.run`) و`net` (`std.http` و`std.server` والنماذج السحابية، انظر 4.6.2) و`fs` (`std.fs`، انظر 4.10)
و`db` (`std.db`) و`py` (`std.py`) و`mcp` (`std.mcp`) و`process` (`std.process`).

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
| `List<T>` | `.length` · `push(T)`* · `pop() -> Option<T>`* · `insert(Int, T)`* · `remove(Int) -> T`* · `contains(T)` · `index_of(T) -> Option<Int>` · `is_empty()` · `first()` / `last() -> Option<T>` · `reverse()` · `sorted()` (أعداد أو نصوص) · `slice(start, end)` · `sum()` (أعداد) · `join(sep)` (`List<String>`) · `map(fn(T) -> U)` · `par_map(fn(T) -> U)` (مثل `map` على عدة أنوية، §4.5) · `filter` / `any` / `all` / `count(fn(T) -> Bool)` · `find(fn(T) -> Bool) -> Option<T>` · `par_filter(fn(T) -> Bool)` (مثل `filter` على عدة أنوية) · `par_any` / `par_all` / `par_count(fn(T) -> Bool)` · `par_find(fn(T) -> Bool) -> Option<T>` (مثل نظيراتها على عدة أنوية، §4.5) · `fold(U, fn(U, T) -> U)` · `par_fold(T, fn(T, T) -> T)` (طيّ بأجزاء ثابتة على عدة أنوية، §4.5) · `sort_by(fn(T) -> K)` · `par_sort_by(fn(T) -> K)` (المفاتيح على عدة أنوية) · `take(n)` / `skip(n)` · `unique()` · `min()` / `max() -> Option<T>` (أعداد أو نصوص) · `enumerate() -> List<{ index: Int, value: T }>` · `chunks(n) -> List<List<T>>` · `flat_map(fn(T) -> List<U>)` · `group_by(fn(T) -> K) -> Map<K, List<T>>` (`K` هو `Int` أو `String`) · `zip(List<U>) -> List<(T, U)>` (بطول الأقصر) · `windows(n) -> List<List<T>>` · `sum_by(fn(T) -> N)` (`N` هو `Int` أو `Float`) · `min_by` / `max_by(fn(T) -> K) -> Option<T>` (أول الأصغر أو الأكبر) · `par_flat_map` / `par_group_by` / `par_sum_by` / `par_min_by` / `par_max_by` (مثل نظيراتها، و`f` على عدة أنوية، §4.5) |
| `String` | `.length` (حروف مرئية) · `upper()` · `lower()` · `trim()` · `contains(s)` · `starts_with(s)` · `ends_with(s)` · `split(sep) -> List<String>` · `chars() -> List<String>` · `replace(a, b)` · `repeat(n)` · `parse_int()` · `parse_float()` · `is_empty()` · `index_of(s) -> Option<Int>` · `slice(start, end)` (بالحروف المرئية) · `lines()` · `words()` · `trim_start()` / `trim_end()` · `pad_start(n[, fill])` / `pad_end(n[, fill])` · `reverse()` |
| `Int` | `to_float()` · `abs()` · `pow(n)` (n ≥ 0) · `clamp(min, max)` |
| `Float` | `to_int()` · `round()` · `floor()` · `ceil()` (→ `Int`) · `abs()` · `sqrt()` · `pow(Float)` · `clamp(min, max)` · `round_to(digits) -> Float` · `to_fixed(digits) -> String` |
| `Option<T>` | `is_some()` · `is_none()` · `unwrap_or(T) -> T` · `unwrap_or_else(fn() -> T)` · `map(fn(T) -> U) -> Option<U>` · `and_then(fn(T) -> Option<U>)` · `filter(fn(T) -> Bool)` · `is_some_and(fn(T) -> Bool)` · `or(Option<T>)` · `ok_or(E) -> Result<T, E>` |
| `Result<T,E>` | `is_ok()` · `is_err()` · `unwrap_or(T) -> T` · `unwrap_or_else(fn(E) -> T)` · `map(fn(T) -> U) -> Result<U, E>` · `map_err(fn(E) -> F) -> Result<T, F>` · `and_then(fn(T) -> Result<U, E>)` · `ok() -> Option<T>` · `err() -> Option<E>` |
| `Map<K,V>` | `.length` · `get(K) -> Option<V>` · `contains_key(K)` · `remove(K) -> Option<V>`* · `keys() -> List<K>` · `values() -> List<V>` · `is_empty()` · `get_or(K, V) -> V` · `entries() -> List<{ key: K, value: V }>` · `filter(fn(K, V) -> Bool) -> Map<K, V>` · `any` / `all` / `count(fn(K, V) -> Bool)` · `map_values(fn(V) -> U) -> Map<K, U>` (منذ 1.7، بترتيب المفاتيح) |
| `Set<T>` | `.length` · `contains(T)` · `add(T)`* · `remove(T) -> Bool`* · `is_empty()` · `to_list()` · `union(Set<T>)` · `intersection(Set<T>)` · `difference(Set<T>)` · `is_subset(Set<T>) -> Bool` · `filter(fn(T) -> Bool) -> Set<T>` · `any` / `all` / `count(fn(T) -> Bool)` · `map(fn(T) -> U) -> Set<U>` (`U` هو `Int` أو `String`؛ منذ 1.7) |
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
| `E0007` | `{` داخل نص غير مغلق، أو `{}` فارغ (لقوس حرفي اكتب `\{`) |
| `E0101` | رمز غير متوقع (المتوقع كذا) |
| `E0102` | المتوقع تعبير |
| `E0103` | ميزة غير مدعومة (معاملات النوع لدالة غير عامة، `all` أو `timeout` كقيمة) |
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
| `E1015` | خطأ في الاعتماديات `[dependencies]` |

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
| `E2022` | أداة غير صالحة في `tools` لـ `ai.run` |
| `E2023` | تعبير نمطي حرفي غير صالح في `regex.*` |
| `E2024` | نمط بدائل `\|` يربط أسماء |
| `E2025` | نمط نطاق لا يطابق أي عدد |
| `E2026` | تفريق قيمة ليست صفًا بالطول المطلوب (`let (a, b) = ...`) |
| `E2027` | رقم عنصر خارج حدود الصف (`t.5`) |

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
| `R0006` | معامل غير صالح (جذر عدد سالب، تكرار نص بعدد سالب أو إلى أكثر من 100 مليون حرف، إكمال نص بـ `pad_start`/`pad_end` إلى أكثر من 100 مليون حرف، `pow` بأس سالب، `clamp` بحدود معكوسة، `random.int` بحدود معكوسة، تاريخ غير صالح في `time.make`) |
| `R0007` | تحويل عشري إلى `Int` غير ممكن |
| `R0008` | استخدام ثابت قبل تهيئته |
| `R0009` | خطأ داخلي في المفسّر (يُبلَّغ عنه كعلة) |
| `R0010` | انتظار `Future` أثناء تنفيذه |
| `R0011` | مفتاح غير موجود في `Map` عند `m[k]` |
| `R0012` | انتظار عملية أوقفتها `timeout` |
| `R0013` | تجاوز البرنامج كله حده الزمني (8 ثوانٍ في ساحة التجربة بالمتصفح) |

---

## 9. غير المنفذ بعد (مخطط)

لم يبقَ شيء مخطط له في اللغة نفسها دون تنفيذ. (يعمل منذ 1.9: الدوال المدمجة كقيم (§4.15) و`ai.search` (§4.6.5)؛ ومنذ 1.8:
الدالة ذات القيود كقيمة (§4.16)؛ ومنذ 1.7: `ai.stream` و`ai.embed` و`impl<T: A>`؛ ومنذ 1.2 تنفيذ `async fn` على عدة أنوية، §4.5.)
