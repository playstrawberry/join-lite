//! A text the server owner writes, shown in pages.
//!
//! The owner's words are a file in the script's data folder, `<name>.txt`, with one optional file per language,
//! `<name>.fr.txt`, `<name>.es.txt`, `<name>.tr.txt` (and `<name>.en.txt`): a player reads the file of his
//! client's language when it exists and `<name>.txt` otherwise. A line is one chat line; `&` colour codes work.
//!
//! The files are read once at load and never again: a command reads memory only. Everything the script itself
//! says around the owner's text (the page line, an empty text, a wrong page number) comes from the script's
//! lang files under the keys in [`KEYS`].
#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
pub mod runtime;

use sp_lang::Language;
use std::{io::Read, path::Path};

/// The largest file read.
pub const MAX_BYTES: u64 = 32 * 1024;
/// The most lines one text holds.
pub const MAX_LINES: usize = 400;
/// The longest line, in bytes.
pub const MAX_LINE_BYTES: usize = 512;
/// Lines on one page.
pub const PER_PAGE: usize = 8;

/// The lang keys a script that shows pages must have in every language. `page` takes the page and the page
/// count, `next` the same two, then the command and the next page's number; `no_page` takes the page count.
pub const KEYS: [&str; 4] = ["empty", "next", "no_page", "page"];

/// One text, as lines.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    lines: Vec<String>,
}

impl Text {
    /// Reads the file's content. Blank lines at the end are dropped; everything else stays as written.
    /// `Err("data_error")` for a control character, a line over [`MAX_LINE_BYTES`] or more than [`MAX_LINES`].
    pub fn parse(text: &str) -> Result<Self, &'static str> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut lines = Vec::new();
        for line in text.lines() {
            if line.len() > MAX_LINE_BYTES || line.chars().any(char::is_control) {
                return Err("data_error");
            }
            lines.push(line.trim_end().to_owned());
        }
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        if lines.len() > MAX_LINES {
            return Err("data_error");
        }
        Ok(Self { lines })
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Every line, in the file's order.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// How many pages the text has; an empty text has none.
    pub fn pages(&self) -> usize {
        self.lines.len().div_ceil(PER_PAGE)
    }

    /// The lines of page `number`, counted from 1.
    pub fn page(&self, number: usize) -> Option<&[String]> {
        let start = number.checked_sub(1)?.checked_mul(PER_PAGE)?;
        let end = start.saturating_add(PER_PAGE).min(self.lines.len());
        self.lines.get(start..end).filter(|page| !page.is_empty())
    }
}

/// A text in the owner's languages.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Texts {
    default: Text,
    languages: Vec<(Language, Text)>,
}

fn read(path: &Path) -> Result<Option<String>, &'static str> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("data_error"),
    };
    let mut text = String::new();
    // One byte more than the limit is read, so a longer file is seen and refused instead of cut.
    let read = file
        .take(MAX_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|_| "data_error")?;
    if u64::try_from(read).unwrap_or(u64::MAX) > MAX_BYTES {
        return Err("data_error");
    }
    Ok(Some(text))
}

impl Texts {
    /// Reads `<dir>/<stem>.txt` and the language files next to it. A missing `<stem>.txt` is created empty,
    /// so the owner finds the file to write into. `stem` is the script's own constant, never a player's word.
    pub fn load(dir: &Path, stem: &str) -> Result<Self, &'static str> {
        Self::load_with(dir, stem, "")
    }

    /// As [`Texts::load`], with `first` written into a missing `<stem>.txt` (the script's own starting text).
    pub fn load_with(dir: &Path, stem: &str, first: &str) -> Result<Self, &'static str> {
        let main = dir.join(format!("{stem}.txt"));
        let default = match read(&main)? {
            Some(text) => Text::parse(&text)?,
            None => {
                std::fs::write(&main, first).map_err(|_| "save_error")?;
                Text::parse(first)?
            }
        };
        let mut languages = Vec::new();
        for language in Language::ALL {
            let path = dir.join(format!("{stem}.{}.txt", language.code()));
            if let Some(text) = read(&path)? {
                languages.push((language, Text::parse(&text)?));
            }
        }
        Ok(Self { default, languages })
    }

    /// The main file's text, whatever the reader's language.
    pub fn main(&self) -> &Text {
        &self.default
    }

    /// The text a client of this locale reads: its language's file when the owner wrote one, else the main file.
    pub fn of(&self, locale: &str) -> &Text {
        let language = Language::of_locale(locale);
        self.languages
            .iter()
            .find(|(l, text)| *l == language && !text.is_empty())
            .map_or(&self.default, |(_, text)| text)
    }
}

/// What a command with this page argument shows.
#[derive(Debug, PartialEq, Eq)]
pub enum Shown<'a> {
    /// The owner wrote nothing yet.
    Empty,
    /// There is no such page; the text has this many.
    NoPage(usize),
    /// These lines; this page of that many.
    Page(&'a [String], usize, usize),
}

/// Picks the page for a command. No number means the first page.
pub fn show(text: &Text, page: Option<usize>) -> Shown<'_> {
    if text.is_empty() {
        return Shown::Empty;
    }
    let number = page.unwrap_or(1);
    match text.page(number) {
        Some(lines) => Shown::Page(lines, number, text.pages()),
        None => Shown::NoPage(text.pages()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbered(count: usize) -> String {
        (1..=count).map(|n| format!("line {n}\n")).collect()
    }

    #[test]
    fn a_text_is_cut_into_pages_and_a_wrong_page_is_told_the_count() {
        let text = Text::parse(&numbered(17)).unwrap();
        assert_eq!(text.pages(), 3);
        let Shown::Page(lines, 1, 3) = show(&text, None) else {
            panic!("first page");
        };
        assert_eq!(lines.len(), PER_PAGE);
        assert_eq!(lines[0], "line 1");
        let Shown::Page(last, 3, 3) = show(&text, Some(3)) else {
            panic!("last page");
        };
        assert_eq!(last, ["line 17".to_owned()]);
        assert_eq!(show(&text, Some(4)), Shown::NoPage(3));
        assert_eq!(show(&text, Some(0)), Shown::NoPage(3));
        assert_eq!(show(&text, Some(usize::MAX)), Shown::NoPage(3));
        assert_eq!(show(&Text::default(), Some(1)), Shown::Empty);
    }

    #[test]
    fn the_owners_lines_stay_as_written_and_end_blanks_are_dropped() {
        let text = Text::parse("\u{feff}&6Rules\r\n\r\n  1. Be kind  \n\n\n").unwrap();
        assert_eq!(text.lines, ["&6Rules", "", "  1. Be kind"]);
        assert!(Text::parse("").unwrap().is_empty());
        assert!(Text::parse("\n \n").unwrap().is_empty());
    }

    #[test]
    fn a_file_that_is_too_long_or_holds_control_characters_is_refused() {
        assert!(Text::parse(&numbered(MAX_LINES)).is_ok());
        assert_eq!(Text::parse(&numbered(MAX_LINES + 1)), Err("data_error"));
        assert!(Text::parse(&"x".repeat(MAX_LINE_BYTES)).is_ok());
        assert_eq!(
            Text::parse(&"x".repeat(MAX_LINE_BYTES + 1)),
            Err("data_error")
        );
        for bad in ["a\u{1b}[2Jb", "a\tb", "a\u{7}", "a\u{0}b"] {
            assert_eq!(Text::parse(bad), Err("data_error"), "{bad:?}");
        }
    }

    #[test]
    fn a_player_reads_his_languages_file_and_the_main_file_otherwise() {
        let dir = std::env::temp_dir().join(format!("sp-pages-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let texts = Texts::load(&dir, "rules").unwrap();
        assert!(texts.of("en_us").is_empty());
        assert_eq!(std::fs::read_to_string(dir.join("rules.txt")).unwrap(), "");
        std::fs::write(dir.join("rules.txt"), "main\n").unwrap();
        std::fs::write(dir.join("rules.tr.txt"), "kurallar\n").unwrap();
        std::fs::write(dir.join("rules.fr.txt"), "\n").unwrap();
        let texts = Texts::load(&dir, "rules").unwrap();
        assert_eq!(texts.of("tr_tr").lines, ["kurallar"]);
        assert_eq!(
            texts.of("fr_fr").lines,
            ["main"],
            "an empty language file falls back"
        );
        assert_eq!(texts.of("en_us").lines, ["main"]);
        assert_eq!(texts.of("de_de").lines, ["main"]);
        let first = Texts::load_with(&dir, "join", "&a[+] {name}\n").unwrap();
        assert_eq!(first.main().lines(), ["&a[+] {name}"]);
        std::fs::write(dir.join("join.txt"), "").unwrap();
        let emptied = Texts::load_with(&dir, "join", "&a[+] {name}\n").unwrap();
        assert!(emptied.main().is_empty(), "an emptied file stays empty");
        std::fs::write(dir.join("rules.es.txt"), "x".repeat(MAX_BYTES as usize + 1)).unwrap();
        assert_eq!(Texts::load(&dir, "rules"), Err("data_error"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
