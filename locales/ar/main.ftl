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
item-image = صورة
search-mode-smart = ذكي
search-mode-meaning = بالمعنى
search-mode-words = الكلمات نفسها
search-mode-smart-help = الكلمات والمعنى معًا (موصى به)
search-mode-meaning-help = بالمعنى فقط: «رحلة» تجد «طيران إلى جدة»
search-mode-words-help = العناصر التي تحتوي ما تكتبه فقط
top-sort-by = ترتيب حسب
top-sort-by-heading = ترتيب حسب
top-results =
    { $count ->
        [one] نتيجة واحدة
        [two] نتيجتان
        [few] { $count } نتائج
       *[other] { $count } نتيجة
    }
list-show-weaker =
    { $count ->
        [one] إظهار نتيجة أضعف واحدة
        [two] إظهار نتيجتين أضعف
        [few] إظهار { $count } نتائج أضعف
       *[other] إظهار { $count } نتيجة أضعف
    }
list-empty-title = سيظهر سجل الحافظة هنا
list-empty-help =
    انسخ أي شيء، أو أفلت الملفات هنا.
    اضغط { $key } في أي مكان لتختار نسخة وتلصقها.
list-no-matches = لا نتائج لـ «{ $query }»
list-meaning-finds-more = البحث بالمعنى يجد الكلمات المرتبطة أيضًا.
list-set-up-meaning = فعّل البحث بالمعنى
popup-open-window = فتح النافذة الكاملة
popup-close = إغلاق
popup-key-paste = لصق
popup-key-plain = نص عادي
popup-key-copy = نسخ
popup-key-quick = لصق سريع
popup-key-all = كل المفاتيح
quick-paste-keys = Ctrl+1 حتى 9
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
top-pause = إيقاف مؤقت
top-resume = استئناف
search-hint-images = ابحث في النصوص والصور بالمعنى…
search-hint-meaning = ابحث بالكلمات أو بالمعنى…
search-hint = ابحث في الحافظة…
search-hint-short = ابحث…
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
download-retrying =
    { $seconds ->
        [one] انقطع الاتصال. إعادة المحاولة بعد ثانية واحدة…
        [two] انقطع الاتصال. إعادة المحاولة بعد ثانيتين…
        [few] انقطع الاتصال. إعادة المحاولة بعد { $seconds } ثوانٍ…
       *[other] انقطع الاتصال. إعادة المحاولة بعد { $seconds } ثانية…
    }
update-later = لاحقًا
update-skip = تخطَّ هذا الإصدار
update-download = تنزيل
update-restart = أعد التشغيل للتحديث
update-try-again = حاول مجددًا
update-whats-new = ما الجديد
common-cancel = إلغاء
status-model-loading = جارٍ تحميل نموذج البحث…
status-model-off = بالكلمات فقط · فعّل البحث بالمعنى
status-model-unavailable = نموذج البحث غير متاح · أصلحه من الإعدادات
status-model-failing = خادم البحث لا يعمل · حاول مجددًا
status-model-failing-tip = البحث بالمعنى يفشل الآن. يحاول التطبيق مجددًا من تلقاء نفسه، وانقر لتحاول الآن.
status-model-words-only = بالكلمات فقط
status-model-tip = نموذج البحث: انقر للتغيير
status-model-on = { $model } · { $server }
status-model-tip-named = نموذج البحث: { $model } على { $server }. انقر للتغيير.
status-indexing = جارٍ الفهرسة · { $done } من { $total }
status-adding-files = جارٍ إضافة الملفات · أُضيف { $count }
status-recording-paused = التسجيل متوقف
status-recording-stopped = توقف التسجيل بسبب خطأ · ابدأه من جديد
status-recording-stopped-tip = توقف التسجيل بعد أخطاء متكررة. انقر لتبدأه من جديد.
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
item-color = لون · { $value }
item-delete = حذف
item-pin = تثبيت
item-unpin = إلغاء التثبيت
item-copy = نسخ
item-open-details = فتح التفاصيل
item-paste-into = لصق في { $app }
item-paste = لصق
item-paste-plain = لصق كنص عادي
item-copy-only = نسخ فقط
item-copy-plain = نسخ كنص عادي
item-copy-image-text = نسخ النص الموجود في الصورة
item-ignore-app = عدم تسجيل النسخ من { $app }
item-menu = الإجراءات
item-paste-group =
    { $count ->
        [one] لصق عنصر واحد
        [two] لصق العنصرين معًا
        [few] لصق { $count } عناصر معًا
       *[other] لصق { $count } عنصر معًا
    }
item-copy-group =
    { $count ->
        [one] نسخ عنصر واحد
        [two] نسخ العنصرين معًا
        [few] نسخ { $count } عناصر معًا
       *[other] نسخ { $count } عنصر معًا
    }
item-pin-group =
    { $count ->
        [one] تثبيت عنصر واحد
        [two] تثبيت العنصرين
        [few] تثبيت { $count } عناصر
       *[other] تثبيت { $count } عنصر
    }
item-unpin-group =
    { $count ->
        [one] إلغاء تثبيت عنصر واحد
        [two] إلغاء تثبيت العنصرين
        [few] إلغاء تثبيت { $count } عناصر
       *[other] إلغاء تثبيت { $count } عنصر
    }
item-tag-group =
    { $count ->
        [one] إضافة وسم إلى عنصر واحد
        [two] إضافة وسم إلى العنصرين
        [few] إضافة وسم إلى { $count } عناصر
       *[other] إضافة وسم إلى { $count } عنصر
    }
item-delete-group =
    { $count ->
        [one] حذف عنصر واحد
        [two] حذف العنصرين
        [few] حذف { $count } عناصر
       *[other] حذف { $count } عنصر
    }
group-selected =
    { $count ->
        [one] عنصر واحد محدد
        [two] عنصران محددان
        [few] { $count } عناصر محددة
       *[other] { $count } عنصر محدد
    }
group-pin = تثبيت
group-unpin = إلغاء التثبيت
group-tag = إضافة وسم
group-copy = نسخ معًا
group-delete = حذف
group-clear = إلغاء التحديد
keys-sheet-title = المفاتيح
keys-sheet-close = إغلاق
checklist-title = ابدأ من هنا
checklist-copy = انسخ أي شيء في أي برنامج
checklist-paste = اضغط { $keys } في برنامج آخر، ثم الصق عنصرًا
checklist-paste-no-key = افتح هذه النافذة فوق برنامج آخر، ثم الصق عنصرًا
checklist-pin = ثبّت عنصرًا ليبقى في الأعلى
checklist-meaning = فعّل البحث بالمعنى (اختياري)
checklist-set-up = إعداد
checklist-close = إغلاق قائمة البدء
whats-new-title = ما الجديد في الإصدار { $version }
whats-new-close = إغلاق
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
details-delete = حذف
details-unpin = إلغاء التثبيت
details-pin-keep = تثبيت: احتفظ به دائمًا
details-save-image = حفظ الصورة باسم…
details-show-in-folder = إظهار في المجلد
details-open-in-browser = فتح في المتصفح
details-more-ways = طرق أخرى للصق أو النسخ
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
details-read-failed = تعذرت قراءة النص
details-read-again = حاول مجددًا
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
details-key-quick = لصق أحد العناصر التسعة الأولى
details-key-menu = القائمة
details-key-all = كل المفاتيح
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
toast-show = إظهار
toast-skip-next = لن تُحفظ النسخة التالية
toast-undo = تراجع
toast-images-refused = خادم البحث لا يقبل الصور، فيُبحث عنها بالنص الذي فيها.
toast-images-off = أوقف إرسال الصور
toast-images-turned-off = أُوقف إرسال الصور لهذا النموذج.

## Settings

settings-page-general = عام
settings-page-search = البحث والذكاء الاصطناعي
settings-page-privacy = الخصوصية
settings-page-about = حول التطبيق
settings-page-clipboard = الحافظة
settings-page-shortcuts = الاختصارات واللصق
settings-page-appearance = المظهر
app-title = مدير الحافظة
settings-back = رجوع (Esc)
settings-title = الإعدادات
settings-saved-automatically = تُحفظ التغييرات تلقائيًا
settings-back-to-default = العودة إلى الافتراضي
settings-browse = استعراض…
settings-filter-matching = الملفات المطابقة
settings-filter-all = كل الملفات
settings-language-and-time = اللغة والوقت
settings-start-and-tray = البدء وعلبة النظام
settings-language = اللغة
settings-language-hint = لغة التطبيق
settings-language-help = العربية تجعل اتجاه النافذة من اليمين إلى اليسار.
settings-time-format = تنسيق الوقت
settings-time-format-hint = طريقة عرض الأوقات
settings-time-format-help = تُحفظ بالتوقيت العالمي (UTC) وتُعرض بتوقيتك المحلي.
settings-time-24h = 24 ساعة
settings-time-12h = 12 ساعة
settings-start-at-sign-in = التشغيل عند تسجيل الدخول
settings-start-at-sign-in-hint = يفتح التطبيق كلما سجّلت الدخول إلى ويندوز
settings-start-at-sign-in-help = يبدأ مخفيًا: في علبة النظام إن كانت مفعّلة، وإلا مصغّرًا.
settings-tray = الاستمرار في العمل من علبة النظام
settings-tray-hint = يعرض أيقونة في منطقة الإشعارات
settings-tray-help = إغلاق النافذة أو تصغيرها يخفيها هناك ويستمر تسجيل السجل. انقر الأيقونة للفتح، وانقر بالزر الأيمن للخروج.
settings-startup-note = عند تفعيل الخيارين، يبدأ التطبيق بهدوء في علبة النظام كلما سجّلت الدخول.
settings-updates = التحديثات
settings-update-check = التحقق من التحديثات تلقائيًا
settings-update-check-hint = يبحث عن إصدار جديد عند بدء التطبيق ومرة يوميًا
settings-update-check-help = يُجلب ملف الإصدار من GitHub فقط، ولا يُرسل أي شيء عنك أو عن حافظتك.
settings-update-download = تنزيل التحديثات تلقائيًا
settings-update-download-hint = تُنزَّل الإصدارات الجديدة في الخلفية وتُثبَّت عند الخروج
settings-update-download-help = عند إيقافه، يُطلب إذنك قبل تنزيل أي شيء.
settings-version = الإصدار
settings-version-hint = يُتحقق من كل تنزيل بمفتاح توقيع هذا التطبيق قبل تثبيته
settings-check-now = تحقّق الآن
settings-update-checking = جارٍ التحقق…
settings-update-current = لديك أحدث إصدار
update-ready = الإصدار { $version } جاهز
settings-updates-off-dev = التحديثات معطّلة في نسخ التطوير
update-development-build = { $version } (نسخة تطوير)
settings-look = الشكل
settings-theme = السمة
settings-theme-hint = «النظام» يتبع إعداد الوضع الفاتح/الداكن في ويندوز
settings-theme-system = النظام
settings-theme-light = فاتح
settings-theme-dark = داكن
settings-dark-panels = الألواح الداكنة
settings-dark-panels-hint = في المظهر الداكن، لوحة التفاصيل والبطاقات والقوائم: أغمق من النافذة، أو أفتح
settings-dark-panels-darker = أغمق
settings-dark-panels-lighter = أفتح
settings-accent = لون التمييز
settings-accent-hint = يُستخدم للتحديد والأزرار والإبراز
accent-indigo = نيلي
accent-sky = سماوي
accent-emerald = زمردي
accent-amber = كهرماني
accent-rose = وردي
settings-list = قائمة
settings-layout = التخطيط
settings-layout-hint = الشبكة تعرض بطاقات والقائمة تعرض صفوفًا، وكلاهما يفتح التفاصيل عند الطلب
settings-layout-help = التفاصيل تُبقي العنصر المحدد بجانب القائمة. زر العرض بجانب مربع البحث يبدّل أيضًا.
settings-density = الكثافة
settings-density-hint = المضغوط يعرض عناصر أكثر بإخفاء السطر الثاني
settings-density-comfortable = مريح
settings-density-compact = مضغوط
settings-pinned-on-top = المثبتة في الأعلى
settings-pinned-on-top-hint = تظهر العناصر المثبتة أولًا أثناء التصفح، أيًّا كان الترتيب
settings-pinned-on-top-help = أما نتائج البحث فتُرتّب حسب اختيارك فقط.
settings-group-by-day = التجميع حسب اليوم
settings-group-by-day-hint = إظهار عناوين المثبتة واليوم وأمس… أثناء التصفح (لا أثناء البحث)
settings-recording = التسجيل
settings-how-long = مدة بقاء العناصر
settings-record = تسجيل سجل الحافظة
settings-record-hint = احفظ كل ما تنسخه
settings-record-help = عطّله للإيقاف المؤقت، وزر الإيقاف في النافذة الرئيسية يفعل الشيء نفسه.
settings-skip-secrets = تجاهل كلمات المرور ومفاتيح API
settings-skip-secrets-hint = لا تحفظ النسخ التي تبدو كلمة مرور أو مفتاح API أو رمزًا أو مفتاحًا خاصًا
settings-skip-secrets-help = ما زال بإمكانك لصقها كالمعتاد.
note-title = ملاحظة مدير الحافظة
settings-screen-notes = ملاحظات على الشاشة
settings-screen-notes-hint = ملاحظة في زاوية الشاشة حين لا تُحفظ نسخة، أو حين لن تُحفظ النسخة التالية
settings-screen-notes-help = لا تأخذ لوحة المفاتيح أبدًا، وتبقى صامتة أثناء برنامج بملء الشاشة أو عرض تقديمي. انقرها لفتح النافذة.
settings-never-kept = ما لا يُحفظ أبدًا
settings-skip-next = عدم تسجيل النسخة التالية
settings-skip-next-hint = اضغط هذا المفتاح قبل أن تنسخ شيئًا لا تريد حفظه، مثل كلمة مرور
settings-change-key = تغيير
settings-data-goes = أين تذهب بياناتك
settings-data-history = سجلك
settings-data-history-encrypted = مشفّر على هذا الجهاز
settings-data-history-local = على هذا الجهاز
settings-data-search = البحث بالمعنى
settings-data-reader = النص في الصور
settings-data-updates = التحقق من التحديثات
settings-data-off = متوقف
settings-data-on-pc = يبقى على هذا الجهاز
settings-data-sent = يُرسَل إلى { $host }
settings-data-windows = يقرؤه Windows على هذا الجهاز
settings-data-updates-on = ينزّل ملف الإصدار فقط، من GitHub
settings-pasting = اللصق
settings-reset-section = إعادة الضبط
settings-merge-duplicates = دمج النسخ المكررة
settings-merge-duplicates-hint = نسخ شيء موجود لديك ينقله إلى الأعلى بدلًا من إضافته مجددًا
settings-reader-kind = نوع النموذج
settings-reader-kind-hint = يضبط التعليمات والإعدادات التي تناسب النموذج
settings-reader-kind-help = الخادم هو من يشغّل النموذج، ويشرح ملف README طريقة تشغيله.
reader-kind-other = نموذج آخر
settings-reader-prompt = التعليمة
settings-reader-prompt-hint = ما يُطلب من النموذج مع كل صورة
settings-reader-prompt-help = يملؤه نوع النموذج. وقد يريد نموذج آخر كلماته الخاصة، مثل "OCR:" لـ PaddleOCR-VL.
settings-reader-send-question = هل تُرسل الصور التي تنسخها إلى { $host } لقراءتها؟ هذا الخادم ليس على هذا الجهاز.
settings-hide-window = إخفاء النافذة
settings-hide-window-hint = تخفي النافذة بعد نسخ عنصر، لتلصقه بنفسك
settings-hide-window-help = أما اللصق فيخفي النافذة دائمًا.
settings-keep-up-to = الاحتفاظ بما يصل إلى
settings-keep-up-to-hint = عند تجاوز العدد يُحذف الأقدم
settings-keep-up-to-help = لا تُحذف أبدًا العناصر المثبتة والموسومة والملفات التي أضفتها.
settings-history-items = { $size } عنصر
settings-delete-older = حذف العناصر الأقدم من
settings-delete-older-hint = تحذف العناصر الأقدم من المدة التي تختارها
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
settings-data-folder-hint = يُحفظ السجل والصور والنماذج والإعدادات هنا، على هذا الجهاز فقط
settings-open = فتح
settings-log-file = ملف السجل
settings-log-file-hint = سجل بما فعله التطبيق في هذه الجلسة
settings-log-file-help = أرفقه عند الإبلاغ عن مشكلة.
settings-bug-report = الإبلاغ عن خطأ
settings-bug-report-hint = ينسخ الإصدار وإصدار ويندوز وإعداداتك دون العناوين والمفاتيح، وآخر سطور السجل
settings-bug-report-copy = انسخ التفاصيل
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
settings-clear-history = مسح سجل الحافظة
settings-clear-history-hint = يحذف العناصر المنسوخة
settings-clear-history-help = تبقى العناصر المثبتة والموسومة والملفات التي أضفتها.
settings-delete-items =
    { $count ->
        [one] حذف عنصر واحد
        [two] حذف عنصرين
        [few] حذف { $count } عناصر
       *[other] حذف { $count } عنصر
    }
settings-clear = مسح…
settings-reset-all = إعادة ضبط كل الإعدادات
settings-reset-all-hint = يعيد كل إعداد إلى قيمته الافتراضية. تبقى خوادمك المحفوظة وقواعدك
settings-reset-everything = إعادة ضبط الكل
settings-reset = إعادة ضبط…
settings-strictness-all = إظهار الكل
settings-strictness-related = المرتبطة فقط
settings-strictness-close = القريبة فقط
settings-strictness-custom = مخصص ({ $value })
settings-search = البحث
settings-search-mode = طريقة البحث
settings-search-mode-hint = كيف يجد البحث عناصرك
settings-search-mode-help =
    ذكي: يجد العناصر التي تحتوي كلماتك والعناصر التي تحمل المعنى نفسه في قائمة واحدة.
    بالمعنى: بالمعنى فقط (يحتاج نموذج بحث).
    الكلمات نفسها: العناصر التي تحتوي ما تكتبه فقط.
settings-strictness = نتائج المعنى المعروضة
settings-strictness-hint = مدى التطابق المطلوب لعرض العنصر
settings-strictness-help = «إظهار الكل» يرتّب كل شيء حسب القرب، والخيارات الأشد تخفي التطابقات الضعيفة. تظهر مطابقات الكلمات نفسها دائمًا.
settings-meaning-section = البحث بالمعنى
settings-read-section = قراءة النص في الصور
settings-servers-section = الخوادم
settings-engine = المحرك
settings-search-engine-hint = أين يعمل البحث بالمعنى
settings-search-engine-help = معطّل: البحث بالكلمات فقط، مع النص الموجود في الصور. خادمك: نموذج على أحد خوادمك يجد العناصر بالمعنى أيضًا.
settings-read-engine-hint = ما يقرأ النص في الصور، لتُوجد لقطات الشاشة والصور بكلماتها
settings-read-engine-help = يقرأ Windows اللغات المثبتة فيه، ومنها العربية إن كانت مثبتة. خادمك: نموذج رؤية على أحد خوادمك.
settings-engine-off = معطّل
settings-engine-windows = Windows
settings-engine-server = خادمك
settings-model = النموذج
settings-search-model-hint = النموذج الذي يستخدمه البحث بالمعنى
settings-search-model-help = يُطبَّق غيره فورًا، ولكل نموذج فهرسه، فلا تحتاج العودة إلى نموذج فهرسةً جديدة.
settings-read-model-hint = نموذج الرؤية الذي يقرأ النص في الصور
settings-read-model-help = يقرأ الصور التي تُنسخ من الآن، وتبقى الصور المقروءة بنصها.
settings-model-choose = اختر نموذجًا
settings-model-on-server = { $model } · { $server }
settings-model-add-server = إضافة خادم…
settings-model-none-search = لا يوجد نموذج بحث بعد. أضف خادمًا، ثم نموذج بحث عليه.
settings-model-none-read = لا يوجد نموذج قراءة بعد. أضف خادمًا، ثم نموذج قراءة عليه.
settings-model-off-note = البحث بالكلمات فقط. فعّل نموذجًا على خادم لتجد العناصر بالمعنى أيضًا، مثلًا «رحلة» تجد «طيران إلى جدة».
settings-read-off-note = لا يُقرأ النص في الصور، لذا تُوجد الصور بالمعنى فقط، إن كان نموذج البحث يقبل الصور.
settings-read-windows-note = يستخدم اللغات المثبتة في Windows.
settings-servers-none = لا توجد خوادم بعد. أضف الخادم الذي تعمل عليه نماذجك: llama.cpp أو Ollama أو LM Studio أو vLLM أو Unsloth أو OpenRouter أو غيرها.
settings-server-add = إضافة خادم
settings-server-edit = تعديل
settings-server-not-saved = لم يُحفظ
settings-server-discard = تجاهل التغييرات
settings-server-models-count =
    { $count ->
        [one] نموذج واحد
        [two] نموذجان
        [few] { $count } نماذج
       *[other] { $count } نموذج
    }
settings-server-back = العودة إلى البحث والذكاء الاصطناعي
settings-server-add-title = إضافة خادم
settings-server-edit-title = تعديل الخادم
settings-server-kind = النوع
server-kind-other = غير ذلك
settings-server-name = الاسم
settings-server-name-hint = ما تسمي به القوائم وشريط الحالة هذا الخادم
settings-server-test-connection = اختبار الاتصال
settings-server-found =
    { $count ->
        [one] متصل · وُجد نموذج واحد
        [two] متصل · وُجد نموذجان
        [few] متصل · وُجدت { $count } نماذج
       *[other] متصل · وُجد { $count } نموذج
    }
settings-server-needed = مطلوب
settings-server-models-section = النماذج
settings-server-no-models = لا توجد نماذج بعد. أضف التي تستخدمها على هذا الخادم.
settings-model-add-search = إضافة نموذج بحث
settings-model-add-read = إضافة نموذج قراءة
settings-model-job-search = بحث
settings-model-job-read = قراءة النص
settings-model-open = فتح إعدادات النموذج
settings-model-close = إغلاق إعدادات النموذج
settings-model-remove = إزالة هذا النموذج
settings-model-test = اختبار
settings-model-test-help = يرسل إلى النموذج نصًا قصيرًا للاختبار، وصورة اختبار إن كان يقرأ النص أو كان «إرسال الصور» مفعّلًا، ويعرض ما يعود. لا يُرسَل شيء من سجلك.
settings-model-test-search = يعمل · { $size } رقمًا في كل متجه · { $seconds } ث
settings-model-test-multi = يعمل · { $vectors } متجهات من { $size } رقمًا · { $seconds } ث
settings-model-test-read = قرأ: «{ $text }» · { $seconds } ث
settings-model-test-images = الصور تعمل
settings-model-test-no-images = لا يقبل الصور: «{ $said }». أوقف «إرسال الصور».
settings-model-test-images-failed = فشلت صورة الاختبار: { $error }
settings-model-on-the-server = النموذج على الخادم
settings-model-on-the-server-hint = اسم النموذج على الخادم
settings-model-on-the-server-help = اختر واحدًا مما يعرضه، أو اكتب اسمًا لا يعرضه. اتركه فارغًا لنموذج الخادم الافتراضي.
settings-model-name = الاسم في القوائم
settings-model-name-hint = اسم قصير للقوائم وشريط الحالة
settings-model-name-help = اتركه فارغًا ليظهر اسم النموذج.
settings-model-fill = املأ التفاصيل
settings-model-fill-help = يبحث عن النموذج في models.dev، وهي قائمة عامة لنماذج الذكاء الاصطناعي، ويملأ اسمه. وتُملأ أيضًا التعليمات والإعدادات التي تحتاجها النماذج المعروفة. لا يُرسَل شيء من سجلك.
settings-model-fill-looking = جارٍ البحث عنه في models.dev…
settings-model-fill-found = models.dev: { $name }، يأخذ حتى { $tokens } رمز.
settings-model-fill-found-no-limit = models.dev: { $name }.
settings-model-fill-not-found = لا يذكر models.dev هذا النموذج.
settings-model-fill-prompts = مُلئت التعليمات التي دُرِّب عليها هذا النموذج.
settings-model-fill-kind = مُلئت إعدادات { $model }.
settings-model-fill-images = يأخذ الصور أيضًا: فعّل «إرسال الصور» إن كان خادمك يأخذها.
settings-model-fill-no-images = لا يأخذ الصور، لذا لا يستطيع قراءة النص فيها.
settings-model-fill-needs-name = اكتب اسم النموذج أولًا.
settings-model-prompts-tip = هذا النموذج يبحث أفضل بتعليماته الخاصة
settings-server-in-use-removed = نموذج مستخدم لم يعد على هذا الخادم: الحفظ يوقف عمله.
settings-server-send-and-save = إرسال وحفظ
settings-server-save = حفظ
settings-server-delete = حذف الخادم
settings-server-delete-question = هل تحذف { $name } ونماذجه؟
settings-server-delete-confirm = حذف
settings-server-needs-address = اكتب عنوان الخادم لحفظه.
settings-vector-all = كامل
settings-server-address = عنوان الخادم
settings-server-address-hint = عنوان الخادم، ويكون عادةً منتهيًا بـ ‎/v1
settings-server-address-help = غيّر المنفذ إن كان خادمك يستخدم غيره.
settings-server-remote = يُرسَل ما تنسخه إلى { $host } لفهرسته، لأن هذا الخادم ليس على هذا الجهاز.
settings-server-remote-http = يُرسَل ما تنسخه إلى { $host } لفهرسته، لأن هذا الخادم ليس على هذا الجهاز. العنوان يبدأ بـ http، لذا يُنقل دون تشفير.
settings-server-send-question = هل تُرسل سجلك كله إلى { $host } لفهرسته؟ هذا الخادم ليس على هذا الجهاز.
settings-server-send = إرسال
settings-server-send-cancel = إلغاء
settings-server-key = مفتاح API
settings-server-key-hint = فقط إن طلبه خادمك
settings-server-key-help = يُرسل كرمز Bearer ويُحفظ على هذا الجهاز فقط.
settings-server-hide-key = إخفاء المفتاح
settings-server-show-key = إظهار المفتاح
settings-server-optional = اختياري
settings-server-models = النماذج التي يعرضها الخادم
settings-server-default = الافتراضي للخادم
settings-server-default-hint = الافتراضي: { $model }
settings-server-checking = جارٍ التحقق من الخادم…
settings-server-launch-note = يشغّل التطبيق llama-server بنفسه حين يُستخدم نموذج من هذا الخادم.
settings-server-use-them = استخدمها
settings-server-query-prompt = عبارة الاستعلام
settings-server-query-prompt-hint = نص يوضع قبل ما تبحث عنه
settings-server-query-prompt-help = تحتاجه بعض النماذج (EmbeddingGemma: «task: search result | query: »).
settings-server-none = لا شيء
settings-server-item-prompt = عبارة العنصر
settings-server-item-prompt-hint = نص يوضع قبل كل عنصر عند فهرسته (EmbeddingGemma: «title: none | text: »)
settings-server-vector-size = حجم المتجه
settings-server-vector-size-hint = كم رقمًا يُبقى من كل متجه
settings-server-vector-size-help = الأقل يستهلك ذاكرة أقل ويبحث أسرع، بجودة أقل قليلًا؛ ولا تبقى جيدة بعد التقصير إلا نماذج Matryoshka مثل EmbeddingGemma وnomic-embed وtext-embedding-3 من OpenAI. يعرض الاختبار كم رقمًا يعطي النموذج.
settings-server-images = إرسال الصور
settings-server-images-hint = أرسل الصور المنسوخة أيضًا، للنماذج التي تفهم الصور
settings-server-images-help = مثل CLIP وSigLIP وBiUltraVBERT، أو EmbeddingGemma 2 مع ملف mmproj، على خادم يأخذ الصور. وإلا فتُوجد الصور بالنص الذي فيها.
settings-server-launch = شغّل llama-server نيابةً عني
settings-server-launch-hint = يشغّل التطبيق llama-server بنموذجك، من ملف أو باسمه على Hugging Face، في الخلفية ويوقفه عند الخروج
settings-server-hf = أو اسم النموذج على Hugging Face
settings-server-hf-hint = مثل ggml-org/GLM-OCR-GGUF: ينزّله llama-server في أول مرة، ويُستخدم بدل الملف
settings-server-one-model = يشغّل llama-server نموذجًا واحدًا فقط. لتشغيل نموذج للبحث ونموذج للقراءة، احفظهما خادمَي llama.cpp منفصلين، لكل منهما منفذه.
settings-server-program = برنامج llama-server
settings-server-program-hint = الملف llama-server.exe من إصدار llama.cpp
settings-server-model-file = ملف النموذج (‎.gguf)
settings-server-projector = مُسقِط الصور (‎.gguf)
settings-server-projector-hint = ملف mmproj اختياري، للنماذج التي تضمّن الصور
settings-server-port = المنفذ
settings-server-extra-args = وسائط إضافية
settings-server-extra-args-hint = تُمرَّر إلى llama-server كما هي
settings-server-extra-args-help = يحتاج نموذج البحث عادةً إلى ‎--pooling mean. ويحتاج نموذج القراءة إلى مساحة للصورة ونصها، مثل ‎-c 8192، ودون ‎--pooling.
settings-shortcuts-global = في أي مكان في ويندوز
settings-shortcut-popup = فتح النافذة السريعة
settings-shortcut-popup-hint = نافذة صغيرة عند مؤشر الماوس: ابحث، ثم الصق في البرنامج الذي أنت فيه
settings-shortcut-popup-help = اضغط مرة أخرى لإغلاقها.
settings-shortcut-window = فتح النافذة الكاملة
settings-shortcut-window-hint = النافذة التي فيها كل شيء: عوامل التصفية والتفاصيل والإعدادات
settings-shortcut-window-help = اضغط مرة أخرى لإخفائها.
settings-shortcut-ignore = لا تسجّل النسخة التالية
settings-shortcut-ignore-hint = اضغطه قبل نسخ شيء خاص، فتُتجاهل النسخة التالية
settings-shortcut-not-usable = غير صالح
settings-shortcut-taken = يستخدمه تطبيق آخر
settings-shortcuts-local = داخل التطبيق
settings-shortcut-paste = لصق العنصر المحدد في البرنامج الذي كنت تستخدمه
settings-shortcut-paste-hint = يعمل هذا إذا فُتحت النافذة بالاختصار
settings-shortcut-paste-help = وفي غير ذلك، ومع البرامج التي تعمل كمسؤول، يُنسخ العنصر فقط.
settings-shortcut-paste-plain = لصق العنصر المحدد نصًا عاديًا دون تنسيق
settings-shortcut-copy-only = نسخ العنصر المحدد دون لصقه
settings-shortcut-pin = تثبيت أو إلغاء التثبيت
settings-shortcut-delete = حذف
settings-shortcut-tag = إضافة وسم
settings-shortcut-view = التبديل بين الشبكة والقائمة والتفاصيل
settings-shortcut-used-twice = مستخدم مرتين
settings-shortcut-move = التنقل في القائمة
settings-shortcut-move-grid = التنقل بين البطاقات في الشبكة
settings-shortcut-quick-paste = لصق أحد العناصر التسعة الأولى
settings-shortcut-menu = فتح قائمة العنصر المحدد
settings-shortcut-keys = عرض كل المفاتيح
settings-shortcut-escape = إغلاق التفاصيل أو مسح البحث أو الإخفاء
settings-shortcut-built-in = مدمج، لا يمكن تغييره
settings-shortcuts-note = انقر اختصارًا ثم اضغط المفاتيح الجديدة. Backspace للتعطيل، وEsc للإلغاء.

## Errors, statuses, messages and the tray icon

error-saving-item = تعذّر حفظ العنصر: { $error }
error-saving-image = تعذّر حفظ الصورة: { $error }
error-saving-settings = تعذّر حفظ الإعدادات: { $error }
error-deleting = تعذّر الحذف: { $error }
error-copying = تعذّر النسخ: { $error }
error-opening-folder = تعذّر فتح المجلد: { $error }
error-background-crash = توقفت مهمة في الخلفية بسبب خطأ. التفاصيل في ملف السجل.
error-tray-unavailable = أيقونة علبة النظام غير متاحة: { $error }
error-start-at-sign-in = تعذّر تغيير «التشغيل عند تسجيل الدخول»: { $error }
error-hotkeys-unavailable = الاختصارات التي تعمل في أي مكان في ويندوز غير متاحة: { $error }
error-hotkey-taken = الاختصار { $keys } يستخدمه تطبيق آخر أو ويندوز. اختر مفاتيح أخرى في الإعدادات، الاختصارات واللصق.
error-hotkey-invalid = لا يمكن استخدام الاختصار { $keys }: { $error }
error-indexing-skipped = تعذّرت فهرسة عنصر، فتم تخطيه.
error-indexing = تعذّرت فهرسة العناصر للبحث بالمعنى: { $error }
error-query-embedding = تعذّر البحث بالمعنى: { $error }
error-model = تعذّر تحميل نموذج البحث: { $error }
error-model-test = فشل الاختبار: { $error }
error-models-dev = تعذّر البحث عن النموذج في models.dev: { $error }
error-ocr-unavailable = تعذّرت قراءة النص في الصور: { $error }
error-reader-server-stopped = توقف llama-server الخاص بقراءة النص أثناء بدئه ({ $status }). تحقق من ملف النموذج أو اسمه على Hugging Face، ومن الوسائط الإضافية.
error-server-unreachable = تعذّر الوصول إلى الخادم على { $host }. تأكد من أنه يعمل ومن صحة عنوانه.
error-server-needs-key = يحتاج الخادم إلى مفتاح API، أو أن المفتاح غير صحيح. راجع المفتاح في إعدادات الخادم.
error-server-not-found = لا يعرف الخادم هذا العنوان أو هذا النموذج: «{ $said }»
error-server-refused = رفض الخادم الطلب: «{ $said }»
error-server-failed = فشل الخادم (الخطأ { $status }): «{ $said }»
error-server-no-images = لا يقبل الخادم الصور: «{ $said }»
error-server-no-models = لا يقدّم الخادم أي نماذج.
error-server-bad-answer = جاء رد الخادم على غير ما يتوقعه التطبيق.
error-settings-unreadable = تعذّرت قراءة الإعدادات ({ $error }). يستخدم التطبيق الآن الإعدادات الافتراضية. حُفظ الملف القديم باسم { $copy }
error-settings-unreadable-no-copy = تعذّرت قراءة الإعدادات ({ $error }). يستخدم التطبيق الآن الإعدادات الافتراضية. وتعذّر نسخ الملف القديم أيضًا: { $reason }
error-history-moved-aside = تعذّر فتح سجلك، فبدأ سجل جديد فارغ. حُفظ السجل القديم باسم { $file }
error-start-failed = تعذّر تشغيل مدير الحافظة.
status-starting-llama = جارٍ تشغيل llama-server…
status-reader-starting = جارٍ تشغيل llama-server لقراءة النص في الصور…
status-reader-downloading = جارٍ تشغيل llama-server لقراءة النص في الصور؛ ينزّل النموذج في أول مرة…
status-connecting = جارٍ الاتصال بـ { $host }…
toast-copied = تم النسخ
toast-copied-admin = تم النسخ. ذلك البرنامج يعمل كمسؤول، لذا لا يسمح Windows لهذا التطبيق باللصق فيه. اضغط Ctrl+V هناك.
toast-copied-text = تم نسخ النص
toast-saved = تم الحفظ
toast-deleted = تم الحذف
toast-updated = تم التحديث إلى الإصدار { $version }
toast-pinned-group =
    { $count ->
        [one] تم تثبيت عنصر واحد
        [two] تم تثبيت عنصرين
        [few] تم تثبيت { $count } عناصر
       *[other] تم تثبيت { $count } عنصر
    }
toast-unpinned-group =
    { $count ->
        [one] تم إلغاء تثبيت عنصر واحد
        [two] تم إلغاء تثبيت عنصرين
        [few] تم إلغاء تثبيت { $count } عناصر
       *[other] تم إلغاء تثبيت { $count } عنصر
    }
toast-deleted-group =
    { $count ->
        [one] تم حذف عنصر واحد
        [two] تم حذف عنصرين
        [few] تم حذف { $count } عناصر
       *[other] تم حذف { $count } عنصر
    }
toast-copied-group =
    { $count ->
        [one] تم نسخ عنصر واحد
        [two] تم نسخ عنصرين معًا
        [few] تم نسخ { $count } عناصر معًا
       *[other] تم نسخ { $count } عنصر معًا
    }
toast-copied-group-pictures =
    { $count ->
        [one] تم نسخ عنصر واحد
        [two] تم نسخ عنصرين معًا
        [few] تم نسخ { $count } عناصر معًا
       *[other] تم نسخ { $count } عنصر معًا
    }، دون { $pictures ->
        [one] صورة واحدة
        [two] صورتين
        [few] { $pictures } صور
       *[other] { $pictures } صورة
    }
toast-pictures-not-together = لا يمكن نسخ الصور معًا
toast-tag-added-group =
    { $count ->
        [one] تمت إضافة #{ $tag } إلى عنصر واحد
        [two] تمت إضافة #{ $tag } إلى عنصرين
        [few] تمت إضافة #{ $tag } إلى { $count } عناصر
       *[other] تمت إضافة #{ $tag } إلى { $count } عنصر
    }
toast-deleted-key = تم الحذف. في المرة القادمة: { $keys }
toast-copied-key = تم النسخ. في المرة القادمة: { $keys }
toast-pinned-key = تم التثبيت. في المرة القادمة: { $keys }
toast-unpinned-key = تم إلغاء التثبيت. في المرة القادمة: { $keys }
toast-imported =
    { $count ->
        [one] تم استيراد ملف واحد
        [two] تم استيراد ملفين
        [few] تم استيراد { $count } ملفات
       *[other] تم استيراد { $count } ملف
    }
toast-pruned-age =
    { $count ->
        [one] حُذف عنصر واحد أقدم من { $age }
        [two] حُذف عنصران أقدم من { $age }
        [few] حُذف { $count } عناصر أقدم من { $age }
        [many] حُذف { $count } عنصرًا أقدم من { $age }
       *[other] حُذف { $count } عنصر أقدم من { $age }
    }
toast-pruned-limit =
    { $count ->
        [one] حُذف عنصر واحد تجاوز حد السجل
        [two] حُذف عنصران تجاوزا حد السجل
        [few] حُذف { $count } عناصر تجاوزت حد السجل
        [many] حُذف { $count } عنصرًا تجاوز حد السجل
       *[other] حُذف { $count } عنصر تجاوز حد السجل
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
toast-skipped-private = لم يُحفظ: وُسم بأنه خاص
toast-skipped-by = وسمه { $program }
toast-skipped-from = من { $program }
toast-skipped-password = لم يُحفظ: كلمة مرور
toast-skipped-api-key = لم يُحفظ: مفتاح API
toast-skipped-sign-in = لم يُحفظ: رمز تسجيل دخول
toast-skipped-private-key = لم يُحفظ: مفتاح خاص
toast-skipped-link = لم يُحفظ: رابط فيه كلمة مرور
toast-skipped-key = لم يُحفظ: مفتاح
toast-hint-characters =
    { $count ->
        [one] حرف واحد
        [two] حرفان
        [few] { $count } أحرف
        [many] { $count } حرفًا
       *[other] { $count } حرف
    }
toast-skipped-rule = لم يُحفظ: إحدى قواعدك تتخطاه
toast-app-ignored = لن تُسجَّل النسخ من { $app }. يمكنك حذف القاعدة من الإعدادات، الخصوصية.
tray-show = إظهار مدير الحافظة
tray-pause-recording = إيقاف التسجيل مؤقتًا
tray-resume-recording = استئناف التسجيل
tray-quit = خروج
tray-tooltip-paused = مدير الحافظة · التسجيل متوقف
