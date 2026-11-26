---
layout: default
title: Playground
lang: en
wide: true
other: ar/playground.html
---

# Playground

Write Elomar-Lang and run it here, in your browser. Nothing to install.

{% assign text = site.data.playground.en %}
{% include playground.html lang="en" text=text %}

## Gallery

Short programs that show what the language can do. Each one opens in the playground above and runs; its link (copy it from the address bar) opens it for anyone you send it to.

{% include gallery.html lang="en" run="Run it" %}
