use crate::{
    lang,
    model::{FIRST_JOIN, FIRST_QUIT, Format},
};
use sp_pages::Texts;
use std::path::PathBuf;
use strawberry_plugin_api::{
    Context, EventHandler, Player, Plugin, PluginMetadata, Result, Server,
    events::{FailureVerdict, HandlerOptions, HandlerPriority, PlayerJoinEvent, PlayerLeaveEvent},
    events_wit::{PlayerJoinEventData, PlayerLeaveEventData},
    permissions, register_plugin,
    text::TextComponent,
};

/// The prefix the rank script gives this player; empty when it is not installed or he has none. Asked only
/// for a line that names `{prefix}`.
fn prefix_of(player: &Player) -> String {
    sp_ranks::get(&[player.get_id().to_string()])
        .and_then(|answer| answer.ranks.into_iter().next())
        .map(|(_, rank)| rank.prefix)
        .unwrap_or_default()
}

/// The owner's line for this player, or `None` when the owner hides it.
fn line(format: &Format, player: &Player) -> Option<TextComponent> {
    let prefix = if format.wants_prefix() {
        prefix_of(player)
    } else {
        String::new()
    };
    format
        .line(&player.get_name(), &prefix)
        .map(|text| TextComponent::from_legacy_string_with_code(&text, '&'))
}

struct Line(Format);

impl EventHandler<PlayerJoinEvent> for Line {
    fn handle(&self, _server: Server, mut event: PlayerJoinEventData) -> PlayerJoinEventData {
        match line(&self.0, &event.player) {
            Some(text) => event.join_message = text,
            // No line: the event's line is taken back, the join itself is untouched.
            None => event.cancelled = true,
        }
        event
    }
}

impl EventHandler<PlayerLeaveEvent> for Line {
    fn handle(&self, _server: Server, mut event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        match line(&self.0, &event.player) {
            Some(text) => event.leave_message = text,
            None => event.cancelled = true,
        }
        event
    }
}

struct JoinLite;

impl Plugin for JoinLite {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "join-lite".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Strawberry Interactive LLC".into()],
            description: lang::text("en", "description"),
            dependencies: vec![],
            permissions: vec![
                permissions::FS_READ_DATA.into(),
                permissions::FS_WRITE_DATA.into(),
            ],
        }
    }

    fn on_load(&self, context: Context) -> Result<()> {
        let dir = PathBuf::from(context.get_data_folder());
        std::fs::create_dir_all(&dir).map_err(|_| lang::text("en", "data_error"))?;
        let load = |stem: &str, first: &str| -> Result<Format> {
            let texts = Texts::load_with(&dir, stem, first).map_err(|key| lang::text("en", key))?;
            Format::of(texts.main()).map_err(|key| lang::text("en", key))
        };
        let join = load("join", FIRST_JOIN)?;
        let quit = load("quit", FIRST_QUIT)?;
        // Before the scripts that decide whether a line is shown at all (a hidden staff member's join is
        // cancelled after this one ran) and before the ones that decorate it.
        let options = HandlerOptions {
            priority: HandlerPriority::Normal,
            blocking: true,
            ignore_cancelled: true,
            on_failure: FailureVerdict::Allow,
        };
        context.register_event_handler_with_options::<PlayerJoinEvent, _>(Line(join), options)?;
        context.register_event_handler_with_options::<PlayerLeaveEvent, _>(Line(quit), options)?;
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<()> {
        Ok(())
    }
}

register_plugin!(JoinLite);
