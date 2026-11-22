//! The command that shows a text (the plugin target only).
use crate::{Shown, Texts, show};
use std::sync::Arc;
use strawberry_plugin_api::{
    Server,
    command::{Arg, ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs},
    command_wit::Number,
    commands::CommandHandler,
    common::NamedColor,
    text::TextComponent,
};

/// The script's lang files: the text of a key for a locale, with its arguments filled in.
pub type Lang = fn(locale: &str, key: &str, args: &[&str]) -> String;

struct Show {
    label: &'static str,
    texts: Arc<Texts>,
    lang: Lang,
}

impl CommandHandler for Show {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        // The sender's locale is the contract's enum (`TrTr`); `sp-lang` reads that spelling too.
        let locale = format!("{:?}", sender.get_locale());
        let page = match args.get_value("page") {
            Arg::Num(Ok(Number::Int32(value))) => Some(usize::try_from(value).unwrap_or(0)),
            Arg::Num(_) => Some(0),
            _ => None,
        };
        let refuse = |key: &str, args: &[&str]| {
            CommandError::CommandFailed(
                TextComponent::text(&(self.lang)(&locale, key, args)).color_named(NamedColor::Red),
            )
        };
        match show(self.texts.of(&locale), page) {
            Shown::Empty => Err(refuse("empty", &[])),
            Shown::NoPage(pages) => Err(refuse("no_page", &[&pages.to_string()])),
            Shown::Page(lines, number, pages) => {
                for line in lines {
                    sender.send_message(TextComponent::from_legacy_string_with_code(line, '&'));
                }
                if pages > 1 {
                    let (number_text, pages_text) = (number.to_string(), pages.to_string());
                    let foot = if number < pages {
                        (self.lang)(
                            &locale,
                            "next",
                            &[
                                &number_text,
                                &pages_text,
                                self.label,
                                &(number + 1).to_string(),
                            ],
                        )
                    } else {
                        (self.lang)(&locale, "page", &[&number_text, &pages_text])
                    };
                    sender.send_message(TextComponent::text(&foot).color_named(NamedColor::Gray));
                }
                Ok(1)
            }
        }
    }
}

/// `/<label> [page]`: shows `texts` to whoever typed it, in his language. The script registers the command
/// with its own permission.
pub fn command(label: &'static str, description: &str, texts: Arc<Texts>, lang: Lang) -> Command {
    let show = || Show {
        label,
        texts: Arc::clone(&texts),
        lang,
    };
    Command::new(&[label.into()], description)
        .execute(show())
        .then(
            CommandNode::argument("page", &ArgumentType::Integer((Some(1), Some(1000))))
                .execute(show()),
        )
}
