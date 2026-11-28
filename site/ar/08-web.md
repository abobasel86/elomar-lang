---
layout: default
title: خوادم الويب وHTTP
lang: ar
slug: 08-web
---

# 8. خوادم الويب وHTTP

## خادم ويب في أسطر قليلة

```elomar
use std.server

fn handle(req: HttpRequest) -> HttpResponse {
    return match req.path {
        "/" => server.html(200, "<h1 dir=\"rtl\">أهلًا وسهلًا</h1>")
        "/hello" => server.text(200, "مرحبًا يا {req.query.get_or("name", "ضيف")}!")
        "/api/status" => server.json(200, { ok: true, version: "1.0" })
        _ => server.text(404, "غير موجود: {req.path}")
    }
}

fn main() {
    print("افتح http://127.0.0.1:8080")
    match server.serve("127.0.0.1:8080", handle) {
        Ok(_) => {}
        Err(e) => print("تعذر التشغيل: {e.message()}")
    }
}
```

واسمح بالعنوان في `elomar.toml`:

```toml
[permissions]
listen = ["127.0.0.1:8080"]
```

شغّله ثم افتح `http://127.0.0.1:8080/hello?name=لينا` في المتصفح. العنوان `127.0.0.1`
يقبل الاتصالات من جهازك فقط؛ واستخدم `0.0.0.0:8080` (في الكود وفي `listen`) لتقبلها
من أجهزة أخرى.

## الطلبات والاستجابات

| حقل `HttpRequest` | مثال |
|---|---|
| `method` | `"GET"` أو `"POST"` |
| `path` | `"/hello"` |
| `query` | `["name": "لينا"]` من `?name=لينا` |
| `headers` | أسماؤها بأحرف صغيرة: `req.headers.get("user-agent")` |
| `body` | النص المرسل مع `POST` أو `PUT` |

المعالج يعيد `HttpResponse`، وهو سجل `{ status, headers, body }`. الدوال المساعدة
`server.text` و`server.html` و`server.json` و`server.redirect` تبنيه لك، أو اكتبه بنفسك:

```elomar
use std.server

fn handle(req: HttpRequest) -> HttpResponse {
    return ({ status: 418, headers: ["X-Reason": "teapot"], body: "أنا إبريق شاي" })
}

fn main() {
    print(handle({ method: "GET", path: "/", query: [:], headers: [:], body: "" }).status)
}
```

ولأن المعالج دالة عادية، يمكنك اختباره دون تشغيل خادم، كما في السطر الأخير.

## واجهة JSON مع قاعدة بيانات

في المستودع مثال كامل في
[`examples/web-api`](https://github.com/abobasel86/elomar-lang/tree/main/examples/web-api):
يحفظ الملاحظات في SQLite ويجيب على `GET /notes` و`POST /notes` و`GET /notes/1`.
والطريقة لحفظ الحالة بين الطلبات هي تمرير قاعدة البيانات إلى المعالج بدالة مجهولة:

```text
server.serve("127.0.0.1:8080", |req| handle(conn, req))
```

## تسجيل الدخول {#login}

تحتاج صفحة الدخول ثلاثة أشياء يرسلها المتصفح أو يحفظها (منذ 1.20):

- `server.form(req)` تقرأ ما أرسله نموذج HTML ‏`<form method="post">` خريطةً: `name=لينا&password=...` تصير
  `["name": "لينا", ...]`.
- `server.cookies(req)` تقرأ ملفات الارتباط (cookies) التي أعادها المتصفح.
- `server.with_cookie(reply, name, value)` تضيف ملف ارتباط إلى الرد. وهو `HttpOnly` (لا تقرؤه سكربتات الصفحة)
  و`SameSite=Lax` (لا ترسله المواقع الأخرى مع نماذجها). والقيمة الفارغة تحذفه، وبها يكون الخروج.

```elomar
use std.server

fn main() {
    let req = { method: "POST", path: "/login", query: [:], headers: ["cookie": "theme=dark"], body: "name=Lina&password=secret+123" }
    let form = server.form(req)
    print(form["name"], form["password"])
    print(server.cookies(req)["theme"])
    let reply = server.with_cookie(server.redirect("/"), "session", "abc")
    print(reply.headers["Set-Cookie"])
}
```

ومع `crypto.hash_password` (انظر [درس صندوق الأدوات](10-toolbox.html#passwords)) وقاعدة بيانات يكتمل موقع
بحسابات. وفي المستودع واحد مكتوب بالعربية كاملًا:
[`examples/login`](https://github.com/abobasel86/elomar-lang/tree/main/examples/login) فيه حساب جديد ودخول وصفحة
خاصة وخروج، واختباراته (`elomar test` في مجلده) تستدعي الموقع بطلبات تُبنى في الاختبار، دون شبكة.

## الاتصال بخوادم أخرى

```elomar
use std.http
use std.json

type Answer = { ok: Bool }

fn main() {
    match http.get("https://example.com/api/status") {
        Ok(resp) => {
            print(resp.status)
            print(json.parse<Answer>(resp.body))
        }
        Err(e) => print(e.kind(), e.message())
    }
}
```

يجب السماح بالمضيف أولًا: `network_hosts = ["example.com"]` تحت `[permissions]`. وتثق اتصالات HTTPS بالشهادات
المضمّنة في Elomar وبالتي يثق بها نظامك، فتعمل خلف برامج الحماية وخوادم الشركات الوسيطة التي تفحص الاتصالات الآمنة.
