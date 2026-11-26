---
layout: default
title: الدروس
lang: ar
---

# دروس Elomar-Lang

اثنا عشر درسًا قصيرًا. في كل درس برامج يمكنك نسخها إلى ملف وتشغيلها بالأمر
`elomar run`. لا تحتاج أي خبرة سابقة في البرمجة للدروس الخمسة الأولى.
وفي آخر معظم الدروس تمارين تحلّها في الصفحة نفسها: اكتب حلك، واضغط **تحقّق**،
فترى هل يعمل.

<p><img class="demo" src="../assets/demo.gif" width="1040" height="580" alt="طرفية: الأمر elomar new helpdesk --ai mock ينشئ مشروعًا، ثم النوع Ticket واستدعاء ai.infer، ثم elomar run يطبع [URGENT] billing، ثم رسالة خطأ بالعربية لمتغير اسمه عربي"></p>

لا وقت للتثبيت الآن؟ [صفحة التجربة](playground.html) تشغّل البرامج في متصفحك، وفي
[معرض البرامج](playground.html#ex-hello) أمثلة قصيرة تعمل بنقرة واحدة.

{% for l in site.data.lessons -%}
{{ forloop.index }}. [{{ l.ar }}]({{ l.slug }}.html)
{% endfor %}

كل الدوال في [المرجع](12-reference.html)، وصفحة [الجديد](news.html) تذكر ما أضافه كل إصدار. ولكل التفاصيل،
[مواصفة اللغة](https://github.com/abobasel86/elomar-lang/blob/main/docs/grammar.md) مكتوبة بالعربية وتشرح كل القواعد
والدوال ورموز الأخطاء.
