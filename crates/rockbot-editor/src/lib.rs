//! `rockbot-editor` — embedded editor stub for RockBot TUI.
//!
//! Provides the `/editor` chat command to open an inline editor.

use rockbot_chat::{ChatCommand, CommandContext, CommandInfo, CommandResult};

fn sanitize_overlay_component(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '/' | '.' | '_' | '-' => ch,
            _ => '_',
        })
        .collect()
}

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
            let overlay_target = sanitize_overlay_component(file);
            CommandResult::Action(rockbot_chat::CommandAction::ShowOverlay(format!(
                "editor:{overlay_target}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_overlay_component;

    #[test]
    fn sanitize_overlay_component_removes_unsafe_chars() {
        assert_eq!(
            sanitize_overlay_component("../notes\n:secret"),
            "../notes__secret"
        );
    }
}
