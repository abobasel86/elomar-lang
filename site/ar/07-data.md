---
layout: default
title: الملفات وJSON وقواعد البيانات
lang: ar
slug: 07-data
---

# 7. الملفات وJSON وقواعد البيانات

## الصلاحيات أولًا

لا يستخدم البرنامج إلا الملفات التي تسمح بها في `elomar.toml` بجانب برنامجك:

```toml
[permissions]
filesystem_read = ["data"]
filesystem_write = ["output"]
```

أي شيء آخر يُرفض بقيمة خطأ. هذا يحميك من برنامج (أو حزمة، أو نموذج ذكاء اصطناعي)
يلمس ملفات لا ينبغي له لمسها.

## الملفات

```elomar
use std.fs

fn main() {
    match fs.write_text("output/note.txt", "السطر الأول\n") {
        Ok(_) => print("تم الحفظ")
        Err(e) => print("تعذر الحفظ: {e.message()}")
    }
    match fs.read_text("data/students.txt") {
        Ok(text) => print(text.lines())
        Err(e) => print(e.kind())    // NotFound أو PolicyDenied ...
    }
}
```

ودوال أخرى: `fs.append_text` و`fs.exists` و`fs.list_dir`. وكل دالة تلمس الملفات لها
**تأثير** `fs`: الدالة العامة التي تقرأ ملفات يجب أن تصرّح بذلك بـ `!{fs}` في توقيعها،
فيعرف القارئ ذلك.

ويمكن أيضًا إنشاء المجلدات ونقل الملفات وحذفها:

```elomar
use std.fs

fn main() {
    print(fs.create_dir("output/reports/2026"))     // Ok(()) مع المجلدات الأم الناقصة
    print(fs.copy("data/students.txt", "output/reports/students.txt"))
    print(fs.rename("output/reports/students.txt", "output/reports/old.txt"))
    print(fs.size("output/reports/old.txt"), fs.is_dir("output/reports"))
    print(fs.remove("output/reports/old.txt"))      // ملف
    print(fs.remove_dir("output/reports"))          // مجلد وكل ما فيه
}
```

الحذف يحتاج صلاحية الكتابة، والمجلد المذكور بنفسه في `[permissions]` لا يُحذف أبدًا،
بل ما بداخله فقط.

## جداول CSV

برامج الجداول تحفظ الجداول بصيغة CSV. و`csv.parse_records` تقرؤها، والسطر الأول أسماء
الأعمدة:

```elomar
use std.csv

fn main() {
    let text = "الاسم,المدينة\nلينا,عمّان\nعمر,\"إربد، الأردن\"\n"
    match csv.parse_records(text) {
        Ok(rows) => {
            for row in rows {
                print("{row["الاسم"]} تسكن في {row["المدينة"]}")
            }
        }
        Err(problem) => print("ملف CSV غير صالح: {problem}")
    }
    print(csv.stringify([["الاسم", "الدرجة"], ["لينا", "91"]]))
}
```

أما `csv.parse` فتعطي الصفوف قوائم. ولقراءة ملف، استخدمها مع `fs.read_text`.

## JSON

`json.parse<T>` تقرأ JSON وتتأكد أن له شكل `T`:

```elomar
use std.json

type City = { name: String, people: Int, tags: List<String> }

fn main() {
    let text = "\{\"name\": \"عمّان\", \"people\": 4000000, \"tags\": [\"جبال\"]\}"
    match json.parse<City>(text) {
        Ok(city) => print(city.name, city.people)
        Err(e) => print("JSON غير صالح: {e.message()}")
    }
    let back = json.stringify({ ok: true, items: [1, 2] })
    print(back)
}
```

## قواعد البيانات (SQLite)

```elomar
use std.db

type Book = { id: Int, title: String, year: Int }

fn main() {
    let conn = match db.open(":memory:") {
        Ok(c) => c
        Err(e) => {
            print("تعذر الفتح: {e.message()}")
            return
        }
    }
    db.execute(conn, "CREATE TABLE books (id INTEGER PRIMARY KEY, title TEXT, year INT)")
    db.execute(conn, "INSERT INTO books (title, year) VALUES (:t, :y)", { t: "الأيام", y: 1929 })
    match db.query<Book>(conn, "SELECT * FROM books WHERE year < :before", { before: 2000 }) {
        Ok(books) => print(books)
        Err(e) => print(e.message())
    }
}
```

القيم تدخل عبر معاملات مسماة مثل `:t`، ولا تُلصق أبدًا داخل نص SQL، فحقن SQL
غير ممكن. وقاعدة البيانات في ملف (`db.open("app.db")`) تحتاج ذلك الملف في صلاحيات
القراءة والكتابة.
