---
layout: default
title: الدروس
lang: ar
---

# دروس Elomar-Lang

عشرة دروس قصيرة. في كل درس برامج يمكنك نسخها إلى ملف وتشغيلها بالأمر
`elomar run`. لا تحتاج أي خبرة سابقة في البرمجة للدروس الخمسة الأولى.

{% for l in site.data.lessons -%}
{{ forloop.index }}. [{{ l.ar }}]({{ l.slug }}.html)
{% endfor %}

تبحث عن كل التفاصيل؟ [مرجع اللغة](https://github.com/abobasel86/elomar-lang/blob/main/docs/grammar.md)
مكتوب بالعربية ويشرح كل القواعد والدوال ورموز الأخطاء.
