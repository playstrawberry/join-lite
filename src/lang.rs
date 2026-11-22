//! The script's texts: `lang/{en,fr,es,tr}.lang`, embedded and checked for the same keys while it compiles.
//! The join and quit lines themselves are the owner's own words (`join.txt`, `quit.txt`), not texts of the script.
use sp_lang::{Catalog, Language};

static LANG: Catalog = sp_lang::catalog!("join-lite");

/// The text of `key` in the language of `locale` (English for any other).
pub fn text(locale: &str, key: &str) -> String {
    let raw = LANG
        .get_or_en(Language::of_locale(locale), key)
        .unwrap_or_default();
    sp_lang::render(raw, &[], "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_the_script_names_has_a_text_in_every_language() {
        let own = ["description", "data_error", "save_error"];
        for key in own {
            for language in Language::ALL {
                assert!(
                    LANG.get(language, key).is_some_and(|t| !t.is_empty()),
                    "{key} in {}",
                    language.code()
                );
            }
        }
        assert_eq!(LANG.len(), own.len(), "a key nobody names");
    }
}
