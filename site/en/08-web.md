---
layout: default
title: Web servers and HTTP
lang: en
slug: 08-web
---

# 8. Web servers and HTTP

## A web server in a few lines

```elomar
use std.server

fn handle(req: HttpRequest) -> HttpResponse {
    return match req.path {
        "/" => server.html(200, "<h1>Welcome</h1>")
        "/hello" => server.text(200, "Hello, {req.query.get_or("name", "guest")}!")
        "/api/status" => server.json(200, { ok: true, version: "1.0" })
        _ => server.text(404, "Not found: {req.path}")
    }
}

fn main() {
    print("Open http://127.0.0.1:8080")
    match server.serve("127.0.0.1:8080", handle) {
        Ok(_) => {}
        Err(e) => print("cannot start: {e.message()}")
    }
}
```

Allow the address in `elomar.toml`:

```toml
[permissions]
listen = ["127.0.0.1:8080"]
```

Run it, then open `http://127.0.0.1:8080/hello?name=Lina` in a browser.
`127.0.0.1` accepts connections only from your own computer; use
`0.0.0.0:8080` (in the code and in `listen`) to accept them from other
machines.

## Requests and responses

| `HttpRequest` field | Example |
|---|---|
| `method` | `"GET"`, `"POST"` |
| `path` | `"/hello"` |
| `query` | `["name": "Lina"]` from `?name=Lina` |
| `headers` | names in lower case: `req.headers.get("user-agent")` |
| `body` | the text sent with `POST` or `PUT` |

A handler returns an `HttpResponse`, a record `{ status, headers, body }`.
The helpers `server.text`, `server.html`, `server.json` and
`server.redirect` build one for you, or you can write the record yourself:

```elomar
use std.server

fn handle(req: HttpRequest) -> HttpResponse {
    return ({ status: 418, headers: ["X-Reason": "teapot"], body: "I am a teapot" })
}

fn main() {
    print(handle({ method: "GET", path: "/", query: [:], headers: [:], body: "" }).status)
}
```

Because a handler is an ordinary function, you can test it without
starting a server, as the last line shows.

## A JSON API with a database

The repository has a complete example in
[`examples/web-api`](https://github.com/abobasel86/elomar-lang/tree/main/examples/web-api):
it stores notes in SQLite and answers `GET /notes`, `POST /notes` and
`GET /notes/1`. The trick for keeping state between requests is to pass the
database into the handler with a closure:

```text
server.serve("127.0.0.1:8080", |req| handle(conn, req))
```

## Logging in {#login}

A login page needs three things the browser sends or keeps (since 1.20):

- `server.form(req)` reads what an HTML `<form method="post">` sent, as a
  map: `name=Lina&password=...` becomes `["name": "Lina", ...]`.
- `server.cookies(req)` reads the cookies the browser sent back.
- `server.with_cookie(reply, name, value)` adds a cookie to a reply. It is
  `HttpOnly` (page scripts cannot read it) and `SameSite=Lax` (other sites
  cannot send it with their forms). An empty value deletes the cookie,
  which is how you log out.

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

Together with `crypto.hash_password` (see the
[toolbox lesson](10-toolbox.html#passwords)) and a database, that is a
whole site with accounts. The repository has one, written all in Arabic:
[`examples/login`](https://github.com/abobasel86/elomar-lang/tree/main/examples/login)
signs up, logs in, shows a private page and logs out, and its tests
(`elomar test` in that folder) call the site with requests built in the
test, without a network.

## Calling other servers

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

Hosts must be allowed first: `network_hosts = ["example.com"]` under
`[permissions]`. HTTPS trusts the certificates bundled with Elomar and the
ones your system trusts, so it works behind antivirus programs and company
proxies that inspect secure connections.
