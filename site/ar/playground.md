---
layout: default
title: جرّب اللغة
lang: ar
wide: true
other: en/playground.html
---

# جرّب اللغة

اكتب برنامجًا بلغة Elomar-Lang وشغّله هنا في المتصفح، دون تثبيت أي شيء.

{% assign text = site.data.playground.ar %}
{% include playground.html lang="ar" text=text %}

## معرض البرامج

برامج قصيرة تعرض ما تستطيعه اللغة. كل برنامج يُفتح في محرر التجربة أعلاه ويعمل مباشرة، ورابطه (انسخه من شريط العنوان) يفتحه لأي شخص ترسله إليه.

{% include gallery.html lang="ar" run="شغّله" %}
