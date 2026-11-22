//! The owner's join and quit lines.
use sp_pages::Text;

/// What a missing `join.txt` starts with.
pub const FIRST_JOIN: &str = "&a[+] &f{name}\n";
/// What a missing `quit.txt` starts with.
pub const FIRST_QUIT: &str = "&c[-] &f{name}\n";

/// One line as the owner wrote it. `{name}` is the player's name; `{prefix}` is his rank's prefix as `[VIP] `
/// (nothing when he has none); `&` colour codes work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Format {
    line: Option<String>,
}

impl Format {
    /// The file's one line. An empty file means the line is not shown at all. A second line is refused
    /// (`Err("data_error")`): a join line is one chat line, and nothing the owner wrote is dropped silently.
    pub fn of(text: &Text) -> Result<Self, &'static str> {
        match text.lines() {
            [] => Ok(Self { line: None }),
            [line] => Ok(Self {
                line: Some(line.clone()),
            }),
            _ => Err("data_error"),
        }
    }

    /// Whether the line names the prefix, so the prefix is asked for only then.
    pub fn wants_prefix(&self) -> bool {
        self.line.as_deref().is_some_and(|l| l.contains("{prefix}"))
    }

    /// The line for this player, still with its `&` codes; `None` when the owner emptied the file (no line
    /// for this event). The name and the prefix are filled in once and
    /// can bring no colour code or placeholder of their own.
    pub fn line(&self, name: &str, prefix: &str) -> Option<String> {
        let line = self.line.as_deref()?;
        let name = clean(name);
        let prefix = clean(prefix);
        let prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!("[{prefix}] ")
        };
        let mut out = String::with_capacity(line.len() + name.len() + prefix.len());
        let mut rest = line;
        while let Some(at) = rest.find('{') {
            out.push_str(&rest[..at]);
            rest = &rest[at..];
            if let Some(after) = rest.strip_prefix("{name}") {
                out.push_str(&name);
                rest = after;
            } else if let Some(after) = rest.strip_prefix("{prefix}") {
                out.push_str(&prefix);
                rest = after;
            } else {
                out.push('{');
                rest = &rest[1..];
            }
        }
        out.push_str(rest);
        Some(out)
    }
}

/// A word from outside the owner's file, without what could change the line's look.
fn clean(word: &str) -> String {
    word.chars()
        .filter(|c| !matches!(c, '&' | '\u{a7}') && !c.is_control())
        .take(64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(text: &str) -> Format {
        Format::of(&Text::parse(text).unwrap()).unwrap()
    }

    #[test]
    fn the_starting_lines_name_the_player_and_need_no_prefix() {
        let join = format(FIRST_JOIN);
        assert!(!join.wants_prefix());
        assert_eq!(join.line("Steve", "").as_deref(), Some("&a[+] &fSteve"));
        assert_eq!(
            format(FIRST_QUIT).line("Steve", "VIP").as_deref(),
            Some("&c[-] &fSteve")
        );
    }

    #[test]
    fn the_prefix_is_bracketed_and_missing_without_a_rank() {
        let line = format("&e{prefix}{name} joined the game");
        assert!(line.wants_prefix());
        assert_eq!(
            line.line("Bo", "VIP").as_deref(),
            Some("&e[VIP] Bo joined the game")
        );
        assert_eq!(line.line("Bo", "").as_deref(), Some("&eBo joined the game"));
    }

    #[test]
    fn a_name_or_prefix_cannot_bring_colours_or_placeholders() {
        let line = format("{prefix}{name} {other} {name}");
        assert_eq!(
            line.line("&cEvil\u{a7}k{name}", "&4{prefix}\u{7}")
                .as_deref(),
            Some("[4{prefix}] cEvilk{name} {other} cEvilk{name}")
        );
        let long = "x".repeat(200);
        assert_eq!(format("{name}").line(&long, "").map(|l| l.len()), Some(64));
    }

    #[test]
    fn an_empty_file_hides_the_line_and_two_lines_are_refused() {
        assert_eq!(format("").line("Steve", ""), None);
        assert_eq!(format("\n\n").line("Steve", "VIP"), None);
        assert!(!format("").wants_prefix());
        assert_eq!(
            Format::of(&Text::parse("one\ntwo\n").unwrap()),
            Err("data_error")
        );
    }
}
