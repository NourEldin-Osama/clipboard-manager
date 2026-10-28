# Changelog

What changed in each version, for the people who use the app. Each release
starts its notes with the section of the version it was built from, so add
the notes here before a push that makes a release.

## 0.4.0

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
