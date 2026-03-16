//! `rockbot-shell` — shell command execution stub for RockBot TUI.
//!
//! Provides the `/shell` chat command to run shell commands inline.

use rockbot_chat::{ChatCommand, CommandContext, CommandInfo, CommandResult};

/// Register shell chat commands.
pub fn register_chat_commands(registry: &mut rockbot_chat::ChatCommandRegistry) {
    registry.register(Box::new(ShellCommand));
}

struct ShellCommand;

impl ChatCommand for ShellCommand {
    fn info(&self) -> CommandInfo {
        CommandInfo {
            name: "shell",
            aliases: &["sh", "!"],
            description: "Run a shell command",
            usage: "/shell <command>",
        }
    }

    fn execute(&self, args: &str, _ctx: &CommandContext) -> CommandResult {
        let cmd = args.trim();
        if cmd.is_empty() {
            CommandResult::Handled("Usage: /shell <command>".to_string())
        } else {
            // Stub: actual execution will be wired later with security checks
            CommandResult::Handled(format!("Shell execution not yet implemented: {cmd}"))
        }
    }
}
