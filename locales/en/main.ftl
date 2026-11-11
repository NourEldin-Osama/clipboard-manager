### Clipboard Manager: every text of the window, in English.
### English is also the fallback: a text missing in another language shows in
### English. Keys say what a text is for, starting with the part of the window
### it belongs to. docs/TRANSLATING.md explains how to add a language.

## Times and dates

# A 12-hour clock time, such as "2:05 PM".
time-clock-am = { $time } AM
time-clock-pm = { $time } PM
time-just-now = Just now
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
time-today-at = Today at { $time }
time-yesterday-at = Yesterday at { $time }
# $date is a whole date, such as "Monday 14 Oct 2026".
time-date-at = { $date } at { $time }
date-day-month = { $day } { $month }
date-day-month-year = { $day } { $month } { $year }
date-weekday-date = { $weekday } { $date }
weekday-mon = Monday
weekday-tue = Tuesday
weekday-wed = Wednesday
weekday-thu = Thursday
weekday-fri = Friday
weekday-sat = Saturday
weekday-sun = Sunday
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
item-image = Image
search-mode-smart = Smart
search-mode-meaning = Meaning
search-mode-words = Exact words
search-mode-smart-help = Exact words and similar meanings together (recommended)
search-mode-meaning-help = Only by meaning: "trip" finds "flight to Jeddah"
search-mode-words-help = Only items containing the words you type
top-sort-by = Sort by
top-sort-by-heading = SORT BY
top-results =
    { $count ->
        [one] 1 result
       *[other] { $count } results
    }
list-show-weaker =
    { $count ->
        [one] Show 1 weaker match
       *[other] Show { $count } weaker matches
    }
list-hide-weaker =
    { $count ->
        [one] Hide 1 weaker match
       *[other] Hide { $count } weaker matches
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
popup-key-paste = paste
popup-key-quick = quick paste
popup-key-all = all shortcuts
quick-paste-keys = Ctrl+1 to 9
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
top-add = Add files
top-add-files-tip = Add files or a folder to search (you can also drop them on the window)
top-add-files = Add files…
top-add-folder = Add a folder…
top-details-pane = Details pane
search-hint-images = Search text and images by meaning…
search-hint-meaning = Search by words or meaning…
search-hint = Search your clipboard…
search-hint-short = Search…
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
update-failed = Update failed: { $error }
update-failed-title = Update failed
update-progress = { $done } / { $total } MB
download-retrying =
    { $seconds ->
        [one] Connection lost. Trying again in 1 second…
       *[other] Connection lost. Trying again in { $seconds } seconds…
    }
update-skip = Skip this version
update-download = Download
update-restart = Restart to update
update-try-again = Try again
update-whats-new = What's new
update-close = Close
common-cancel = Cancel
status-model-loading = Loading search model…
status-model-off = Words only · set up meaning search
status-model-unavailable = Search model unavailable · fix in Settings
status-model-failing = Search server isn't working · Try again
status-model-failing-tip = Searching by meaning fails now. The app tries again by itself; click to try now.
status-model-words-only = Words only
status-model-tip = Search model: click to change
status-model-on = Search by meaning is on
status-model-none = Words only · add a search model
status-model-tip-named = Search model: { $model } on { $server }. Click to change.
status-indexing = Indexing · { $done } of { $total }
status-adding-files = Adding files · { $count } added
status-pause-recording = Pause recording
status-resume-recording = Resume recording
status-recording-stopped = Recording stopped · Start again
status-recording-stopped-tip = Recording stopped after repeated errors. Click to start it again.
status-shortcut-taken = Shortcut { $keys } is used by another app
status-open-shortcut-settings = Open the shortcut settings
status-dismiss-error = Click to dismiss
status-key-paste = paste
status-key-copy = copy
status-key-all = all shortcuts
list-drop-files = Drop to add to your searchable library
item-files-count =
    { $count ->
        [one] 1 file
       *[other] { $count } files
    }
item-empty = (empty)
item-color = Color · { $value }
item-delete = Delete
item-pin = Pin
item-unpin = Unpin
item-copy = Copy
item-open-details = Open details
item-paste-into = Paste into { $app }
item-paste = Paste
item-paste-plain = Paste as plain text
item-copy-only = Copy only
item-copy-plain = Copy as plain text
item-copy-image-text = Copy the text in the image
item-ignore-app = Never record from { $app }…
item-menu = Actions
item-paste-group =
    { $count ->
        [one] Paste 1 item
       *[other] Paste { $count } items together
    }
item-copy-group =
    { $count ->
        [one] Copy 1 item
       *[other] Copy { $count } items together
    }
item-pin-group =
    { $count ->
        [one] Pin 1 item
       *[other] Pin { $count } items
    }
item-unpin-group =
    { $count ->
        [one] Unpin 1 item
       *[other] Unpin { $count } items
    }
item-tag-group =
    { $count ->
        [one] Add a tag to 1 item
       *[other] Add a tag to { $count } items
    }
item-delete-group =
    { $count ->
        [one] Delete 1 item
       *[other] Delete { $count } items
    }
group-selected =
    { $count ->
        [one] 1 selected
       *[other] { $count } selected
    }
group-pin = Pin
group-unpin = Unpin
group-tag = Add a tag
group-copy = Copy together
group-delete = Delete
group-clear = Clear the selection
keys-sheet-title = Keys
keys-sheet-close = Close
checklist-title = Get started
checklist-copy = Copy something in any program
checklist-paste = Press { $keys } in another program, then paste an item
checklist-paste-no-key = Open this window over another program, then paste an item
checklist-pin = Pin an item, so it stays at the top
checklist-meaning = Turn on search by meaning (optional)
checklist-set-up = Set up
checklist-meaning-found = Turn on search by meaning with { $server } on this PC
checklist-turn-on = Turn on
checklist-close = Close the checklist
whats-new-title = What's new in { $version }
whats-new-close = Close
hint-with-keys = { $action } ({ $keys })
details-only-this-tag = Show only items with this tag
details-remove-tag = Remove tag
details-tag-hint = tag, another…
details-add-tag = Add a tag
details-tag = Tag
details-add-tag-tip = Add a tag
details-close = Close details (Esc)
details-pinned = Pinned
details-from-files = from your files
details-formatted = with formatting
details-copied-from = from { $app }
details-delete = Delete
details-unpin = Unpin
details-pin-keep = Pin: keep forever
details-save-image = Save image as…
details-show-in-folder = Show in folder
details-open-in-browser = Open in browser
details-more-ways = More ways to paste or copy
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
details-read-failed = couldn't read the text
details-read-again = Try again
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
details-key-quick = paste one of the first nine
details-key-menu = menu
details-key-all = all shortcuts
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
toast-show = Show
toast-skip-next = Next copy won't be saved
toast-history-almost-full = Your history is 90% full: { $used } of { $limit }. When it's full, the oldest go first.
toast-history-change-limit = Change the limit
toast-undo = Undo
toast-images-refused = The search server doesn't take images, so they are found by their text.
toast-images-off = Turn off Send images
toast-images-turned-off = Send images is off for this model.

## Settings

settings-page-general = General
settings-page-search = Search & AI
settings-page-servers = Servers
settings-page-privacy = Privacy
settings-page-about = About
settings-page-clipboard = Clipboard
settings-page-shortcuts = Shortcuts and pasting
settings-page-appearance = Appearance
app-title = Clipboard Manager
settings-back = Back (Esc)
settings-title = Settings
settings-saved-automatically = Changes are saved automatically
settings-back-to-default = Back to default
settings-browse = Browse…
settings-filter-matching = Matching files
settings-filter-all = All files
settings-language-and-time = Language and time
settings-start-and-tray = Start and tray
settings-language = Language
settings-language-hint = Menus, buttons and messages
settings-language-help = Arabic also lays the window out right to left.
settings-time-format = Time format
settings-time-format-hint = Times in the list and details, such as 14:30 or 2:30 PM
settings-time-format-help = They're stored in UTC and shown in your time zone.
settings-time-24h = 24-hour
settings-time-12h = 12-hour
settings-start-at-sign-in = Start when you sign in
settings-start-at-sign-in-hint = Opens the app each time you sign in to Windows
settings-start-at-sign-in-help = It starts hidden: in the system tray when that's on, otherwise minimised.
settings-tray = Keep running in the system tray
settings-tray-hint = Shows an icon in the system tray, near the clock
settings-tray-help = Closing or minimising the window hides it there and history keeps recording. Click the icon to open; right-click it to quit.
settings-startup-note = With "Start when you sign in" and "Keep running in the system tray" both on, the app starts quietly in the tray.
settings-updates = Updates
settings-update-check = Check for updates automatically
settings-update-check-hint = Looks for a new version when the app starts and once a day
settings-update-check-help = Only the version file on GitHub is fetched; nothing about you or your clipboard is sent.
settings-update-download = Download updates automatically
settings-update-download-hint = New versions download in the background and install when you quit
settings-update-download-help = Off: you're asked before anything is downloaded.
settings-version = Version
settings-version-hint = Every download is checked against this app's signing key before it's installed
settings-check-now = Check now
settings-update-checking = Checking…
settings-update-current = You're up to date
update-ready = Version { $version } is ready
update-ready-detail = Restart to update, or it installs when you quit.
settings-updates-off-dev = Updates are off in development builds
update-development-build = { $version } (development build)
settings-look = Look
settings-theme = Theme
settings-theme-hint = System follows your Windows light/dark setting
settings-theme-system = System
settings-theme-light = Light
settings-theme-dark = Dark
settings-dark-panels = Dark panels
settings-dark-panels-hint = In the dark theme, the details pane, cards and menus: darker than the window, or lighter
settings-dark-panels-darker = Darker
settings-dark-panels-lighter = Lighter
settings-accent = Accent colour
settings-accent-hint = Used for selection, buttons and highlights
accent-indigo = Indigo
accent-sky = Sky
accent-emerald = Emerald
accent-amber = Amber
accent-rose = Rose
settings-list = List
settings-layout = Layout
settings-layout-hint = Grid shows cards, List shows rows; both open details on demand
settings-layout-help = Detail keeps the selected item beside the list. Next to the search box, the Grid and List switch and the details pane button change this too.
settings-density = Density
settings-density-hint = Compact fits more items on screen by hiding the second line
settings-density-comfortable = Comfortable
settings-density-compact = Compact
settings-card-height = Card height
settings-card-height-hint = How tall the cards of the grid are
settings-card-height-same = All the same
settings-card-height-fit = Fit the content
settings-pinned-on-top = Pinned on top
settings-pinned-on-top-hint = Pinned items come first while you browse, whatever the order
settings-pinned-on-top-help = Search results are ordered by your sort choice only.
settings-group-by-day = Group by day
settings-group-by-day-hint = Show Pinned, Today, Yesterday… headers while browsing (not while searching)
settings-recording = Recording
settings-how-long = How long items stay
settings-record = Record clipboard history
settings-record-hint = Save everything you copy
settings-record-help = Turn off to pause; the pause button in the main window does the same.
settings-skip-secrets = Skip passwords and API keys
settings-skip-secrets-hint = Don't save copies that look like a password, API key, token or private key
settings-skip-secrets-help = You still can paste them normally.
note-title = Clipboard Manager note
settings-screen-notes = Notes on the screen
settings-screen-notes-hint = A note at the corner of the screen when a copy isn't saved, or the next one won't be
settings-screen-notes-help = It never takes the keyboard, and stays quiet while a full-screen program or a presentation runs. Click it to open the window.
settings-never-kept = Never kept
settings-skip-next = Don't record the next copy
settings-skip-next-hint = Press this key before you copy something you don't want kept, such as a password
settings-change-key = Change
settings-data-goes = Where your data goes
settings-data-history = Your history
settings-data-history-encrypted = Encrypted on this PC
settings-data-history-local = On this PC
settings-data-search = Search by meaning
settings-data-reader = Text in images
settings-data-updates = Update check
settings-data-off = Off
settings-data-on-pc = Stays on this PC
settings-data-sent = Sent to { $host }
settings-data-windows = Read by Windows on this PC
settings-data-updates-on = Downloads only the version file, from GitHub
settings-pasting = Pasting
settings-reset-section = Reset
settings-merge-duplicates = Merge duplicate copies
settings-merge-duplicates-hint = Copying something you already have moves it to the top instead of adding it again
settings-reader-kind = Model type
settings-reader-kind-hint = Sets the prompt and the settings that suit the model
settings-reader-kind-help = The server runs the model; the README tells how to start it.
reader-kind-other = Another model
settings-reader-prompt = Prompt
settings-reader-prompt-hint = What the model is asked with each image
settings-reader-prompt-help = The model type fills it in. Another model may want its own words, such as "OCR:" for PaddleOCR-VL.
settings-reader-send-question = Send the images you copy to { $host } to be read? This server is not on this PC.
settings-hide-window = Hide the window
settings-hide-window-hint = Hides the window after you copy an item, so you can paste it
settings-hide-window-help = Pasting always hides the window.
settings-keep-up-to = Keep up to
settings-keep-up-to-hint = When there are more, the oldest are deleted
settings-keep-up-to-help = Pinned and tagged items, and files you added, are never deleted.
settings-history-items = { $size } items
settings-delete-older = Delete items older than
settings-delete-older-hint = Deletes items older than the time you choose
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
settings-rule-programs = Programs in your history
settings-rule-add-other = A text pattern, or another program
settings-rule-needs = Give a program, a pattern, or both. Until then this rule does nothing.
settings-rule-bad-pattern = This regular expression can't be used, so the rule does nothing: { $error }
settings-storage = Storage
settings-data-folder = Data folder
settings-data-folder-hint = History, images, models and settings are stored here, on this PC only
settings-open = Open
settings-log-file = Log file
settings-log-file-hint = A record of what the app did in this session
settings-log-file-help = Attach it when reporting a problem.
settings-bug-report = Bug report
settings-bug-report-hint = Copies the version, Windows' version, your settings without addresses or keys, and the log's last lines
settings-bug-report-copy = Copy details
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
settings-clear-history = Clear clipboard history
settings-clear-history-hint = Deletes copied items
settings-clear-history-help = Pinned and tagged items, and files you added, stay.
settings-delete-items =
    { $count ->
        [one] Delete 1 item
       *[other] Delete { $count } items
    }
settings-clear = Clear…
settings-reset-all = Reset all settings
settings-reset-all-hint = Puts every setting back to its default. Your saved servers and rules stay
settings-reset-everything = Reset everything
settings-reset = Reset…
settings-strictness-any = Anything related
settings-strictness-close = Close
settings-strictness-very-close = Very close
settings-strictness-custom = Custom ({ $value })
settings-search = Search
settings-search-mode = How search works
settings-search-mode-meaning-off = Search by meaning is off, so search finds exact words only
settings-search-mode-help =
    Smart: finds items that contain your words and items that mean the same thing, merged into one list.
    Meaning: only by meaning (needs a search model).
    Exact words: only items containing what you type.
settings-strictness = How close the meaning must be
settings-strictness-hint = Stricter choices hide weaker matches; exact words always show
settings-meaning-section = Search by meaning
settings-meaning-switch = Use search by meaning
settings-meaning-ready = Ready: finds items by what they mean
settings-meaning-off = Off: search finds words only
settings-meaning-none = No search model yet
settings-meaning-unavailable = The search model can't be used: check its server
settings-read-section = Read text in images
settings-servers-section = Saved servers
settings-read-engine = Read by
settings-search-engine-help = Off: search by words only, plus the text found in images. On: a model on one of your servers, on this PC or another one, also finds items by meaning.
settings-read-engine-hint = Reads the words in screenshots and photos, so search finds them
settings-read-engine-help = Windows reads the languages installed in it, Arabic included when it is there. Your own server: a vision model on one of your servers.
settings-engine-off = Off
settings-engine-windows = Windows
settings-engine-server = Your own server
settings-model = Model
settings-search-model-hint = Applies at once; each model keeps its own search data, so going back is instant
settings-search-model-help = Another one applies at once; each model keeps its own index, so going back to one needs no new indexing.
settings-read-model-hint = The vision model that reads the text in images
settings-read-model-help = It reads the images copied from now on; images already read keep their text.
settings-model-choose = Choose a model
settings-model-on-server = { $model } · { $server }
settings-model-on-server-only = on the server
settings-found-title = Found on this PC
settings-found-running = { $server } is running on this PC
settings-found-search = Search
settings-found-use = Use
settings-found-add = Add
settings-found-needs-key = needs its API key
settings-model-add-server = Add a server…
settings-model-none-search = No search model yet. Add a server, then a search model on it.
settings-model-none-read = No reading model yet. Add a server, then a reading model on it.
settings-model-off-note = Searching by words only. Turn on a server's model to also find items by meaning, e.g. “trip” finds “flight to Jeddah”.
settings-read-off-note = Text in images is not read, so images are found only by meaning, when the search model takes images.
settings-read-windows-note = Uses the languages installed in Windows.
settings-servers-none = No servers yet. Add the server your models run on: llama.cpp, Ollama, LM Studio, vLLM, Unsloth, OpenRouter or another one.
settings-server-add = Add server
settings-server-edit = Edit
settings-server-not-saved = Not saved
settings-server-discard = Discard
settings-server-models-count =
    { $count ->
        [one] 1 model
       *[other] { $count } models
    }
settings-server-back = Back to Search & AI
settings-server-add-title = Add server
settings-server-edit-title = Edit server
settings-server-kind = Type
server-kind-other = Other
settings-server-name = Name
settings-server-name-hint = What the menus and the status bar call this server
settings-server-test-connection = Test connection
settings-server-found =
    { $count ->
        [one] Connected · 1 model found
       *[other] Connected · { $count } models found
    }
settings-server-needed = needed
settings-server-models-section = Models
settings-server-no-models = No models yet. Add the ones you use on this server.
settings-model-add-search = Add a search model
settings-model-add-read = Add a reading model
settings-model-job-search = Search
settings-model-job-read = Read text
settings-model-open = Open the model's settings
settings-model-close = Close the model's settings
settings-model-remove = Remove this model
settings-model-test = Test
settings-model-test-help = Sends the model a short test text, and a test image if it reads text or Send images is on, and shows what comes back. Nothing from your history is sent.
settings-model-test-search = Works · { $size } numbers per vector · { $seconds } s
settings-model-test-multi = Works · { $vectors } vectors of { $size } numbers · { $seconds } s
settings-model-test-read = Read: “{ $text }” · { $seconds } s
settings-model-test-images = Images work
settings-model-test-no-images = Doesn't take images: “{ $said }”. Turn off Send images.
settings-model-test-images-failed = The test image failed: { $error }
settings-model-on-the-server = Model on the server
settings-model-on-the-server-hint = The model's name on the server
settings-model-on-the-server-help = Pick one it lists, or type one it doesn't list. Leave it empty for the server's default model.
settings-model-name = Name in menus
settings-model-name-hint = A short name for the menus and the status bar
settings-model-name-help = Leave it empty to show the model's name.
settings-model-fill = Fill in details
settings-model-fill-help = Looks the model up on models.dev, a public list of AI models, and fills in its name. The prompts and settings that well-known models need are filled in too. Nothing from your history is sent.
settings-model-fill-looking = Looking it up on models.dev…
settings-model-fill-found = models.dev: { $name }, takes up to { $tokens } tokens.
settings-model-fill-found-no-limit = models.dev: { $name }.
settings-model-fill-not-found = models.dev doesn't list this model.
settings-model-fill-prompts = Filled in the prompts this model is trained with.
settings-model-fill-kind = Filled in the settings for { $model }.
settings-model-fill-images = It takes images too: turn on “Send images” if your server takes them.
settings-model-fill-no-images = It doesn't take images, so it can't read the text in them.
settings-model-fill-needs-name = Type the model's name first.
settings-model-prompts-tip = This model searches better with its own prompts
settings-server-in-use-removed = A model in use is gone from this server: saving turns its job off.
settings-server-send-and-save = Send and save
settings-server-save = Save
settings-server-delete = Delete server
settings-server-delete-question = Delete { $name } and its models?
settings-server-delete-confirm = Delete
settings-server-needs-address = Type the server's address to save it.
settings-vector-all = Full
settings-server-address = Server address
settings-server-address-hint = The server's address, usually ending in /v1
settings-server-address-help = Change the port if your server uses another one.
settings-server-remote = What you copy is sent to { $host } to be indexed, because this server is not on this PC.
settings-server-remote-http = What you copy is sent to { $host } to be indexed, because this server is not on this PC. The address starts with http, so it travels without encryption.
settings-server-send-question = Send your whole history to { $host } to be indexed? This server is not on this PC.
settings-server-send = Send
settings-server-send-cancel = Cancel
settings-server-key = API key
settings-server-key-hint = Only if your server asks for one
settings-server-key-help = Sent as a Bearer token and stored only on this PC.
settings-server-hide-key = Hide key
settings-server-show-key = Show key
settings-server-optional = optional
settings-server-models = Models the server lists
settings-server-default = Server default
settings-server-default-hint = default: { $model }
settings-server-checking = Checking the server…
settings-server-launch-note = The app starts llama-server itself when a model of this server is in use.
settings-server-use-them = Use them
settings-server-query-prompt = Query prompt
settings-server-query-prompt-hint = Text put before what you search for
settings-server-query-prompt-help = Some models need it (EmbeddingGemma: “task: search result | query: ”).
settings-server-none = none
settings-server-item-prompt = Item prompt
settings-server-item-prompt-hint = Text put before each item when it's indexed (EmbeddingGemma: “title: none | text: ”)
settings-server-vector-size = Vector size
settings-server-vector-size-hint = How many numbers of each vector to keep
settings-server-vector-size-help = Fewer use less memory and search faster, with slightly lower quality; only Matryoshka models such as EmbeddingGemma, nomic-embed and OpenAI's text-embedding-3 stay good when shortened. Test shows how many the model gives.
settings-server-images = Send images
settings-server-images-hint = Send copied images too, for models that understand images
settings-server-images-help = For example CLIP, SigLIP, BiUltraVBERT, or EmbeddingGemma 2 with its mmproj file, on a server that takes images. Otherwise images are found by the text in them.
settings-server-launch = Start llama-server for me
settings-server-launch-hint = The app starts llama-server with your model, from a file or by its Hugging Face name, in the background and stops it when you quit
settings-server-hf = Or a Hugging Face name
settings-server-hf-hint = Such as ggml-org/GLM-OCR-GGUF: llama-server downloads it the first time, and it is used instead of the file
settings-server-one-model = One llama-server runs one model. To start a search model and a reading model, save them as two llama.cpp servers, each with its own port.
settings-server-program = llama-server program
settings-server-program-hint = llama-server.exe from a llama.cpp release
settings-server-model-file = Model file (.gguf)
settings-server-projector = Image projector (.gguf)
settings-server-projector-hint = Optional mmproj file, for models that embed images
settings-server-port = Port
settings-server-extra-args = Extra arguments
settings-server-extra-args-hint = Passed to llama-server as-is
settings-server-extra-args-help = A search model usually needs --pooling mean. A reading model needs room for the picture and its text, such as -c 8192, and no --pooling.
settings-shortcuts-global = Anywhere in Windows
settings-shortcut-popup = Open the quick popup
settings-shortcut-popup-hint = A small window at the mouse pointer: search, then paste into the program you are in
settings-shortcut-popup-help = Press again to close it.
settings-shortcut-window = Open the full window
settings-shortcut-window-hint = The window with everything: filters, details and settings
settings-shortcut-window-help = Press again to hide it.
settings-shortcut-ignore = Don't record the next copy
settings-shortcut-ignore-hint = Press before copying something private: the next copy is skipped
settings-shortcut-not-usable = Not usable
settings-shortcut-taken = Used by another app
settings-shortcuts-local = Inside the app
settings-shortcut-paste = Paste the selected item into the program you were using
settings-shortcut-paste-hint = This works when the window was opened with the shortcut
settings-shortcut-paste-help = Otherwise, and for programs that run as administrator, the item is only copied.
settings-shortcut-paste-plain = Paste the selected item as plain text, without formatting
settings-shortcut-copy-only = Copy the selected item without pasting it
settings-shortcut-pin = Pin or unpin
settings-shortcut-delete = Delete
settings-shortcut-tag = Add a tag
settings-shortcut-view = Switch between Grid, List and Detail
settings-shortcut-details = Open details
settings-shortcut-used-twice = Used twice
settings-shortcut-move = Move through the list
settings-shortcut-move-grid = Move between cards in the grid
settings-shortcut-quick-paste = Paste one of the first nine items
settings-shortcut-menu = Open the menu of the selected item
settings-shortcut-keys = Show every key
settings-shortcut-escape = Close details, clear the search, or hide
settings-shortcut-built-in = Built in; can't be changed
settings-shortcuts-note = Click a shortcut, then press the new keys. Backspace turns it off, Esc cancels.

## Errors, statuses, messages and the tray icon

error-saving-item = Could not save the item: { $error }
error-saving-image = Could not save the image: { $error }
error-saving-settings = Could not save the settings: { $error }
error-deleting = Could not delete: { $error }
error-copying = Could not copy: { $error }
error-opening-folder = Could not open the folder: { $error }
error-background-crash = A background task stopped because of an error. The log file has the details.
error-tray-unavailable = The system tray icon is not available: { $error }
error-start-at-sign-in = Could not change “Start when you sign in”: { $error }
error-hotkeys-unavailable = Shortcuts that work anywhere in Windows are not available: { $error }
error-hotkey-taken = { $keys } is already used by another app or by Windows. Choose other keys in Settings, Shortcuts and pasting.
error-hotkey-invalid = Cannot use the shortcut { $keys }: { $error }
error-indexing-skipped = Indexing failed on an item; it was skipped.
error-indexing = Could not index items for search by meaning: { $error }
error-query-embedding = Could not search by meaning: { $error }
error-model = Could not load the search model: { $error }
error-model-test = The test failed: { $error }
error-models-dev = Could not look the model up on models.dev: { $error }
error-ocr-unavailable = Cannot read text in images: { $error }
error-reader-server-stopped = The llama-server for reading text stopped while starting ({ $status }). Check its model file or Hugging Face name, and its extra arguments.
error-server-unreachable = Can't reach the server at { $host }. Check that it is running and that its address is right.
error-server-needs-key = The server needs an API key, or the key is wrong. Check the key in the server's settings.
error-server-not-found = The server doesn't know this address or model: “{ $said }”
error-server-refused = The server refused the request: “{ $said }”
error-server-failed = The server failed (error { $status }): “{ $said }”
error-server-no-images = The server doesn't take images: “{ $said }”
error-server-no-models = The server offers no models.
error-server-bad-answer = The server's answer is not what the app expected.
error-settings-unreadable = Could not read the settings ({ $error }). The app now uses the default settings. The old file is saved as { $copy }
error-settings-unreadable-no-copy = Could not read the settings ({ $error }). The app now uses the default settings. The old file could not be copied either: { $reason }
error-history-moved-aside = Your history couldn't be opened, so a new, empty one started. The old one was kept as { $file }
error-start-failed = Clipboard Manager could not start.
status-starting-llama = Starting llama-server…
status-reader-starting = Starting llama-server to read text in images…
status-reader-downloading = Starting llama-server to read text in images; it downloads the model the first time…
status-connecting = Connecting to { $host }…
toast-copied = Copied
toast-copied-admin = Copied. That program runs as administrator, so Windows doesn't let this app paste into it. Press Ctrl+V there.
toast-copied-text = Copied text
toast-saved = Saved
toast-deleted = Deleted
toast-updated = Updated to { $version }
toast-pinned-group =
    { $count ->
        [one] Pinned 1 item
       *[other] Pinned { $count } items
    }
toast-unpinned-group =
    { $count ->
        [one] Unpinned 1 item
       *[other] Unpinned { $count } items
    }
toast-deleted-group =
    { $count ->
        [one] Deleted 1 item
       *[other] Deleted { $count } items
    }
toast-copied-group =
    { $count ->
        [one] Copied 1 item
       *[other] Copied { $count } items together
    }
toast-copied-group-pictures =
    { $count ->
        [one] Copied 1 item
       *[other] Copied { $count } items together
    }, without { $pictures ->
        [one] 1 picture
       *[other] { $pictures } pictures
    }
toast-pictures-not-together = Pictures can't be copied together
toast-tag-added-group =
    { $count ->
        [one] Added #{ $tag } to 1 item
       *[other] Added #{ $tag } to { $count } items
    }
toast-deleted-key = Deleted. Next time: { $keys }
toast-copied-key = Copied. Next time: { $keys }
toast-pinned-key = Pinned. Next time: { $keys }
toast-unpinned-key = Unpinned. Next time: { $keys }
toast-imported =
    { $count ->
        [one] Imported 1 file
       *[other] Imported { $count } files
    }
toast-pruned-age =
    { $count ->
        [one] Deleted 1 item older than { $age }
       *[other] Deleted { $count } items older than { $age }
    }
toast-pruned-limit =
    { $count ->
        [one] Deleted 1 item over your history limit
       *[other] Deleted { $count } items over your history limit
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
toast-skipped-private = Not saved: marked private
toast-skipped-by = By { $program }
toast-skipped-from = From { $program }
toast-skipped-password = Not saved: a password
toast-skipped-api-key = Not saved: an API key
toast-skipped-sign-in = Not saved: a sign-in token
toast-skipped-private-key = Not saved: a private key
toast-skipped-link = Not saved: a link with a password
toast-skipped-key = Not saved: a key
toast-hint-characters =
    { $count ->
        [one] 1 character
       *[other] { $count } characters
    }
toast-skipped-rule = Not saved: one of your rules skips it
toast-never-record-question = Never record from { $app }? Copies from it won't be saved.
toast-never-record = Never record
toast-app-ignored = Copies from { $app } won't be recorded. You can remove the rule in Settings, Privacy.
tray-show = Show Clipboard Manager
tray-pause-recording = Pause recording
tray-resume-recording = Resume recording
tray-quit = Quit
tray-tooltip-paused = Clipboard Manager · Recording paused
