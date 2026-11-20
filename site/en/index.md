---
layout: default
title: Tutorial
lang: en
---

# The Elomar-Lang tutorial

Ten short lessons. Each one has programs you can copy into a file and run
with `elomar run`. No programming experience is needed for the first five.

{% for l in site.data.lessons -%}
{{ forloop.index }}. [{{ l.en }}]({{ l.slug }}.html)
{% endfor %}

Looking for every detail? The [language reference](https://github.com/abobasel86/elomar-lang/blob/main/docs/grammar.md)
lists all syntax, functions and error codes.
