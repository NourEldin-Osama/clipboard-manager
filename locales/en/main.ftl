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
    Press { $key } anywhere to pick a copy and paste it.
list-no-matches = No matches for “{ $query }”
list-meaning-finds-more = Searching by meaning finds related words too.
list-set-up-meaning = Set up meaning search
popup-open-window = Open the full window
popup-close = Close
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
status-indexing = Indexing · { $done } of { $total }
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
details-formatted = With formatting
details-copied-from = Copied from { $app }
details-ignore-app = Don't record copies from { $app }
details-delete = Delete
details-unpin = Unpin
details-pin-keep = Pin: keep forever
details-save-image = Save image as…
details-show-in-folder = Show in folder
details-open-in-browser = Open in browser
details-copy = Copy
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
details-key-paste = paste
details-key-paste-plain = paste as plain text
details-key-copy-only = copy
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

## Settings

settings-page-general = General
settings-page-search = Search & AI model
settings-page-clipboard = Clipboard
settings-page-shortcuts = Shortcuts
settings-page-appearance = Appearance
settings-page-data = Data & privacy
app-title = Clipboard Manager
settings-back = Back (Esc)
settings-title = Settings
settings-apply-model = Apply model
settings-discard-changes = Discard changes
settings-model-not-applied = Model changes not applied yet
settings-saved-automatically = Changes are saved automatically
settings-back-to-default = Back to default
settings-browse = Browse…
settings-filter-matching = Matching files
settings-filter-all = All files
settings-language-and-time = Language and time
settings-language = Language
settings-language-help = The language of the app. Arabic also lays the window out right to left.
settings-time-format = Time format
settings-time-format-help = How times are shown. They're stored in UTC and shown in your time zone.
settings-time-24h = 24-hour
settings-time-12h = 12-hour
settings-startup = Startup
settings-start-at-sign-in = Start when you sign in
settings-start-at-sign-in-help = Opens the app each time you sign in to Windows. It starts hidden: in the system tray when that's on, otherwise minimised.
settings-tray-section = System tray
settings-tray = Keep running in the system tray
settings-tray-help = Shows an icon in the notification area. Closing or minimising the window hides it there and history keeps recording. Click the icon to open; right-click it to quit.
settings-startup-note = With both on, the app starts quietly in the tray each time you sign in.
settings-updates = Updates
settings-update-check = Check for updates automatically
settings-update-check-help = Looks for a new version when the app starts and once a day. Only the version file on GitHub is fetched; nothing about you or your clipboard is sent.
settings-update-download = Download updates automatically
settings-update-download-help = On: a new version downloads in the background and installs when you restart or quit. Off: you're asked before anything is downloaded.
settings-version = Version
settings-version-help = Every download is checked against this app's signing key before it's installed.
settings-check-now = Check now
settings-update-checking = Checking…
settings-update-current = You're up to date
update-ready = Version { $version } is ready
settings-updates-off-dev = Updates are off in development builds
update-development-build = { $version } (development build)
settings-look = Look
settings-theme = Theme
settings-theme-help = System follows your Windows light/dark setting.
settings-theme-system = System
settings-theme-light = Light
settings-theme-dark = Dark
settings-accent = Accent colour
settings-accent-help = Used for selection, buttons and highlights.
accent-indigo = Indigo
accent-sky = Sky
accent-emerald = Emerald
accent-amber = Amber
accent-rose = Rose
settings-list = List
settings-layout = Layout
settings-layout-help = Grid shows cards, List shows rows; both open details on demand. Detail keeps the selected item beside the list. The view button next to the search box switches too.
settings-density = Density
settings-density-help = Compact fits more items on screen by hiding the second line.
settings-density-comfortable = Comfortable
settings-density-compact = Compact
settings-pinned-on-top = Pinned on top
settings-pinned-on-top-help = Pinned items come first while you browse, whatever the order. Search results are ordered by your sort choice only.
settings-group-by-day = Group by day
settings-group-by-day-help = Show Pinned, Today, Yesterday… headers while browsing (not while searching).
settings-recording = Recording
settings-record = Record clipboard history
settings-record-help = Save everything you copy. Turn off to pause; the pause button in the main window does the same.
settings-skip-secrets = Skip passwords and API keys
settings-skip-secrets-help = Don't save copies that look like a password, API key, token or private key. You still can paste them normally.
settings-merge-duplicates = Merge duplicate copies
settings-merge-duplicates-help = Copying something you already have moves it to the top instead of adding it again.
settings-read-images = Read text in images
settings-read-images-help = Find screenshots and photos by the words in them.
settings-reader = Read text with
settings-reader-help = Windows reads the languages installed in Windows, Arabic included when it is there. The built-in reader reads Latin letters only and downloads 12 MB first. A server runs a vision model you choose.
settings-reader-windows = Windows
settings-reader-built-in = Built-in
settings-reader-server = A server
settings-reader-address-help = The address of a server with a vision model, usually ending in /v1. Images go to its /chat/completions.
settings-reader-model-help = Leave empty to use the server's default model.
settings-reader-prompt = Prompt
settings-reader-prompt-help = What the model is asked with each image. Some models want their own words: "Free OCR." for DeepSeek-OCR, "OCR:" for PaddleOCR-VL.
settings-reader-remote = The images you copy are sent to { $host } to be read, because this server is not on this PC.
settings-reader-remote-http = The images you copy are sent to { $host } to be read, because this server is not on this PC. The address starts with http, so they travel without encryption.
settings-reader-send-question = Send the images you copy to { $host } to be read? This server is not on this PC.
settings-reader-apply = Apply
settings-reader-new-images = Images already read keep their text; the new reader reads the rest.
settings-after-copying = After copying from the list
settings-hide-window = Hide the window
settings-hide-window-help = When an item is copied without pasting it, the window hides so you can paste it yourself. Pasting always hides the window.
settings-history-size = History size
settings-keep-up-to = Keep up to
settings-keep-up-to-help = When there are more, the oldest are deleted. Pinned and tagged items, and files you added, are never deleted.
settings-history-items = { $size } items
settings-delete-older = Delete items older than
settings-delete-older-help = Off unless you choose a time. Older items are deleted when you leave Settings, then every hour. Pinned and tagged items, and files you added, are never deleted.
settings-age-off = Off
settings-age-day = 1 day
settings-age-week = 1 week
settings-age-month = 1 month
settings-age-three-months = 3 months
settings-age-year = 1 year
settings-age-days =
    { $count ->
        [one] 1 day
       *[other] { $count } days
    }
settings-rules = Rules
settings-rules-help = Copies a rule matches are not recorded. A program is its file name, such as chrome.exe; leave it empty for every program. Leave the pattern empty to skip everything from the program.
settings-rule-program = Program
settings-rule-pattern = Pattern
settings-rule-regex = Regular expression
settings-rule-regex-help = Off: * stands for any text and ? for one character, and the pattern must match the whole copy, so *word* finds a word anywhere. On: a regular expression, found anywhere in the copy.
settings-rule-delete = Delete this rule
settings-rule-add = Add a rule
settings-rule-needs = Give a program, a pattern, or both. Until then this rule does nothing.
settings-rule-bad-pattern = This regular expression can't be used, so the rule does nothing: { $error }
settings-storage = Storage
settings-data-folder = Data folder
settings-data-folder-help = History, images, models and settings are stored here, on this PC only.
settings-open = Open
settings-log-file = Log file
settings-log-file-help = A record of what the app did in this session. Attach it when reporting a problem.
settings-show = Show
settings-stored = Stored
settings-stored-counts =
    { $count ->
        [one] 1 item
       *[other] { $count } items
    } · { $images ->
        [one] 1 image
       *[other] { $images } images
    } · { $pinned } pinned
settings-clean-up = Clean up
settings-clear-history = Clear clipboard history
settings-clear-history-help = Deletes copied items. Pinned and tagged items, and files you added, stay.
settings-delete-items =
    { $count ->
        [one] Delete 1 item
       *[other] Delete { $count } items
    }
settings-clear = Clear…
settings-reset-all = Reset all settings
settings-reset-all-help = Puts every setting back to its default, including the search model.
settings-reset-everything = Reset everything
settings-reset = Reset…
settings-privacy-note = Nothing you copy is sent anywhere. Search models run on this PC or on a server you choose. The update check only downloads the version file from GitHub.
settings-privacy-note-remote = What you copy is sent to { $host } to be indexed, because the search model runs on that server. Nothing else is sent anywhere, and the update check only downloads the version file from GitHub.
settings-privacy-note-reader-remote = The images you copy are sent to { $host } to be read, because the text reader runs on that server. Nothing else is sent anywhere, and the update check only downloads the version file from GitHub.
settings-privacy-note-remote-both = What you copy is sent to { $search } to be indexed, and the images you copy to { $reader } to be read, because those servers are not on this PC. Nothing else is sent anywhere, and the update check only downloads the version file from GitHub.
settings-privacy-encrypted = Your history is encrypted on this PC: only your Windows account can open it.
settings-strictness-all = Show all
settings-strictness-related = Related only
settings-strictness-close = Close only
settings-strictness-custom = Custom ({ $value })
settings-search = Search
settings-search-mode = How search works
settings-search-mode-help =
    Smart: finds items that contain your words and items that mean the same thing, merged into one list.
    Meaning: only by meaning (needs a search model).
    Exact words: only items containing what you type.
settings-strictness = Meaning matches to show
settings-strictness-help = How closely an item has to match before it's listed. "Show all" ranks everything by closeness; stricter options hide weak matches. Exact word matches are always shown.
settings-model-section = Meaning search model
settings-model = Model
settings-model-help =
    Off: search by words only (plus text found in images).
    Built-in: a model inside this app that understands text and images, no other software needed (one-time download).
    Your server: use a model you already run, such as llama.cpp, Unsloth, Sentence Transformers, Ollama or LM Studio.
settings-model-off = Off
settings-model-builtin = Built-in
settings-model-server = Your server
settings-model-off-note = Searching by words only. Turn on a model to also find items by meaning, e.g. “trip” finds “flight to Jeddah”.
settings-model-reindex = Your items are indexed again in the background.
settings-builtin-note = Runs on this PC's processor. Finds photos and screenshots from a description, like “receipt from a restaurant”.
settings-model-download-size = { $repo } · { $size } GB download
settings-model-downloaded = Downloaded
settings-model-progress = { $done } / { $total } MB
settings-model-download = Download { $size } GB
settings-model-use = Use
model-siglip-multilingual = SigLIP base, multilingual (text + images)
model-siglip-english = SigLIP base, English (text + images)
settings-vector-full = Full · best quality
settings-vector-512 = 512 · smaller
settings-vector-256 = 256 · compact
settings-vector-128 = 128 · tiny, fastest
settings-server-quick-setup = Quick setup:
settings-server-address = Server address
settings-server-address-help = The address of the embeddings server, usually ending in /v1.
settings-server-remote = What you copy is sent to { $host } to be indexed, because this server is not on this PC.
settings-server-remote-http = What you copy is sent to { $host } to be indexed, because this server is not on this PC. The address starts with http, so it travels without encryption.
settings-server-send-question = Send your whole history to { $host } to be indexed? This server is not on this PC.
settings-server-send = Send
settings-server-send-cancel = Cancel
settings-server-check-again = Check again
settings-server-key = API key
settings-server-key-help = Only if your server asks for one. Sent as a Bearer token and stored only on this PC.
settings-server-hide-key = Hide key
settings-server-show-key = Show key
settings-server-optional = optional
settings-server-model = Model
settings-server-model-help = Leave empty to use the server's default embedding model. Pick from ▾ or type a name your server knows.
settings-server-models = Models on this server
settings-server-default = Server default
settings-server-default-named = Server default · { $model }
settings-server-listed = LISTED BY THE SERVER (MAY INCLUDE CHAT MODELS)
settings-server-default-hint = default: { $model }
settings-server-default-lower = server default
settings-server-checking = Checking the server…
settings-server-the-default-model = the default model
settings-server-connected-uses = Connected · uses { $model }
settings-server-no-default = Connected, but no default embedding model: choose one
settings-server-connected = Connected
settings-server-launch-note = The app starts llama-server itself; it connects after Apply.
settings-server-gemma-tip = EmbeddingGemma searches better with its own prompts
settings-server-use-them = Use them
settings-server-advanced = Advanced
settings-server-query-prompt = Query prompt
settings-server-query-prompt-help = Text put before what you search for. Some models need it (EmbeddingGemma: “task: search result | query: ”).
settings-server-none = none
settings-server-item-prompt = Item prompt
settings-server-item-prompt-help = Text put before each item when it's indexed (EmbeddingGemma: “title: none | text: ”).
settings-server-vector-size = Vector size
settings-server-vector-size-help = Smaller vectors use less memory and search faster, with slightly lower quality. Works with Matryoshka models such as EmbeddingGemma and UltraVBERT.
settings-vector-custom = { $count } numbers
settings-server-images = Search images with this model
settings-server-images-help = Send images to the server too. Only for models that understand images (CLIP, SigLIP, BiUltraVBERT…). Otherwise images are found by the text in them.
settings-server-launch = Start llama-server for me
settings-server-launch-help = The app starts llama-server with your .gguf file in the background and stops it when you quit.
settings-server-program = llama-server program
settings-server-program-help = llama-server.exe from a llama.cpp release.
settings-server-model-file = Model file (.gguf)
settings-server-projector = Image projector (.gguf)
settings-server-projector-help = Optional mmproj file, for models that embed images.
settings-server-port = Port
settings-server-extra-args = Extra arguments
settings-server-extra-args-help = Passed to llama-server as-is.
settings-shortcuts-global = Anywhere in Windows
settings-shortcut-popup = Open the quick popup
settings-shortcut-popup-help = A small window at the mouse pointer: search, then paste into the program you are in. Press again to close it.
settings-shortcut-window = Open the full window
settings-shortcut-window-help = The window with everything: filters, details and settings. Press again to hide it.
settings-shortcut-ignore = Don't record the next copy
settings-shortcut-ignore-help = Press before copying something private: the next copy is skipped.
settings-shortcut-not-usable = Not usable
settings-shortcut-taken = Used by another app
settings-shortcuts-local = Inside the app
settings-shortcut-paste = Paste the selected item into the program you were using
settings-shortcut-paste-help = This works when the window was opened with the shortcut. Otherwise, and for programs that run as administrator, the item is only copied.
settings-shortcut-paste-plain = Paste the selected item as plain text, without formatting
settings-shortcut-copy-only = Copy the selected item without pasting it
settings-shortcut-pin = Pin or unpin
settings-shortcut-delete = Delete
settings-shortcut-tag = Add a tag
settings-shortcut-view = Switch between Grid, List and Detail
settings-shortcut-used-twice = Used twice
settings-shortcut-move = Move through the list
settings-shortcut-move-grid = Move between cards in the grid
settings-shortcut-escape = Close details, clear the search, or hide
settings-shortcut-built-in = Built in; can't be changed.
settings-shortcuts-note = Click a shortcut, then press the new keys. Backspace turns it off, Esc cancels.

## Errors, statuses, messages and the tray icon

error-saving-item = Could not save the item: { $error }
error-saving-image = Could not save the image: { $error }
error-saving-settings = Could not save the settings: { $error }
error-deleting = Could not delete: { $error }
error-copying = Could not copy: { $error }
error-opening-folder = Could not open the folder: { $error }
error-background-crash = A background task stopped because of an error. The log file has the details.
error-download-failed = The download failed: { $error }
error-download-crashed = The download stopped because of an error. The log file has the details.
error-tray-unavailable = The system tray icon is not available: { $error }
error-start-at-sign-in = Could not change “Start when you sign in”: { $error }
error-hotkeys-unavailable = Shortcuts that work anywhere in Windows are not available: { $error }
error-hotkey-taken = { $keys } is already used by another app or by Windows. Choose other keys in Settings, Shortcuts.
error-hotkey-invalid = Cannot use the shortcut { $keys }: { $error }
error-indexing-skipped = Indexing failed on an item; it was skipped.
error-query-embedding = Could not search by meaning: { $error }
error-model = Could not load the search model: { $error }
error-model-not-downloaded = { $model } is not downloaded yet. Download it in Settings, Search & AI model.
error-ocr-unavailable = Cannot read text in images: { $error }
error-settings-unreadable = Could not read the settings ({ $error }). The app now uses the default settings. The old file is saved as { $copy }
error-settings-unreadable-no-copy = Could not read the settings ({ $error }). The app now uses the default settings. The old file could not be copied either: { $reason }
error-history-moved-aside = Your history couldn't be opened, so a new, empty one started. The old one was kept as { $file }
error-start-failed = Clipboard Manager could not start.
status-loading-model = Loading { $model }…
status-starting-llama = Starting llama-server…
status-connecting = Connecting to { $host }…
status-downloading-ocr = Downloading OCR models (12 MB, one time)…
toast-copied = Copied
toast-copied-admin = Copied. That program runs as administrator, so Windows doesn't let this app paste into it. Press Ctrl+V there.
toast-copied-text = Copied text
toast-saved = Saved
toast-deleted = Deleted
toast-imported =
    { $count ->
        [one] Imported 1 file
       *[other] Imported { $count } files
    }
toast-cleared =
    { $count ->
        [one] Cleared 1 item
       *[other] Cleared { $count } items
    }
toast-tag-removed =
    { $count ->
        [one] Removed #{ $tag } from 1 item
       *[other] Removed #{ $tag } from { $count } items
    }
toast-downloaded = Downloaded { $model }
toast-skipped-private = Not saved: the app you copied from marked it as private
toast-skipped-secret = Not saved: looks like a password or API key (see Settings)
toast-skipped-rule = Not saved: one of your rules skips it
toast-app-ignored = Copies from { $app } won't be recorded. You can remove the rule in Settings, Clipboard.
tray-show = Show Clipboard Manager
tray-pause-recording = Pause recording
tray-resume-recording = Resume recording
tray-quit = Quit
