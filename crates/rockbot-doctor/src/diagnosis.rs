//! Config error classification and context extraction.

/// Result of analyzing a config parse failure.
#[derive(Debug, Clone)]
pub struct ConfigDiagnosis {
    /// The raw serde/toml error message as-is.
    pub raw_error: String,
    /// Classified error kind.
    pub kind: DiagnosisKind,
    /// The path in the TOML document where the error occurred (e.g. "gateway.port").
    pub field_path: Option<String>,
    /// Human-readable explanation in plain English (filled by AI).
    pub explanation: String,
    /// Line number in the TOML where the error was detected, if available.
    pub line: Option<usize>,
}

/// Classification of config errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosisKind {
    /// Extra field the config struct doesn't recognize.
    UnknownField,
    /// Field present but wrong type (e.g. `port = "8080"` instead of `port = 8080`).
    WrongType,
    /// Required field is absent.
    MissingField,
    /// Field present and right type, but value fails validation.
    InvalidValue,
    /// TOML syntax error (not a schema problem).
    SyntaxError,
    /// Environment variable reference (`${VAR}`) not set.
    EnvVarMissing,
    /// Catch-all for unclassifiable errors.
    Other,
}

impl DiagnosisKind {
    /// Return a short label for the error kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UnknownField => "unknown_field",
            Self::WrongType => "wrong_type",
            Self::MissingField => "missing_field",
            Self::InvalidValue => "invalid_value",
            Self::SyntaxError => "syntax_error",
            Self::EnvVarMissing => "env_var_missing",
            Self::Other => "other",
        }
    }

    /// Return a user-friendly label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::UnknownField => "Unknown Field",
            Self::WrongType => "Wrong Type",
            Self::MissingField => "Missing Field",
            Self::InvalidValue => "Invalid Value",
            Self::SyntaxError => "Syntax Error",
            Self::EnvVarMissing => "Missing Environment Variable",
            Self::Other => "Configuration Error",
        }
    }
}

/// Fast deterministic classification of a config error from the error message.
///
/// This runs before the AI model, using pattern matching on serde/toml error strings.
pub fn classify_error(error: &str) -> ConfigDiagnosis {
    let lower = error.to_lowercase();

    let (kind, field_path, line) = if lower.contains("unknown field") {
        let field = extract_quoted_value(error, "unknown field");
        let line = extract_line_number(error);
        (DiagnosisKind::UnknownField, field, line)
    } else if lower.contains("missing field") {
        let field = extract_quoted_value(error, "missing field");
        let line = extract_line_number(error);
        (DiagnosisKind::MissingField, field, line)
    } else if lower.contains("invalid type")
        || (lower.contains("expected")
            && lower.contains("found")
            && !lower.contains("expected newline")
            && !lower.contains("expected a"))
    {
        let line = extract_line_number(error);
        let field = extract_field_from_context(error);
        (DiagnosisKind::WrongType, field, line)
    } else if lower.contains("environment variable") || lower.contains("env var") {
        let var = extract_env_var(error);
        (DiagnosisKind::EnvVarMissing, var, None)
    } else if lower.contains("expected")
        || lower.contains("unexpected")
        || lower.contains("invalid")
    {
        let line = extract_line_number(error);
        (DiagnosisKind::SyntaxError, None, line)
    } else {
        let line = extract_line_number(error);
        (DiagnosisKind::Other, None, line)
    };

    ConfigDiagnosis {
        raw_error: error.to_string(),
        kind,
        field_path,
        explanation: String::new(), // Filled by AI later
        line,
    }
}

/// Extract a TOML excerpt around the error location for AI context.
pub fn extract_toml_excerpt(raw_toml: &str, diagnosis: &ConfigDiagnosis) -> String {
    if let Some(line_num) = diagnosis.line {
        let lines: Vec<&str> = raw_toml.lines().collect();
        let start = line_num.saturating_sub(3);
        let end = (line_num + 3).min(lines.len());

        lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let actual_line = start + i + 1;
                let marker = if actual_line == line_num {
                    ">>>"
                } else {
                    "   "
                };
                format!("{marker} {actual_line:4}: {l}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else if let Some(ref field_path) = diagnosis.field_path {
        // Try to find the section in the TOML
        let section = field_path.split('.').next().unwrap_or(field_path);
        let mut in_section = false;
        let mut excerpt_lines = Vec::new();

        for (i, line) in raw_toml.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_section = trimmed.contains(section);
            }
            if in_section {
                excerpt_lines.push(format!("   {:4}: {}", i + 1, line));
            }
            if excerpt_lines.len() > 15 {
                break;
            }
        }

        if excerpt_lines.is_empty() {
            truncate_toml(raw_toml, 20)
        } else {
            excerpt_lines.join("\n")
        }
    } else {
        truncate_toml(raw_toml, 20)
    }
}

/// Extract the current value of a field from raw TOML text.
pub fn extract_field_value(raw_toml: &str, field_path: &str) -> String {
    // Try parsing as a toml::Value for reliable extraction
    if let Ok(value) = raw_toml.parse::<toml::Value>() {
        let parts: Vec<&str> = field_path.split('.').collect();
        let mut current = &value;
        for part in &parts {
            match current.get(part) {
                Some(v) => current = v,
                None => return "<not found>".to_string(),
            }
        }
        return current.to_string();
    }
    "<parse error>".to_string()
}

/// Extract a quoted value following a keyword in an error message.
fn extract_quoted_value(error: &str, keyword: &str) -> Option<String> {
    let lower = error.to_lowercase();
    if let Some(pos) = lower.find(keyword) {
        let after = &error[pos + keyword.len()..];
        // Look for `name` or 'name' patterns
        if let Some(start) = after.find('`') {
            let rest = &after[start + 1..];
            if let Some(end) = rest.find('`') {
                return Some(rest[..end].to_string());
            }
        }
        if let Some(start) = after.find('\'') {
            let rest = &after[start + 1..];
            if let Some(end) = rest.find('\'') {
                return Some(rest[..end].to_string());
            }
        }
        if let Some(start) = after.find('"') {
            let rest = &after[start + 1..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// Extract a line number from an error message (e.g. "at line 5").
fn extract_line_number(error: &str) -> Option<usize> {
    let lower = error.to_lowercase();
    for pattern in &["at line ", "line "] {
        if let Some(pos) = lower.find(pattern) {
            let after = &error[pos + pattern.len()..];
            let num_str: String = after.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(n) = num_str.parse::<usize>() {
                return Some(n);
            }
        }
    }
    None
}

/// Try to extract a field path from error context.
fn extract_field_from_context(error: &str) -> Option<String> {
    // serde often includes the path like "gateway.port" or "for key \"port\""
    for pattern in &["for key \"", "for key `"] {
        if let Some(pos) = error.find(pattern) {
            let after = &error[pos + pattern.len()..];
            let end_char = if pattern.contains('`') { '`' } else { '"' };
            if let Some(end) = after.find(end_char) {
                return Some(after[..end].to_string());
            }
        }
    }
    None
}

/// Extract an environment variable name from error text.
fn extract_env_var(error: &str) -> Option<String> {
    // Look for ${VAR_NAME} pattern
    if let Some(start) = error.find("${") {
        let after = &error[start + 2..];
        if let Some(end) = after.find('}') {
            return Some(after[..end].to_string());
        }
    }
    None
}

/// Truncate TOML to first N lines.
fn truncate_toml(raw: &str, max_lines: usize) -> String {
    raw.lines()
        .take(max_lines)
        .enumerate()
        .map(|(i, l)| format!("   {:4}: {}", i + 1, l))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_classify_unknown_field() {
        let d = classify_error("unknown field `foo_bar` at line 5 column 1");
        assert_eq!(d.kind, DiagnosisKind::UnknownField);
        assert_eq!(d.field_path.as_deref(), Some("foo_bar"));
        assert_eq!(d.line, Some(5));
    }

    #[test]
    fn test_classify_missing_field() {
        let d = classify_error("missing field `port` at line 3");
        assert_eq!(d.kind, DiagnosisKind::MissingField);
        assert_eq!(d.field_path.as_deref(), Some("port"));
    }

    #[test]
    fn test_classify_wrong_type() {
        let d = classify_error(
            "invalid type: string \"8080\", expected u16 for key \"gateway.port\" at line 4",
        );
        assert_eq!(d.kind, DiagnosisKind::WrongType);
        assert_eq!(d.line, Some(4));
    }

    #[test]
    fn test_classify_env_var() {
        let d = classify_error("Environment variable ${API_KEY} is not set");
        assert_eq!(d.kind, DiagnosisKind::EnvVarMissing);
        assert_eq!(d.field_path.as_deref(), Some("API_KEY"));
    }

    #[test]
    fn test_classify_syntax() {
        let d = classify_error("expected newline, found a period at line 12 column 15");
        assert_eq!(d.kind, DiagnosisKind::SyntaxError);
        assert_eq!(d.line, Some(12));
    }

    #[test]
    fn test_extract_toml_excerpt_with_line() {
        let toml = "a = 1\nb = 2\nc = 3\nd = 4\ne = 5\nf = 6\ng = 7\n";
        let d = ConfigDiagnosis {
            raw_error: String::new(),
            kind: DiagnosisKind::Other,
            field_path: None,
            explanation: String::new(),
            line: Some(4),
        };
        let excerpt = extract_toml_excerpt(toml, &d);
        assert!(excerpt.contains(">>>"));
        assert!(excerpt.contains("d = 4"));
    }

    #[test]
    fn test_extract_field_value() {
        let toml = "[gateway]\nport = 18080\nbind_host = \"0.0.0.0\"\n";
        assert_eq!(extract_field_value(toml, "gateway.port"), "18080");
        assert_eq!(
            extract_field_value(toml, "gateway.bind_host"),
            "\"0.0.0.0\""
        );
        assert_eq!(extract_field_value(toml, "gateway.missing"), "<not found>");
    }
}
