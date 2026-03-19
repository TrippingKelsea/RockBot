//! `rockbot-shell` — shell command execution stub for RockBot TUI.
//!
//! Provides the `/shell` chat command to run shell commands inline.

use rockbot_chat::{ChatCommand, CommandContext, CommandInfo, CommandResult};

fn sanitize_terminal_echo(input: &str) -> String {
    input
        .chars()
        .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
        .collect()
}

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
            let safe_cmd = sanitize_terminal_echo(cmd);
            CommandResult::Handled(format!("Shell execution not yet implemented: {safe_cmd}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_terminal_echo;

    #[test]
    fn sanitize_terminal_echo_strips_escape_sequences() {
        assert_eq!(
            sanitize_terminal_echo("echo ok\u{1b}[31mboom"),
            "echo ok[31mboom"
        );
    }
}
