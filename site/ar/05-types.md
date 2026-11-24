---
layout: default
title: أنواعك الخاصة والأخطاء
lang: ar
slug: 05-types
---

# 5. أنواعك الخاصة والأخطاء

## السجلات (Records)

السجل يجمع قيمًا لها أسماء. وتعطي شكله اسمًا بـ `type`:

```elomar
type Student = { name: String, grade: Int }

fn صف(s: Student) -> String {
    return "{s.name} حصل على {s.grade}"
}

fn main() {
    let لينا: Student = { name: "لينا", grade: 91 }
    print(صف(لينا))
    let أفضل = { name: لينا.name, grade: لينا.grade + 5 }
    print(أفضل)
}
```

## الأنواع المعدودة (enum): نوع واحد من عدة أشكال

```elomar
enum Shape {
    Circle(Float)
    Rect(Float, Float)
    Dot
}

fn area(s: Shape) -> Float {
    return match s {
        Shape.Circle(r) => 3.14159 * r * r
        Shape.Rect(w, h) => w * h
        Shape.Dot => 0.0
    }
}

fn main() {
    for s in [Shape.Circle(1.0), Shape.Rect(2.0, 3.0), Shape.Dot] {
        print(area(s))
    }
}
```

إذا أضفت شكلًا جديدًا يصبح كل `match` نسيه خطأً، فلا تفوتك حالة.

## لا يوجد null: النوع Option

القيمة التي قد تغيب نوعها `Option<T>`: إما `Some(القيمة)` أو `None`، ويجب
التعامل مع الحالتين:

```elomar
fn find_age(name: String) -> Option<Int> {
    return match name {
        "لينا" => Some(30)
        _ => None
    }
}

fn main() {
    match find_age("لينا") {
        Some(age) => print("العمر {age}")
        None => print("غير معروف")
    }
    print(find_age("عمر").unwrap_or(0))
}
```

وبدل `match` تختصر دوال `Option` الكلام كثيرًا:

```elomar
fn main() {
    let العمر = "42".parse_int()
    print(العمر.map(|n| n + 1))                  // Some(43)
    print(العمر.filter(|n| n >= 18).is_some())   // true
    print("x".parse_int().unwrap_or_else(|| 0))
}
```

## الأخطاء قيم: النوع Result

الدالة التي قد تفشل تعيد `Result<T, E>`: إما `Ok(القيمة)` أو `Err(الخطأ)`. لا
توجد استثناءات تُرمى من وراء ظهرك:

```elomar
fn parse_age(text: String) -> Result<Int, String> {
    return match text.parse_int() {
        Some(n) => match n >= 0 {
            true => Ok(n)
            false => Err("العمر لا يكون سالبًا")
        }
        None => Err("'{text}' ليس عددًا")
    }
}

fn main() {
    for t in ["42", "-1", "abc"] {
        match parse_age(t) {
            Ok(age) => print("صحيح: {age}")
            Err(why) => print("خطأ: {why}")
        }
    }
}
```

## العامل ?

داخل دالة تعيد `Result`، يعيد `?` الخطأ فورًا، أو يعطيك القيمة:

```elomar
fn parse_age(text: String) -> Result<Int, String> {
    return match text.parse_int() {
        Some(n) => Ok(n)
        None => Err("'{text}' ليس عددًا")
    }
}

fn total_age(a: String, b: String) -> Result<Int, String> {
    let x = parse_age(a)?
    let y = parse_age(b)?
    return Ok(x + y)
}

fn main() {
    print(total_age("20", "22"))     // Ok(42)
    print(total_age("20", "x"))      // Err("'x' ليس عددًا")
}
```

ولـ `Result` دوال من النوع نفسه: `map` و`map_err` و`and_then` و`unwrap_or_else` و`ok` و`err`.

## الخروج المبكر من match

يمكن لفرع أن يستخدم `return`:

```elomar
fn double(text: String) -> Int {
    let n = match text.parse_int() {
        Some(n) => n
        None => return 0
    }
    return n * 2
}

fn main() {
    print(double("21"), double("?"))
}
```

## الأنماط المتداخلة

يمكن للنمط أن ينظر داخل نمط آخر:

```elomar
fn show(r: Result<Option<Int>, String>) -> String {
    return match r {
        Ok(Some(n)) => "وُجد {n}"
        Ok(None) => "لا شيء"
        Err(e) => "فشل: {e}"
    }
}

fn main() {
    print(show(Ok(Some(3))), show(Ok(None)), show(Err("القرص")))
}
```

## الصفوف (Tuples): عدة قيم معًا

الصف يجمع بضع قيم دون أن يسميها، وهو مفيد لإرجاع نتيجتين:

```elomar
fn min_max(xs: List<Int>) -> (Int, Int) {
    return (xs.min().unwrap_or(0), xs.max().unwrap_or(0))
}

fn main() {
    let (الأدنى, الأعلى) = min_max([4, 9, 2])
    print(الأدنى, الأعلى)
    let زوج = ("لينا", 91)
    print(زوج.0, زوج.1)
    for (الاسم, الدرجة) in ["لينا": 91, "عمر": 55] {
        print("{الاسم}: {الدرجة}")
    }
}
```

وعندما يكون لكل قيمة معنى يستحق اسمًا، فالسجل أوضح.

## الواجهات (trait): سلوك مشترك

```elomar
trait Describe {
    fn describe(self) -> String
}

type Book = { title: String }

impl Describe for Book {
    fn describe(self) -> String {
        return "كتاب {self.title}"
    }
}

fn main() {
    let b: Book = { title: "الأيام" }
    print(b.describe())
}
```

ويمكن للواجهة أن تعطي دالة جسمًا افتراضيًا، يحصل عليه كل `impl` ما لم يكتب جسمه الخاص:

```elomar
trait Greet {
    fn name(self) -> String
    fn greet(self) -> String {
        return "أهلًا يا {self.name()}!"
    }
}

type Person = { first: String }

impl Greet for Person {
    fn name(self) -> String {
        return self.first
    }
}

fn main() {
    let p: Person = { first: "لينا" }
    print(p.greet())
}
```

ويمكن أيضًا أن تكون الدوال التابعة لأنواع عامة:

```elomar
type Stack<T> = { items: List<T> }

impl<T> Stack<T> {
    fn push(self, x: T) -> Stack<T> {
        return { items: self.items + [x] }
    }
    fn top(self) -> Option<T> {
        return self.items.last()
    }
}

fn main() {
    let s: Stack<String> = { items: [] }
    print(s.push("أ").push("ب").top())
}
```

ويمكن لـ`impl` عام أن يشترط على معامل نوعه واجهةً، بالصيغة `impl<T: Describe>`. عندها تستطيع دواله استخدام دوال
الواجهة على `T`، ويصلح النوع حيثما احتيج إلى الواجهة:

```elomar
trait Describe {
    fn describe(self) -> String
}

type Box<T> = { item: T }

impl Describe for Int {
    fn describe(self) -> String {
        return "العدد {self}"
    }
}

impl<T: Describe> Describe for Box<T> {
    fn describe(self) -> String {
        return "صندوق فيه {self.item.describe()}"
    }
}

fn show<T: Describe>(x: T) -> String {
    return x.describe()
}

fn main() {
    let b: Box<Int> = { item: 7 }
    print(b.describe())     // صندوق فيه العدد 7
    let bb: Box<Box<Int>> = { item: b }
    print(show(bb))         // صندوق فيه صندوق فيه العدد 7
}
```

