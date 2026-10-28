### Clipboard Manager: every text of the window, in Arabic.
### The keys are the same as in locales/en/main.ftl; see docs/TRANSLATING.md.

## Times and dates

time-clock-am = { $time } ص
time-clock-pm = { $time } م
time-just-now = الآن
time-minutes-ago =
    { $count ->
        [one] منذ دقيقة
        [two] منذ دقيقتين
        [few] منذ { $count } دقائق
       *[other] منذ { $count } دقيقة
    }
time-hours-ago =
    { $count ->
        [one] منذ ساعة
        [two] منذ ساعتين
        [few] منذ { $count } ساعات
       *[other] منذ { $count } ساعة
    }
time-yesterday = أمس
time-today-at = اليوم { $time }
time-yesterday-at = أمس { $time }
time-date-at = { $date } { $time }
date-day-month = { $day } { $month }
date-day-month-year = { $day } { $month } { $year }
date-weekday-date = { $weekday } { $date }
weekday-mon = الإثنين
weekday-tue = الثلاثاء
weekday-wed = الأربعاء
weekday-thu = الخميس
weekday-fri = الجمعة
weekday-sat = السبت
weekday-sun = الأحد
month-jan = يناير
month-feb = فبراير
month-mar = مارس
month-apr = أبريل
month-may = مايو
month-jun = يونيو
month-jul = يوليو
month-aug = أغسطس
month-sep = سبتمبر
month-oct = أكتوبر
month-nov = نوفمبر
month-dec = ديسمبر

## Day headers in the list

group-pinned = المثبتة
group-today = اليوم
group-yesterday = أمس
group-this-week = هذا الأسبوع
group-this-month = هذا الشهر
group-older = أقدم

## Names: sort orders, views, filters, item kinds, search modes

sort-best-match = الأقرب تطابقًا
sort-recent = الأحدث
sort-oldest = الأقدم
sort-a-to-z = أبجديًا
sort-most-used = الأكثر استخدامًا
view-grid = شبكة
view-list = قائمة
view-detail = تفاصيل
filter-all = الكل
filter-text = نص
filter-links = الروابط
filter-images = الصور
filter-files = ملفات
filter-pinned = المثبتة
kind-text = نص
kind-link = رابط
kind-image = صورة
kind-files = ملفات
item-image-size = صورة { $width }×{ $height }
search-mode-smart = ذكي
search-mode-meaning = بالمعنى
search-mode-words = الكلمات نفسها
search-mode-smart-help = الكلمات والمعنى معًا (موصى به)
search-mode-meaning-help = بالمعنى فقط: «رحلة» تجد «طيران إلى جدة»
search-mode-words-help = العناصر التي تحتوي ما تكتبه فقط
top-view-tip = العرض: { $view } · انقر للتبديل إلى { $next }
top-sort-by = ترتيب حسب
top-sort-by-heading = ترتيب حسب
top-results =
    { $count ->
        [one] نتيجة واحدة
        [two] نتيجتان
        [few] { $count } نتائج
       *[other] { $count } نتيجة
    }
list-empty-title = سيظهر سجل الحافظة هنا
list-empty-help =
    انسخ أي شيء، أو أفلت الملفات هنا.
    اضغط { $key } في أي مكان لتختار نسخة وتلصقها.
list-no-matches = لا نتائج لـ «{ $query }»
list-meaning-finds-more = البحث بالمعنى يجد الكلمات المرتبطة أيضًا.
list-set-up-meaning = فعّل البحث بالمعنى
popup-open-window = فتح النافذة الكاملة
list-try-smart = جرّب البحث «الذكي» بدلًا منه
list-try-other-words = جرّب كلمات أقل أو مختلفة.
list-search-all-not-text = ابحث في كل العناصر، لا في النصوص فقط
list-search-all-not-links = ابحث في كل العناصر، لا في الروابط فقط
list-search-all-not-images = ابحث في كل العناصر، لا في الصور فقط
list-search-all-not-files = ابحث في كل العناصر، لا في الملفات فقط
list-search-all-not-pinned = ابحث في كل العناصر، لا في المثبتة فقط
list-none-yet = لا شيء هنا بعد
list-none-yet-text = لا يوجد نص بعد
list-none-yet-links = لا توجد روابط بعد
list-none-yet-images = لا توجد صور بعد
list-none-yet-files = لا توجد ملفات بعد
list-none-yet-pinned = لا توجد عناصر مثبتة بعد

## Main window

top-settings = الإعدادات
top-add-files-tip = أضف ملفات أو مجلدًا للبحث فيه (يمكنك أيضًا إفلاتها على النافذة)
top-add-files = إضافة ملفات…
top-add-folder = إضافة مجلد…
top-pause-recording = إيقاف التسجيل مؤقتًا
top-resume-recording = استئناف التسجيل
search-hint-images = ابحث في النصوص والصور بالمعنى…
search-hint-meaning = ابحث بالكلمات أو بالمعنى…
search-hint = ابحث في الحافظة…
search-clear = مسح (Esc)
tags-show-all = إظهار كل الوسوم
tags-filter-tip = العناصر ذات هذا الوسم فقط · انقر بالزر الأيمن للمزيد
tags-remove-everywhere = إزالة #{ $tag } من كل العناصر ({ $count })
tags-more =
    { $count ->
        [one] +وسم واحد
        [two] +وسمان
        [few] +{ $count } وسوم
        [many] +{ $count } وسمًا
       *[other] +{ $count } وسم
    }
update-version-label = { $version } (بناء { $build })
update-available = الإصدار { $version } متاح
update-verifying = جارٍ التحقق من تنزيل الإصدار { $version }…
update-downloading = جارٍ تنزيل الإصدار { $version }…
update-ready-restart = الإصدار { $version } جاهز. أعد التشغيل للتحديث، أو سيُثبَّت عند الخروج.
update-failed = تعذّر التحديث: { $error }
update-progress = { $done } / { $total } ميغابايت
update-later = لاحقًا
update-skip = تخطَّ هذا الإصدار
update-download = تنزيل
update-restart = أعد التشغيل للتحديث
update-try-again = حاول مجددًا
update-whats-new = ما الجديد
common-cancel = إلغاء
status-model-loading = جارٍ تحميل نموذج البحث…
status-model-builtin = مدمج · { $model }
status-model-server-default = النموذج الافتراضي في { $server }
status-model-off = بالكلمات فقط · فعّل البحث بالمعنى
status-model-unavailable = نموذج البحث غير متاح · أصلحه من الإعدادات
status-model-words-only = بالكلمات فقط
status-model-tip = نموذج البحث: انقر للتغيير
status-indexing = جارٍ الفهرسة · بقي { $count }
status-adding-files = جارٍ إضافة الملفات · أُضيف { $count }
status-recording-paused = التسجيل متوقف
status-shortcut-taken = الاختصار { $keys } يستخدمه تطبيق آخر
status-open-shortcut-settings = افتح إعدادات الاختصارات
status-dismiss-error = انقر للإغلاق
status-items =
    { $count ->
        [one] عنصر واحد
        [two] عنصران
        [few] { $count } عناصر
       *[other] { $count } عنصر
    }
status-items-pinned =
    { $count ->
        [one] عنصر واحد
        [two] عنصران
        [few] { $count } عناصر
       *[other] { $count } عنصر
    } · { $pinned } مثبت
list-drop-files = أفلت لإضافتها إلى مكتبتك القابلة للبحث
item-files-count =
    { $count ->
        [one] ملف واحد
        [two] ملفان
        [few] { $count } ملفات
       *[other] { $count } ملف
    }
item-empty = (فارغ)
item-delete = حذف
item-pin = تثبيت
item-unpin = إلغاء التثبيت
item-copy = نسخ
item-open-details = فتح التفاصيل
hint-with-keys = { $action } ({ $keys })
details-only-this-tag = إظهار العناصر ذات هذا الوسم فقط
details-remove-tag = إزالة الوسم
details-tag-hint = وسم، وآخر…
details-add-tag = إضافة وسم
details-tag = وسم
details-add-tag-tip = إضافة وسم
details-close = إغلاق التفاصيل (Esc)
details-pinned = المثبتة
details-from-files = من ملفاتك
details-formatted = مع التنسيق
details-copied-from = منسوخ من { $app }
details-ignore-app = عدم تسجيل النسخ من { $app }
details-delete = حذف
details-unpin = إلغاء التثبيت
details-pin-keep = تثبيت: احتفظ به دائمًا
details-save-image = حفظ الصورة باسم…
details-show-in-folder = إظهار في المجلد
details-open-in-browser = فتح في المتصفح
details-copy = نسخ
details-code-lines =
    { $count ->
        [one] سطر برمجي واحد
        [two] سطران برمجيان
        [few] { $count } أسطر برمجية
        [many] { $count } سطرًا برمجيًا
       *[other] { $count } سطر برمجي
    }
details-characters =
    { $count ->
        [one] حرف واحد
        [two] حرفان
        [few] { $count } أحرف
        [many] { $count } حرفًا
       *[other] { $count } حرف
    }
details-words =
    { $count ->
        [one] كلمة واحدة
        [two] كلمتان
        [few] { $count } كلمات
       *[other] { $count } كلمة
    }
details-image-size = { $width } × { $height } بكسل
details-has-text = تحتوي نصًا
details-no-text = لا نص فيها
details-reading-text = جارٍ قراءة النص…
details-files-count =
    { $count ->
        [one] عنصر واحد
        [two] عنصران
        [few] { $count } عناصر
       *[other] { $count } عنصر
    }
details-image-missing = ملف الصورة مفقود.
details-text-in-image = النص في الصورة
details-copy-text = نسخ النص
details-select-an-item = اختر عنصرًا لعرضه هنا
details-key-move = تنقّل
details-key-paste = لصق
details-key-paste-plain = لصق كنص عادي
details-key-copy-only = نسخ
details-key-pin = تثبيت
details-key-tag = وسم
details-key-delete = حذف
details-key-clear = مسح / إخفاء
shortcut-needs-modifier-global = أضف Ctrl أو Alt أو Shift أو Win
shortcut-needs-modifier = أضف Ctrl أو Alt أو Shift
shortcut-press-keys = اضغط المفاتيح…
shortcut-off = معطّل · انقر للتعيين
shortcut-recording-tip = اضغط المفاتيح الجديدة · Backspace للتعطيل · Esc للإلغاء
shortcut-click-to-set = انقر ثم اضغط المفاتيح الجديدة
toast-save-anyway = احفظه على أي حال
toast-undo = تراجع

## Settings

settings-page-general = عام
settings-page-search = البحث والذكاء الاصطناعي
settings-page-clipboard = الحافظة
settings-page-shortcuts = الاختصارات
settings-page-appearance = المظهر
settings-page-data = البيانات والخصوصية
app-title = مدير الحافظة
settings-back = رجوع (Esc)
settings-title = الإعدادات
settings-apply-model = تطبيق النموذج
settings-discard-changes = تجاهل التغييرات
settings-model-not-applied = لم تُطبّق تغييرات النموذج بعد
settings-saved-automatically = تُحفظ التغييرات تلقائيًا
settings-back-to-default = العودة إلى الافتراضي
settings-browse = استعراض…
settings-filter-matching = الملفات المطابقة
settings-filter-all = كل الملفات
settings-language-and-time = اللغة والوقت
settings-language = اللغة
settings-language-help = لغة التطبيق. العربية تجعل اتجاه النافذة من اليمين إلى اليسار.
settings-time-format = تنسيق الوقت
settings-time-format-help = طريقة عرض الأوقات. تُحفظ بالتوقيت العالمي (UTC) وتُعرض بتوقيتك المحلي.
settings-time-24h = 24 ساعة
settings-time-12h = 12 ساعة
settings-startup = بدء التشغيل
settings-start-at-sign-in = التشغيل عند تسجيل الدخول
settings-start-at-sign-in-help = يفتح التطبيق كلما سجّلت الدخول إلى ويندوز. يبدأ مخفيًا: في علبة النظام إن كانت مفعّلة، وإلا مصغّرًا.
settings-tray-section = علبة النظام
settings-tray = الاستمرار في العمل من علبة النظام
settings-tray-help = يعرض أيقونة في منطقة الإشعارات. إغلاق النافذة أو تصغيرها يخفيها هناك ويستمر تسجيل السجل. انقر الأيقونة للفتح، وانقر بالزر الأيمن للخروج.
settings-startup-note = عند تفعيل الخيارين، يبدأ التطبيق بهدوء في علبة النظام كلما سجّلت الدخول.
settings-updates = التحديثات
settings-update-check = التحقق من التحديثات تلقائيًا
settings-update-check-help = يبحث عن إصدار جديد عند بدء التطبيق ومرة يوميًا. يُجلب ملف الإصدار من GitHub فقط، ولا يُرسل أي شيء عنك أو عن حافظتك.
settings-update-download = تنزيل التحديثات تلقائيًا
settings-update-download-help = مفعّل: يُنزَّل الإصدار الجديد في الخلفية ويُثبَّت عند إعادة التشغيل أو الخروج. معطّل: يُطلب إذنك قبل تنزيل أي شيء.
settings-version = الإصدار
settings-version-help = يُتحقق من كل تنزيل بمفتاح توقيع هذا التطبيق قبل تثبيته.
settings-check-now = تحقّق الآن
settings-update-checking = جارٍ التحقق…
settings-update-current = لديك أحدث إصدار
update-ready = الإصدار { $version } جاهز
settings-updates-off-dev = التحديثات معطّلة في نسخ التطوير
update-development-build = { $version } (نسخة تطوير)
settings-look = الشكل
settings-theme = السمة
settings-theme-help = «النظام» يتبع إعداد الوضع الفاتح/الداكن في ويندوز.
settings-theme-system = النظام
settings-theme-light = فاتح
settings-theme-dark = داكن
settings-accent = لون التمييز
settings-accent-help = يُستخدم للتحديد والأزرار والإبراز.
accent-indigo = نيلي
accent-sky = سماوي
accent-emerald = زمردي
accent-amber = كهرماني
accent-rose = وردي
settings-list = قائمة
settings-layout = التخطيط
settings-layout-help = الشبكة تعرض بطاقات والقائمة تعرض صفوفًا، وكلاهما يفتح التفاصيل عند الطلب. التفاصيل تُبقي العنصر المحدد بجانب القائمة. زر العرض بجانب مربع البحث يبدّل أيضًا.
settings-density = الكثافة
settings-density-help = المضغوط يعرض عناصر أكثر بإخفاء السطر الثاني.
settings-density-comfortable = مريح
settings-density-compact = مضغوط
settings-pinned-on-top = المثبتة في الأعلى
settings-pinned-on-top-help = تظهر العناصر المثبتة أولًا أثناء التصفح، أيًّا كان الترتيب. أما نتائج البحث فتُرتّب حسب اختيارك فقط.
settings-group-by-day = التجميع حسب اليوم
settings-group-by-day-help = إظهار عناوين المثبتة واليوم وأمس… أثناء التصفح (لا أثناء البحث).
settings-recording = التسجيل
settings-record = تسجيل سجل الحافظة
settings-record-help = احفظ كل ما تنسخه. عطّله للإيقاف المؤقت، وزر الإيقاف في النافذة الرئيسية يفعل الشيء نفسه.
settings-skip-secrets = تجاهل كلمات المرور ومفاتيح API
settings-skip-secrets-help = لا تحفظ النسخ التي تبدو كلمة مرور أو مفتاح API أو رمزًا أو مفتاحًا خاصًا. ما زال بإمكانك لصقها كالمعتاد.
settings-merge-duplicates = دمج النسخ المكررة
settings-merge-duplicates-help = نسخ شيء موجود لديك ينقله إلى الأعلى بدلًا من إضافته مجددًا.
settings-read-images = قراءة النص في الصور
settings-read-images-help = اعثر على لقطات الشاشة والصور بالكلمات التي فيها.
settings-reader = قراءة النص بواسطة
settings-reader-help = يقرأ قارئ Windows اللغات المثبتة في Windows، ومنها العربية إن كانت مثبتة. القارئ المدمج يقرأ الحروف اللاتينية فقط وينزّل 12 ميغابايت أولًا. والخادم يشغّل نموذج رؤية تختاره.
settings-reader-windows = قارئ Windows
settings-reader-built-in = القارئ المدمج
settings-reader-server = خادم
settings-reader-address-help = عنوان خادم فيه نموذج رؤية، وينتهي عادةً بـ /v1. تُرسل الصور إلى /chat/completions فيه.
settings-reader-model-help = اتركه فارغًا لاستخدام النموذج الافتراضي للخادم.
settings-reader-prompt = التعليمة
settings-reader-prompt-help = ما يُطلب من النموذج مع كل صورة. بعض النماذج تريد كلماتها الخاصة: "Free OCR." لـ DeepSeek-OCR، و"OCR:" لـ PaddleOCR-VL.
settings-reader-remote = تُرسَل الصور التي تنسخها إلى { $host } لقراءتها، لأن هذا الخادم ليس على هذا الجهاز.
settings-reader-remote-http = تُرسَل الصور التي تنسخها إلى { $host } لقراءتها، لأن هذا الخادم ليس على هذا الجهاز. العنوان يبدأ بـ http، لذا تُنقل دون تشفير.
settings-reader-send-question = هل تُرسل الصور التي تنسخها إلى { $host } لقراءتها؟ هذا الخادم ليس على هذا الجهاز.
settings-reader-apply = تطبيق
settings-reader-new-images = الصور المقروءة من قبل تحتفظ بنصها، ويقرأ القارئ الجديد البقية.
settings-after-copying = بعد النسخ من القائمة
settings-hide-window = إخفاء النافذة
settings-hide-window-help = عند نسخ عنصر دون لصقه، تختفي النافذة لتلصقه بنفسك. أما اللصق فيخفي النافذة دائمًا.
settings-history-size = حجم السجل
settings-keep-up-to = الاحتفاظ بما يصل إلى
settings-keep-up-to-help = عند تجاوز العدد يُحذف الأقدم. لا تُحذف أبدًا العناصر المثبتة والموسومة والملفات التي أضفتها.
settings-history-items = { $size } عنصر
settings-delete-older = حذف العناصر الأقدم من
settings-delete-older-help = متوقف ما لم تختر مدة. تُحذف العناصر الأقدم عند مغادرة الإعدادات، ثم كل ساعة. لا تُحذف أبدًا العناصر المثبتة والموسومة والملفات التي أضفتها.
settings-age-off = متوقف
settings-age-day = يوم واحد
settings-age-week = أسبوع واحد
settings-age-month = شهر واحد
settings-age-three-months = 3 أشهر
settings-age-year = سنة واحدة
settings-age-days =
    { $count ->
        [one] يوم واحد
        [two] يومان
        [few] { $count } أيام
       *[other] { $count } يومًا
    }
settings-rules = القواعد
settings-rules-help = لا تُسجَّل النسخ التي تطابقها قاعدة. البرنامج هو اسم ملفه، مثل chrome.exe؛ اتركه فارغًا لكل البرامج. اترك النمط فارغًا لتخطي كل ما يُنسخ من البرنامج.
settings-rule-program = البرنامج
settings-rule-pattern = النمط
settings-rule-regex = تعبير نمطي
settings-rule-regex-help = عند الإيقاف: * تعني أي نص و? تعني حرفًا واحدًا، ويجب أن يطابق النمط النسخة كلها، لذا *كلمة* تجد الكلمة في أي مكان. عند التشغيل: تعبير نمطي يُبحث عنه في أي مكان من النسخة.
settings-rule-delete = حذف هذه القاعدة
settings-rule-add = إضافة قاعدة
settings-rule-needs = أدخل برنامجًا أو نمطًا أو كليهما. حتى ذلك الحين لا تفعل هذه القاعدة شيئًا.
settings-rule-bad-pattern = لا يمكن استخدام هذا التعبير النمطي، لذا لا تفعل القاعدة شيئًا: { $error }
settings-storage = التخزين
settings-data-folder = مجلد البيانات
settings-data-folder-help = يُحفظ السجل والصور والنماذج والإعدادات هنا، على هذا الجهاز فقط.
settings-open = فتح
settings-log-file = ملف السجل
settings-log-file-help = سجل بما فعله التطبيق في هذه الجلسة. أرفقه عند الإبلاغ عن مشكلة.
settings-show = إظهار
settings-stored = المخزَّن
settings-stored-counts =
    { $count ->
        [one] عنصر واحد
        [two] عنصران
        [few] { $count } عناصر
       *[other] { $count } عنصر
    } · { $images ->
        [one] صورة واحدة
        [two] صورتان
        [few] { $images } صور
       *[other] { $images } صورة
    } · { $pinned } مثبت
settings-clean-up = التنظيف
settings-clear-history = مسح سجل الحافظة
settings-clear-history-help = يحذف العناصر المنسوخة. تبقى العناصر المثبتة والموسومة والملفات التي أضفتها.
settings-delete-items =
    { $count ->
        [one] حذف عنصر واحد
        [two] حذف عنصرين
        [few] حذف { $count } عناصر
       *[other] حذف { $count } عنصر
    }
settings-clear = مسح…
settings-reset-all = إعادة ضبط كل الإعدادات
settings-reset-all-help = يعيد كل إعداد إلى قيمته الافتراضية، بما في ذلك نموذج البحث.
settings-reset-everything = إعادة ضبط الكل
settings-reset = إعادة ضبط…
settings-privacy-note = لا يُرسل أي شيء تنسخه إلى أي مكان. تعمل نماذج البحث على هذا الجهاز أو على خادم تختاره. التحقق من التحديثات يُنزّل ملف الإصدار من GitHub فقط.
settings-privacy-note-remote = يُرسَل ما تنسخه إلى { $host } لفهرسته، لأن نموذج البحث يعمل على ذلك الخادم. لا يُرسَل أي شيء آخر إلى أي مكان، والتحقق من التحديثات يُنزّل ملف الإصدار من GitHub فقط.
settings-privacy-note-reader-remote = تُرسَل الصور التي تنسخها إلى { $host } لقراءتها، لأن قارئ النص يعمل على ذلك الخادم. لا يُرسَل شيء آخر إلى أي مكان، والتحقق من التحديثات ينزّل ملف الإصدار من GitHub فقط.
settings-privacy-note-remote-both = يُرسَل ما تنسخه إلى { $search } لفهرسته، والصور التي تنسخها إلى { $reader } لقراءتها، لأن هذين الخادمين ليسا على هذا الجهاز. لا يُرسَل شيء آخر إلى أي مكان، والتحقق من التحديثات ينزّل ملف الإصدار من GitHub فقط.
settings-privacy-encrypted = سجلك مشفّر على هذا الجهاز: لا يفتحه إلا حسابك في ويندوز.
settings-strictness-all = إظهار الكل
settings-strictness-related = المرتبطة فقط
settings-strictness-close = القريبة فقط
settings-strictness-custom = مخصص ({ $value })
settings-search = البحث
settings-search-mode = طريقة البحث
settings-search-mode-help =
    ذكي: يجد العناصر التي تحتوي كلماتك والعناصر التي تحمل المعنى نفسه في قائمة واحدة.
    بالمعنى: بالمعنى فقط (يحتاج نموذج بحث).
    الكلمات نفسها: العناصر التي تحتوي ما تكتبه فقط.
settings-strictness = نتائج المعنى المعروضة
settings-strictness-help = مدى التطابق المطلوب لعرض العنصر. «إظهار الكل» يرتّب كل شيء حسب القرب، والخيارات الأشد تخفي التطابقات الضعيفة. تظهر مطابقات الكلمات نفسها دائمًا.
settings-model-section = نموذج البحث بالمعنى
settings-model = النموذج
settings-model-help =
    معطّل: البحث بالكلمات فقط (مع النص الموجود في الصور).
    مدمج: نموذج داخل التطبيق يفهم النصوص والصور، دون برامج أخرى (تنزيل لمرة واحدة).
    خادمك: استخدم نموذجًا تشغّله بالفعل، مثل llama.cpp أو Unsloth أو Sentence Transformers أو Ollama أو LM Studio.
settings-model-off = معطّل
settings-model-builtin = مدمج
settings-model-server = خادمك
settings-model-off-note = البحث بالكلمات فقط. فعّل نموذجًا لتجد العناصر بالمعنى أيضًا، مثلًا «رحلة» تجد «طيران إلى جدة».
settings-model-reindex = تُعاد فهرسة عناصرك في الخلفية.
settings-builtin-note = يعمل على معالج هذا الجهاز. يجد الصور ولقطات الشاشة من وصف، مثل «إيصال من مطعم».
settings-model-download-size = تنزيل { $repo } · { $size } غيغابايت
settings-model-downloaded = تم التنزيل
settings-model-progress = { $done } / { $total } ميغابايت
settings-model-download = تنزيل { $size } غيغابايت
settings-model-use = استخدام
model-siglip-multilingual = نموذج SigLIP الأساسي، متعدد اللغات (نصوص وصور)
model-siglip-english = نموذج SigLIP الأساسي، الإنجليزية (نصوص وصور)
settings-vector-full = كامل · أفضل جودة
settings-vector-512 = 512 · أصغر
settings-vector-256 = 256 · مضغوط
settings-vector-128 = 128 · صغير جدًا، الأسرع
settings-server-quick-setup = إعداد سريع:
settings-server-address = عنوان الخادم
settings-server-address-help = عنوان خادم التضمينات، ويكون عادةً منتهيًا بـ ‎/v1.
settings-server-remote = يُرسَل ما تنسخه إلى { $host } لفهرسته، لأن هذا الخادم ليس على هذا الجهاز.
settings-server-remote-http = يُرسَل ما تنسخه إلى { $host } لفهرسته، لأن هذا الخادم ليس على هذا الجهاز. العنوان يبدأ بـ http، لذا يُنقل دون تشفير.
settings-server-send-question = هل تُرسل سجلك كله إلى { $host } لفهرسته؟ هذا الخادم ليس على هذا الجهاز.
settings-server-send = إرسال
settings-server-send-cancel = إلغاء
settings-server-check-again = تحقّق مجددًا
settings-server-key = مفتاح API
settings-server-key-help = فقط إن طلبه خادمك. يُرسل كرمز Bearer ويُحفظ على هذا الجهاز فقط.
settings-server-hide-key = إخفاء المفتاح
settings-server-show-key = إظهار المفتاح
settings-server-optional = اختياري
settings-server-model = النموذج
settings-server-model-help = اتركه فارغًا لاستخدام نموذج التضمين الافتراضي للخادم. اختر من ▾ أو اكتب اسمًا يعرفه خادمك.
settings-server-models = النماذج على هذا الخادم
settings-server-default = الافتراضي للخادم
settings-server-default-named = الافتراضي للخادم · { $model }
settings-server-listed = يعرضها الخادم (قد تتضمن نماذج محادثة)
settings-server-default-hint = الافتراضي: { $model }
settings-server-default-lower = الافتراضي للخادم
settings-server-checking = جارٍ التحقق من الخادم…
settings-server-the-default-model = النموذج الافتراضي
settings-server-connected-uses = متصل · يستخدم { $model }
settings-server-no-default = متصل، لكن لا يوجد نموذج تضمين افتراضي: اختر واحدًا
settings-server-connected = متصل
settings-server-launch-note = يشغّل التطبيق llama-server بنفسه، ويتصل بعد التطبيق.
settings-server-gemma-tip = يبحث EmbeddingGemma أفضل بمطالباته الخاصة
settings-server-use-them = استخدمها
settings-server-advanced = متقدم
settings-server-query-prompt = عبارة الاستعلام
settings-server-query-prompt-help = نص يوضع قبل ما تبحث عنه. تحتاجه بعض النماذج (EmbeddingGemma: «task: search result | query: »).
settings-server-none = لا شيء
settings-server-item-prompt = عبارة العنصر
settings-server-item-prompt-help = نص يوضع قبل كل عنصر عند فهرسته (EmbeddingGemma: «title: none | text: »).
settings-server-vector-size = حجم المتجه
settings-server-vector-size-help = المتجهات الأصغر تستهلك ذاكرة أقل وتبحث أسرع، بجودة أقل قليلًا. تعمل مع نماذج Matryoshka مثل EmbeddingGemma وUltraVBERT.
settings-vector-custom = { $count } رقم
settings-server-images = ابحث في الصور بهذا النموذج
settings-server-images-help = أرسل الصور إلى الخادم أيضًا. فقط للنماذج التي تفهم الصور (CLIP وSigLIP وBiUltraVBERT…). وإلا فتُوجد الصور بالنص الذي فيها.
settings-server-launch = شغّل llama-server نيابةً عني
settings-server-launch-help = يشغّل التطبيق llama-server بملف ‎.gguf الخاص بك في الخلفية ويوقفه عند الخروج.
settings-server-program = برنامج llama-server
settings-server-program-help = الملف llama-server.exe من إصدار llama.cpp.
settings-server-model-file = ملف النموذج (‎.gguf)
settings-server-projector = مُسقِط الصور (‎.gguf)
settings-server-projector-help = ملف mmproj اختياري، للنماذج التي تضمّن الصور.
settings-server-port = المنفذ
settings-server-extra-args = وسائط إضافية
settings-server-extra-args-help = تُمرَّر إلى llama-server كما هي.
settings-shortcuts-global = في أي مكان في ويندوز
settings-shortcut-popup = فتح النافذة السريعة
settings-shortcut-popup-help = نافذة صغيرة عند مؤشر الماوس: ابحث، ثم الصق في البرنامج الذي أنت فيه. اضغط مرة أخرى لإغلاقها.
settings-shortcut-window = فتح النافذة الكاملة
settings-shortcut-window-help = النافذة التي فيها كل شيء: عوامل التصفية والتفاصيل والإعدادات. اضغط مرة أخرى لإخفائها.
settings-shortcut-ignore = لا تسجّل النسخة التالية
settings-shortcut-ignore-help = اضغطه قبل نسخ شيء خاص، فتُتجاهل النسخة التالية.
settings-shortcut-not-usable = غير صالح
settings-shortcut-taken = يستخدمه تطبيق آخر
settings-shortcuts-local = داخل التطبيق
settings-shortcut-paste = لصق العنصر المحدد في البرنامج الذي كنت تستخدمه
settings-shortcut-paste-help = يعمل هذا إذا فُتحت النافذة بالاختصار. وفي غير ذلك، ومع البرامج التي تعمل كمسؤول، يُنسخ العنصر فقط.
settings-shortcut-paste-plain = لصق العنصر المحدد نصًا عاديًا دون تنسيق
settings-shortcut-copy-only = نسخ العنصر المحدد دون لصقه
settings-shortcut-pin = تثبيت أو إلغاء التثبيت
settings-shortcut-delete = حذف
settings-shortcut-tag = إضافة وسم
settings-shortcut-view = التبديل بين الشبكة والقائمة والتفاصيل
settings-shortcut-used-twice = مستخدم مرتين
settings-shortcut-move = التنقل في القائمة
settings-shortcut-move-grid = التنقل بين البطاقات في الشبكة
settings-shortcut-escape = إغلاق التفاصيل أو مسح البحث أو الإخفاء
settings-shortcut-built-in = مدمج، لا يمكن تغييره.
settings-shortcuts-note = انقر اختصارًا ثم اضغط المفاتيح الجديدة. Backspace للتعطيل، وEsc للإلغاء.

## Errors, statuses, messages and the tray icon

error-saving-item = تعذّر حفظ العنصر: { $error }
error-saving-image = تعذّر حفظ الصورة: { $error }
error-saving-settings = تعذّر حفظ الإعدادات: { $error }
error-deleting = تعذّر الحذف: { $error }
error-copying = تعذّر النسخ: { $error }
error-opening-folder = تعذّر فتح المجلد: { $error }
error-background-crash = توقفت مهمة في الخلفية بسبب خطأ. التفاصيل في ملف السجل.
error-download-failed = فشل التنزيل: { $error }
error-download-crashed = توقف التنزيل بسبب خطأ. التفاصيل في ملف السجل.
error-tray-unavailable = أيقونة علبة النظام غير متاحة: { $error }
error-start-at-sign-in = تعذّر تغيير «التشغيل عند تسجيل الدخول»: { $error }
error-hotkeys-unavailable = الاختصارات التي تعمل في أي مكان في ويندوز غير متاحة: { $error }
error-hotkey-taken = الاختصار { $keys } يستخدمه تطبيق آخر أو ويندوز. اختر مفاتيح أخرى في الإعدادات، الاختصارات.
error-hotkey-invalid = لا يمكن استخدام الاختصار { $keys }: { $error }
error-indexing-skipped = تعذّرت فهرسة عنصر، فتم تخطيه.
error-query-embedding = تعذّر البحث بالمعنى: { $error }
error-model = تعذّر تحميل نموذج البحث: { $error }
error-model-not-downloaded = لم يُنزَّل { $model } بعد. نزّله من الإعدادات، البحث والذكاء الاصطناعي.
error-ocr-unavailable = تعذّرت قراءة النص في الصور: { $error }
error-settings-unreadable = تعذّرت قراءة الإعدادات ({ $error }). يستخدم التطبيق الآن الإعدادات الافتراضية. حُفظ الملف القديم باسم { $copy }
error-settings-unreadable-no-copy = تعذّرت قراءة الإعدادات ({ $error }). يستخدم التطبيق الآن الإعدادات الافتراضية. وتعذّر نسخ الملف القديم أيضًا: { $reason }
error-history-moved-aside = تعذّر فتح سجلك على حساب ويندوز هذا، فبدأ سجل جديد فارغ. حُفظ السجل القديم باسم { $file }
error-start-failed = تعذّر تشغيل مدير الحافظة.
status-loading-model = جارٍ تحميل { $model }…
status-starting-llama = جارٍ تشغيل llama-server…
status-connecting = جارٍ الاتصال بـ { $host }…
status-downloading-ocr = جارٍ تنزيل نماذج قراءة النص (12 ميغابايت، مرة واحدة)…
toast-copied = تم النسخ
toast-copied-admin = تم النسخ. ذلك البرنامج يعمل كمسؤول، لذا لا يسمح Windows لهذا التطبيق باللصق فيه. اضغط Ctrl+V هناك.
toast-copied-text = تم نسخ النص
toast-saved = تم الحفظ
toast-deleted = تم الحذف
toast-imported =
    { $count ->
        [one] تم استيراد ملف واحد
        [two] تم استيراد ملفين
        [few] تم استيراد { $count } ملفات
       *[other] تم استيراد { $count } ملف
    }
toast-cleared =
    { $count ->
        [one] تم مسح عنصر واحد
        [two] تم مسح عنصرين
        [few] تم مسح { $count } عناصر
       *[other] تم مسح { $count } عنصر
    }
toast-tag-removed =
    { $count ->
        [one] أُزيل #{ $tag } من عنصر واحد
        [two] أُزيل #{ $tag } من عنصرين
        [few] أُزيل #{ $tag } من { $count } عناصر
       *[other] أُزيل #{ $tag } من { $count } عنصر
    }
toast-downloaded = تم تنزيل { $model }
toast-skipped-private = لم يُحفظ: التطبيق الذي نسخت منه وسمه بأنه خاص
toast-skipped-secret = لم يُحفظ: يبدو كلمة مرور أو مفتاح API (راجع الإعدادات)
toast-skipped-rule = لم يُحفظ: إحدى قواعدك تتخطاه
toast-app-ignored = لن تُسجَّل النسخ من { $app }. يمكنك حذف القاعدة من الإعدادات، الحافظة.
tray-show = إظهار مدير الحافظة
tray-pause-recording = إيقاف التسجيل مؤقتًا
tray-resume-recording = استئناف التسجيل
tray-quit = خروج
