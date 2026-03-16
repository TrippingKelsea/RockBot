//! Prompt templates for the doctor AI.
//!
//! Follows the same structured-output pattern as `rockbot-overseer/judgments.rs`:
//! prompts request a parseable token prefix (`SET:`, `REMOVE`, etc.) so parsing
//! is deterministic.

/// Explain what's wrong with the config in plain English.
pub fn diagnose_prompt(toml_excerpt: &str, error: &str, field_path: &str) -> String {
    format!(
        "<|im_start|>system\n\
         You are a configuration doctor for the RockBot application.\n\
         Your job is to explain configuration errors in clear, simple language.\n\
         Be concise (1-3 sentences). Do NOT repeat the error message.\n\
         Be specific about the field name and what the user should do.\n\
         <|im_end|>\n\
         <|im_start|>user\n\
         A RockBot config file has an error.\n\
         Field: {field_path}\n\
         Error: {error}\n\
         TOML context:\n{toml_excerpt}\n\n\
         Explain what is wrong and how to fix it.\n\
         <|im_end|>\n\
         <|im_start|>assistant\n"
    )
}

/// Suggest a concrete fix for a config error.
///
/// The model is instructed to respond with one of:
/// - `SET: <value>` — the corrected TOML value
/// - `REMOVE` — the field should be deleted
/// - `ADD: <section.field = value>` — a required field is missing
/// - `CANNOT_FIX: <reason>` — the model can't determine a fix
pub fn fix_prompt(field_path: &str, current_value: &str, error: &str, kind: &str) -> String {
    format!(
        "<|im_start|>system\n\
         You are a configuration repair expert for RockBot.\n\
         Respond with EXACTLY one line in one of these formats:\n\
         SET: <corrected_toml_value>\n\
         REMOVE\n\
         ADD: <section.field = value>\n\
         CANNOT_FIX: <reason>\n\
         No explanation, just the fix line.\n\
         <|im_end|>\n\
         <|im_start|>user\n\
         Field: {field_path}\n\
         Current value: {current_value}\n\
         Error type: {kind}\n\
         Error: {error}\n\
         <|im_end|>\n\
         <|im_start|>assistant\n"
    )
}

/// Suggest a fix, injecting recent successful fixes as few-shot examples.
///
/// `examples` is a slice of `(field_pattern, error_kind, fix_description)` tuples
/// drawn from the learned store.
pub fn fix_prompt_with_examples(
    field_path: &str,
    current_value: &str,
    error: &str,
    kind: &str,
    examples: &[(String, String, String)],
) -> String {
    let mut examples_section = String::from("Previous successful fixes:\n");
    for (field_pattern, error_kind, fix_description) in examples {
        examples_section.push_str(&format!(
            "- Field `{field_pattern}`, error type: {error_kind} → {fix_description}\n"
        ));
    }

    format!(
        "<|im_start|>system\n\
         You are a configuration repair expert for RockBot.\n\
         Respond with EXACTLY one line in one of these formats:\n\
         SET: <corrected_toml_value>\n\
         REMOVE\n\
         ADD: <section.field = value>\n\
         CANNOT_FIX: <reason>\n\
         No explanation, just the fix line.\n\
         <|im_end|>\n\
         <|im_start|>user\n\
         {examples_section}\n\
         Field: {field_path}\n\
         Current value: {current_value}\n\
         Error type: {kind}\n\
         Error: {error}\n\
         <|im_end|>\n\
         <|im_start|>assistant\n"
    )
}

/// Detect potentially outdated config keys.
///
/// The model should respond with lines like:
/// - `DEPRECATED: <old.path> -> <new.path>`
/// - `NONE` — if no deprecated fields found
pub fn migration_prompt(raw_toml: &str, known_renames: &str) -> String {
    // Truncate the TOML to avoid overwhelming the context
    let toml_truncated: String = raw_toml.lines().take(80).collect::<Vec<_>>().join("\n");

    format!(
        "<|im_start|>system\n\
         You are a config migration assistant for RockBot.\n\
         Scan the config for deprecated or renamed fields.\n\
         Respond with one line per deprecated field found:\n\
         DEPRECATED: <old.path> -> <new.path>\n\
         Or if none found:\n\
         NONE\n\
         Only list fields actually present in the config.\n\
         <|im_end|>\n\
         <|im_start|>user\n\
         Known renames (old -> new):\n{known_renames}\n\n\
         Config:\n```toml\n{toml_truncated}\n```\n\
         <|im_end|>\n\
         <|im_start|>assistant\n"
    )
}
