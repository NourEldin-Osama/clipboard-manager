### Clipboard Manager: every text of the window, in English.
### English is also the fallback: a text missing in another language shows in
### English. Keys say what a text is for, starting with the part of the window
### it belongs to. docs/TRANSLATING.md explains how to add a language.

## Times and dates

# A 12-hour clock time, such as "2:05 PM".
time-clock-am = { $time } AM
time-clock-pm = { $time } PM
time-just-now = just now
time-minutes-ago =
    { $count ->
        [one] 1 minute ago
       *[other] { $count } minutes ago
    }
time-hours-ago =
    { $count ->
        [one] 1 hour ago
       *[other] { $count } hours ago
    }
time-yesterday = yesterday
time-today-at = Today at { $time }
time-yesterday-at = Yesterday at { $time }
# $date is a whole date, such as "Mon 14 Oct 2026".
time-date-at = { $date } at { $time }
date-day-month = { $day } { $month }
date-day-month-year = { $day } { $month } { $year }
date-weekday-date = { $weekday } { $date }
weekday-mon = Mon
weekday-tue = Tue
weekday-wed = Wed
weekday-thu = Thu
weekday-fri = Fri
weekday-sat = Sat
weekday-sun = Sun
month-jan = Jan
month-feb = Feb
month-mar = Mar
month-apr = Apr
month-may = May
month-jun = Jun
month-jul = Jul
month-aug = Aug
month-sep = Sep
month-oct = Oct
month-nov = Nov
month-dec = Dec

## Day headers in the list, shown in capitals

group-pinned = Pinned
group-today = Today
group-yesterday = Yesterday
group-this-week = This week
group-this-month = This month
group-older = Older

## Names: sort orders, views, filters, item kinds, search modes

sort-best-match = Best match
sort-recent = Recent
sort-oldest = Oldest
sort-a-to-z = A to Z
sort-most-used = Most used
view-grid = Grid
view-list = List
view-detail = Detail
filter-all = All
filter-text = Text
filter-links = Links
filter-images = Images
filter-files = Files
filter-pinned = Pinned
kind-text = Text
kind-link = Link
kind-image = Image
kind-files = Files
item-image-size = Image { $width }×{ $height }
search-mode-smart = Smart
search-mode-meaning = Meaning
search-mode-words = Exact words
search-mode-smart-help = Exact words and meaning, merged (recommended)
search-mode-meaning-help = Only by meaning: "trip" finds "flight to Jeddah"
search-mode-words-help = Only items containing the words you type
top-view-tip = View: { $view } · click for { $next }
top-sort-by = Sort by
top-sort-by-heading = SORT BY
top-results =
    { $count ->
        [one] 1 result
       *[other] { $count } results
    }
list-empty-title = Your clipboard history will appear here
list-empty-help =
    Copy anything, or drop files here.
    Press { $key } to open this window from anywhere.
list-no-matches = No matches for “{ $query }”
list-meaning-finds-more = Searching by meaning finds related words too.
list-set-up-meaning = Set up meaning search
list-try-smart = Try “Smart” search instead
list-try-other-words = Try fewer or different words.
list-search-all-not-text = Search all items, not just text
list-search-all-not-links = Search all items, not just links
list-search-all-not-images = Search all items, not just images
list-search-all-not-files = Search all items, not just files
list-search-all-not-pinned = Search all items, not just pinned ones
list-none-yet = Nothing here yet
list-none-yet-text = No text yet
list-none-yet-links = No links yet
list-none-yet-images = No images yet
list-none-yet-files = No files yet
list-none-yet-pinned = No pinned items yet

## Main window

top-settings = Settings
top-add-files-tip = Add files or a folder to search (you can also drop them on the window)
top-add-files = Add files…
top-add-folder = Add a folder…
top-pause-recording = Pause recording
top-resume-recording = Resume recording
search-hint-images = Search text and images by meaning…
search-hint-meaning = Search by words or meaning…
search-hint = Search your clipboard…
search-clear = Clear (Esc)
tags-show-all = Show all tags again
tags-filter-tip = Only items with this tag · right-click for more
tags-remove-everywhere =
    { $count ->
        [one] Remove #{ $tag } from 1 item
       *[other] Remove #{ $tag } from all { $count } items
    }
tags-more =
    { $count ->
        [one] +1 tag
       *[other] +{ $count } tags
    }
update-version-label = { $version } (build { $build })
update-available = Version { $version } is available
update-verifying = Checking the download of version { $version }…
update-downloading = Downloading version { $version }…
update-ready-restart = Version { $version } is ready. Restart to update, or it installs when you quit.
update-failed = Update failed: { $error }
update-progress = { $done } / { $total } MB
update-later = Later
update-skip = Skip this version
update-download = Download
update-restart = Restart to update
update-try-again = Try again
update-whats-new = What's new
common-cancel = Cancel
status-model-loading = Loading search model…
status-model-builtin = Built-in · { $model }
status-model-server-default = { $server } (default model)
status-model-off = Words only · set up meaning search
status-model-unavailable = Search model unavailable · fix in Settings
status-model-words-only = Words only
status-model-tip = Search model: click to change
status-indexing = Indexing · { $count } left
status-adding-files = Adding files · { $count } added
status-recording-paused = Recording paused
status-shortcut-taken = Shortcut { $keys } is used by another app
status-open-shortcut-settings = Open the shortcut settings
status-dismiss-error = Click to dismiss
status-items =
    { $count ->
        [one] 1 item
       *[other] { $count } items
    }
status-items-pinned =
    { $count ->
        [one] 1 item
       *[other] { $count } items
    } · { $pinned } pinned
list-drop-files = Drop to add to your searchable library
item-files-count =
    { $count ->
        [one] 1 file
       *[other] { $count } files
    }
item-empty = (empty)
item-delete = Delete
item-pin = Pin
item-unpin = Unpin
item-copy = Copy
item-open-details = Open details
hint-with-keys = { $action } ({ $keys })
details-only-this-tag = Show only items with this tag
details-remove-tag = Remove tag
details-tag-hint = tag, another…
details-add-tag = Add tag
details-tag = Tag
details-add-tag-tip = Add a tag
details-close = Close details (Esc)
details-pinned = Pinned
details-from-files = From your files
details-delete = Delete
details-unpin = Unpin
details-pin-keep = Pin: keep forever
details-save-image = Save image as…
details-show-in-folder = Show in folder
details-open-in-browser = Open in browser
details-copy = Copy
details-copy-tip = { $action } · or double-click in the list
details-code-lines =
    { $count ->
        [one] 1 line of code
       *[other] { $count } lines of code
    }
details-characters =
    { $count ->
        [one] 1 character
       *[other] { $count } characters
    }
details-words =
    { $count ->
        [one] 1 word
       *[other] { $count } words
    }
details-image-size = { $width } × { $height } px
details-has-text = contains text
details-no-text = no text found
details-reading-text = reading text…
details-files-count =
    { $count ->
        [one] 1 item
       *[other] { $count } items
    }
details-image-missing = The image file is missing.
details-text-in-image = TEXT IN IMAGE
details-copy-text = Copy text
details-select-an-item = Select an item to see it here
details-key-move = move
details-key-copy = copy & hide
details-key-pin = pin
details-key-tag = tag
details-key-delete = delete
details-key-clear = clear / hide
shortcut-needs-modifier-global = Add Ctrl, Alt, Shift or Win
shortcut-needs-modifier = Add Ctrl, Alt or Shift
shortcut-press-keys = Press keys…
shortcut-off = Off · click to set
shortcut-recording-tip = Press the new keys · Backspace turns it off · Esc cancels
shortcut-click-to-set = Click, then press the new keys
toast-save-anyway = Save anyway
toast-undo = Undo
