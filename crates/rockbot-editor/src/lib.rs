//! `rockbot-editor` — embedded editor stub for RockBot TUI.
//!
//! Provides the `/editor` chat command to open an inline editor.

use rockbot_chat::{ChatCommand, CommandContext, CommandInfo, CommandResult};

/// Register editor chat commands.
pub fn register_chat_commands(registry: &mut rockbot_chat::ChatCommandRegistry) {
    registry.register(Box::new(EditorCommand));
}

struct EditorCommand;

impl ChatCommand for EditorCommand {
    fn info(&self) -> CommandInfo {
        CommandInfo {
            name: "editor",
            aliases: &["edit"],
            description: "Open the inline editor",
            usage: "/editor [file]",
        }
    }

    fn execute(&self, args: &str, _ctx: &CommandContext) -> CommandResult {
        let file = args.trim();
        if file.is_empty() {
            CommandResult::Handled("Usage: /editor [file] — opens an inline editor".to_string())
        } else {
            CommandResult::Action(rockbot_chat::CommandAction::ShowOverlay(format!(
                "editor:{file}"
            )))
        }
    }
}
