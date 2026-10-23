# Translating Clipboard Manager

**TL;DR**

- Every text of the window lives in `locales/<language>/main.ftl`, written in
  [Project Fluent](https://projectfluent.org/).
- The code asks for a text by its key: `t!("toast-copied")`.
- To change a text, edit the `.ftl` files. To add a text, add its key to every language, then use the key in the code.
- `cargo test` checks that every language has every key with the same values, and that the code and the files match.
- A new language needs its own `.ftl` file and a few small code changes, listed at the end.

## The text files

`locales/en/main.ftl` holds the English texts, and `locales/ar/main.ftl` the Arabic texts. The build puts both files
into the app, so the app needs no extra files when it runs.

English is also the fallback. When a language has no text for a key, the app shows the English text. When no language
has the key, the app shows the key itself, so the mistake is easy to see.

The files are split into sections, one for each part of the window. A section starts with a line that begins with
`##`. A line that begins with `#` is a note for translators.

## What a text looks like

A text is a key, an equals sign, and the text:

```fluent
toast-copied = Copied
```

A value from the app goes in braces, with a dollar sign before its name. Keep each name exactly as it is in English.
Only move it to the place where it fits in your sentence:

```fluent
list-no-matches = No matches for "{ $query }"
```

A longer text can go on several lines. Start the text on a new line and indent each line by four spaces:

```fluent
list-empty-help =
    Copy anything, or drop files here.
    Press { $key } to open this window from anywhere.
```

### Numbers and plural forms

A text with a number chooses a form for that number. Each language has its own forms, so write the forms that your
language needs:

```fluent
toast-cleared =
    { $count ->
        [one] Cleared 1 item
       *[other] Cleared { $count } items
    }
```

- `[zero]`, `[one]`, `[two]`, `[few]`, `[many]` and `[other]` are the plural categories of the Unicode CLDR rules.
  The [CLDR table](https://www.unicode.org/cldr/charts/latest/supplemental/language_plural_rules.html) shows which
  numbers belong to which category in each language.
- English uses `one` and `other`. Arabic uses all six; `locales/ar/main.ftl` has real examples.
- The star marks the default form (`*[other]`). Every choice needs one. The app uses it for each category that has no
  form of its own.

### Right-to-left languages

The window takes the direction of each line from its first letter. So in a right-to-left language such as Arabic,
start each text with a word of your language. A line that starts with a Latin name or a value, such as `{ $keys }`
or a model name, is laid out left to right, and the words come out in the wrong order. Numbers do not count: a line
may start with `{ $count }`.

## Rules for keys

- A key is lower-case words joined by dashes, such as `details-save-image`.
- The first word says where the text is: `top-` (the top bar), `list-`, `details-`, `status-` (the status bar),
  `settings-`, `toast-` (a short message at the bottom of the window), `error-`, `tray-` (the system tray icon),
  `time-`, `date-`, and so on.
- Each place gets its own key, even when the English text is the same as in another place. Another language may need
  different words in each place.
- A key holds a whole sentence. Never build a sentence from pieces in the code, because the word order is different in
  each language. Put the changing parts in values instead.

## Adding or changing a text (for developers)

1. Add the key and the English text to `locales/en/main.ftl`, in the right section.
2. Add the same key to every other language file. If you cannot translate the text, ask for help in the pull request.
   Do not copy the English text into the other file.
3. Use the key in the code:
   - `t!("key")` for a plain text.
   - `t!("key", count = n, name = value)` for a text with values. The names must match the `$` names in the files.
   - `log_error!("error-key", error = e.to_string())` for an error message. It returns the text in the interface
     language, to show on the screen. It also writes the English text to the log, so the log reads the same for
     everyone.
4. Run `cargo test`. The tests in `src/i18n.rs` check that:
   - every language has every key, and no key that English does not have;
   - each key has the same `$` values in every language;
   - every key in the code is in the files, and every key in the files is used in the code;
   - plural forms work for a few numbers in each language.

Some texts stay in English in the code, because people do not see them in the window: log lines, the details inside an
error message, and names such as product names and key names (Ctrl, Alt).

## Adding a language

This example adds German, with the language code `de`.

1. Copy `locales/en/main.ftl` to `locales/de/main.ftl`. Translate the text after each `=`. Do not change the keys or
   the `$` names. Write the plural forms that German needs.
2. In `src/config.rs`, add `German` to the `Language` list.
3. In `src/i18n.rs`:
   - Add `const GERMAN_TEXTS: &str = include_str!("../locales/de/main.ftl");`.
   - In `bundle()`, make the list of bundles one longer, load the new file with `load("de", GERMAN_TEXTS)`, and
     return it for `Language::German`.
   - Today `set()` keeps only one yes-or-no value: is the language Arabic? Change it to keep the chosen `Language`, for
     example as a number in an `AtomicU8`. Then make `text()` use that language, and make `rtl()` return true only for
     a language that is written right to left.
   - In the tests, compare the new file with English in `every_language_has_every_text_with_the_same_values`, and add a
     few numbers for German to `plurals_follow_each_language`.
4. In `src/app/settings/general.rs`, add the language to the language choice. Write its name in its own language:
   "Deutsch".
5. Fonts: the app brings its own Arabic font (`add_fonts` in `src/app/view/theme.rs`). A language with another
   script, such as Hebrew, Hindi or Chinese, needs a font there too.
6. Run `cargo test`. Then start a dev build, choose the new language in Settings, General, and look at every page.
   Long words can make buttons and labels wider than in English.
