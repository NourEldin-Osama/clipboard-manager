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
    اضغط { $key } لفتح هذه النافذة من أي مكان.
list-no-matches = لا نتائج لـ «{ $query }»
list-meaning-finds-more = البحث بالمعنى يجد الكلمات المرتبطة أيضًا.
list-set-up-meaning = فعّل البحث بالمعنى
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
status-model-server-default = { $server } (النموذج الافتراضي)
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
details-delete = حذف
details-unpin = إلغاء التثبيت
details-pin-keep = تثبيت: احتفظ به دائمًا
details-save-image = حفظ الصورة باسم…
details-show-in-folder = إظهار في المجلد
details-open-in-browser = فتح في المتصفح
details-copy = نسخ
details-copy-tip = { $action } · أو انقر نقرًا مزدوجًا في القائمة
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
details-key-copy = نسخ وإخفاء
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
