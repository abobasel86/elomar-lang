---
layout: default
title: Elomar-Lang
lang: en
---

# Elomar-Lang

A simple, typed programming language with AI built in, designed to be easy
to learn in **Arabic** and **English**.

```elomar
use std.ai

type Ticket = { category: String, urgent: Bool }

fn main() {
    let name = "Lina"
    print("Hello, {name}!")
    print([3, 1, 2].sorted().map(|x| x * 10))
}
```

<div class="cards">
  <div class="card" lang="ar" dir="rtl">
    <h3>ابدأ بالعربية</h3>
    <p>دروس قصيرة تبدأ من الصفر حتى بناء خادم ويب وبرامج تستخدم الذكاء الاصطناعي.</p>
    <p><a href="ar/">الدروس بالعربية ←</a></p>
  </div>
  <div class="card">
    <h3>Start in English</h3>
    <p>Short lessons from your first program to web servers and AI-powered programs.</p>
    <p><a href="en/">The English tutorial →</a></p>
  </div>
</div>

## Why Elomar-Lang

- **Easy to read.** Plain syntax, clear error messages in English or Arabic,
  and Arabic names for your own variables and functions if you like.
- **Safe by default.** Static types, no `null`, errors as values, and
  programs may only touch the files, hosts, ports and programs you allow.
- **AI as part of the language.** `ai.infer<T>` returns a checked value of
  your type; agents use your functions and MCP tools, with approvals in code.
- **Batteries included.** Files, JSON, HTTP client and server, SQLite,
  regex, time, crypto, Python, packages, a formatter, tests and a VS Code
  extension.

## Install

Download `elomar` for Windows, Linux or macOS from the
[latest release](https://github.com/abobasel86/elomar-lang/releases/latest),
then follow [Getting started](en/01-start.html).
