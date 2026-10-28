# Changelog

What changed in each version, for the people who use the app. Each release
starts its notes with the section of the version it was built from, so add
the notes here before a push that makes a release.

## 0.4.0

- Fix: a global shortcut changed in Settings worked only after a restart.
- An installer: clipboard-manager-windows-x64-setup.exe installs the app
  for your Windows account, without administrator rights, with Start menu
  and desktop shortcuts and an entry in Installed apps. Uninstalling asks
  whether to delete your history and settings too. The exe that runs
  without installing stays, and both update themselves.
- Choose what reads the text in images, in Settings, Clipboard: Windows'
  own reader, the built-in one, or a vision model on a server (such as
  DeepSeek-OCR or PaddleOCR-VL) with your own prompt. Windows' reader also
  reads Arabic when it is installed in Windows. New installs use it;
  existing ones keep the built-in reader until you change it. A server
  outside this PC gets a warning, and the app asks before it sends images
  there.
- Quick popup: Alt+V now opens a small window at the mouse pointer with
  the search field and the list. Pick an item and press Enter to paste it
  where you were. The popup closes when you paste, press Esc or click
  elsewhere. Alt+Shift+V, or the button in the popup, opens the full
  window. If you had changed Alt+V, your shortcut now opens the popup.
- Paste into the program you were using: open the window with the shortcut,
  pick an item and press Enter or double-click it. The window hides and the
  item is pasted where you were. Before, Enter and double-click only copied.
  Shift+Enter pastes text without its formatting, and Ctrl+Enter only
  copies. Programs that run as administrator get only the copy, because
  Windows doesn't let the app press keys there. The keys can be changed in
  Settings, Shortcuts.
- Formatting is kept: text copied with formatting (HTML or RTF, from a
  browser or Word, say) pastes back with it. The formatting is stored
  encrypted, apart from the list, and kept up to 2 MB per copy.
- Rules per app: Settings, Clipboard can skip every copy from a program, or
  text that matches a pattern, from one program or any. A pattern is simple
  (with * and ?) or a regular expression. A skipped copy can still be kept
  with "Save anyway". The details show which program a copy came from, with
  a link to stop recording that program.
- Delete by age: Settings, Clipboard can delete items older than a day, a
  week, a month, 3 months or a year. It is off unless you choose a time,
  and it never deletes pinned or tagged items, or files you added.

## 0.3.0

**Your history is now encrypted.** On Windows, the text and images you copy
are encrypted on disk with a key that only your Windows account on this PC
can open. The first start of this version encrypts your existing history
once. Back up `%APPDATA%\clipboard-manager\data` before you update. A history
copied to another PC or Windows account can't be opened there: the app keeps
it aside and starts a new one.

- Updates are safer: everything the app reads about a new version is signed,
  so an older version can't be offered as a new one.
- Settings warns before your history is sent to a search server outside this
  PC, and asks once when you apply one.
- Arabic: texts read in the right order, and the tray menu, messages and
  errors are translated, with the right plural forms.
- Fixes: the tray menu stopped working after a language change, long errors
  covered the item count, Shift+Delete did nothing, and more.
- A portable copy keeps its window size and place in its own folder.
