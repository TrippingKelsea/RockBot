//! TOML repair — surgical fixes that preserve comments and formatting.

use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

/// A concrete fix to apply to a TOML config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DoctorFix {
    /// Remove a field entirely.
    RemoveField {
        /// Dotted path, e.g. `["gateway", "old_field"]`.
        path: Vec<String>,
    },
    /// Set a field to a new value.
    SetField {
        /// Dotted path.
        path: Vec<String>,
        /// The new TOML value as a string literal (e.g. `"8080"`, `true`, `"\"hello\""`).
        new_value: String,
    },
    /// Add a new field that doesn't exist yet.
    AddField {
        /// Dotted path.
        path: Vec<String>,
        /// The TOML value.
        value: String,
    },
}

impl DoctorFix {
    /// Human-readable description of this fix.
    pub fn describe(&self) -> String {
        match self {
            Self::RemoveField { path } => {
                format!("Remove field `{}`", path.join("."))
            }
            Self::SetField { path, new_value } => {
                format!("Set `{}` = {}", path.join("."), new_value)
            }
            Self::AddField { path, value } => {
                format!("Add `{}` = {}", path.join("."), value)
            }
        }
    }
}

/// Apply a fix to raw TOML text, returning the patched text.
///
/// Uses `toml_edit` to preserve comments and formatting.
pub fn apply_fix(raw_toml: &str, fix: &DoctorFix) -> anyhow::Result<String> {
    let mut doc: DocumentMut = raw_toml
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse TOML for repair: {e}"))?;

    match fix {
        DoctorFix::RemoveField { path } => {
            remove_path(&mut doc, path)?;
        }
        DoctorFix::SetField { path, new_value } => {
            set_path(&mut doc, path, new_value)?;
        }
        DoctorFix::AddField { path, value } => {
            set_path(&mut doc, path, value)?;
        }
    }

    Ok(doc.to_string())
}

/// Remove a field at the given dotted path.
fn remove_path(doc: &mut DocumentMut, path: &[String]) -> anyhow::Result<()> {
    if path.is_empty() {
        anyhow::bail!("Empty field path");
    }

    if path.len() == 1 {
        doc.remove(&path[0]);
        return Ok(());
    }

    // Navigate to the parent table
    let mut current = doc.as_table_mut() as &mut dyn toml_edit::TableLike;
    for segment in &path[..path.len() - 1] {
        current = current
            .get_mut(segment)
            .and_then(|v| v.as_table_like_mut())
            .ok_or_else(|| anyhow::anyhow!("Path segment `{segment}` not found"))?;
    }

    let last = &path[path.len() - 1];
    current.remove(last);
    Ok(())
}

/// Set a field at the given dotted path, creating parent tables as needed.
fn set_path(doc: &mut DocumentMut, path: &[String], value: &str) -> anyhow::Result<()> {
    if path.is_empty() {
        anyhow::bail!("Empty field path");
    }

    // Parse the value as a TOML value
    let parsed_value: toml_edit::Value = value
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid TOML value `{value}`: {e}"))?;

    if path.len() == 1 {
        doc[&path[0]] = toml_edit::Item::Value(parsed_value);
        return Ok(());
    }

    // Ensure parent tables exist
    let mut current: &mut toml_edit::Item = doc.as_item_mut();
    for segment in &path[..path.len() - 1] {
        if current.get(segment).is_none() {
            current[segment] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        current = &mut current[segment];
    }

    let last = &path[path.len() - 1];
    current[last] = toml_edit::Item::Value(parsed_value);
    Ok(())
}

/// Parse the AI model's fix suggestion into a `DoctorFix`.
///
/// Expected formats:
/// - `SET: <value>`
/// - `REMOVE`
/// - `ADD: <section.field = value>`
/// - `CANNOT_FIX: <reason>`
pub fn parse_fix_suggestion(output: &str, field_path: &str) -> Option<DoctorFix> {
    let trimmed = output.trim();

    // Scan line by line for the first recognized directive
    for line in trimmed.lines() {
        let line = line.trim();

        if let Some(rest) = line.strip_prefix("SET:") {
            let value = rest.trim().to_string();
            if !value.is_empty() {
                let path = field_path.split('.').map(String::from).collect();
                return Some(DoctorFix::SetField {
                    path,
                    new_value: value,
                });
            }
        } else if line == "REMOVE" {
            let path = field_path.split('.').map(String::from).collect();
            return Some(DoctorFix::RemoveField { path });
        } else if let Some(rest) = line.strip_prefix("ADD:") {
            let rest = rest.trim();
            // Parse "section.field = value"
            if let Some(eq_pos) = rest.find('=') {
                let add_path = rest[..eq_pos].trim();
                let add_value = rest[eq_pos + 1..].trim().to_string();
                let path = add_path.split('.').map(String::from).collect();
                return Some(DoctorFix::AddField {
                    path,
                    value: add_value,
                });
            }
        } else if line.starts_with("CANNOT_FIX:") {
            return None;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn test_apply_set_field() {
        let toml = "[gateway]\nport = 8080\n";
        let fix = DoctorFix::SetField {
            path: vec!["gateway".into(), "port".into()],
            new_value: "18080".into(),
        };
        let result = apply_fix(toml, &fix).unwrap();
        assert!(result.contains("port = 18080"));
    }

    #[test]
    fn test_apply_remove_field() {
        let toml = "[gateway]\nport = 8080\nold_field = true\n";
        let fix = DoctorFix::RemoveField {
            path: vec!["gateway".into(), "old_field".into()],
        };
        let result = apply_fix(toml, &fix).unwrap();
        assert!(!result.contains("old_field"));
        assert!(result.contains("port = 8080"));
    }

    #[test]
    fn test_apply_add_field() {
        let toml = "[gateway]\nport = 8080\n";
        let fix = DoctorFix::AddField {
            path: vec!["gateway".into(), "bind_host".into()],
            value: "\"0.0.0.0\"".into(),
        };
        let result = apply_fix(toml, &fix).unwrap();
        assert!(result.contains("bind_host = \"0.0.0.0\""));
    }

    #[test]
    fn test_parse_fix_set() {
        let fix = parse_fix_suggestion("SET: 18080", "gateway.port");
        let fix = fix.unwrap();
        match fix {
            DoctorFix::SetField { path, new_value } => {
                assert_eq!(path, vec!["gateway", "port"]);
                assert_eq!(new_value, "18080");
            }
            _ => panic!("Expected SetField"),
        }
    }

    #[test]
    fn test_parse_fix_remove() {
        let fix = parse_fix_suggestion("REMOVE", "gateway.old_field");
        let fix = fix.unwrap();
        assert!(matches!(fix, DoctorFix::RemoveField { .. }));
    }

    #[test]
    fn test_parse_fix_add() {
        let fix = parse_fix_suggestion("ADD: security.sandbox.enabled = true", "security.sandbox");
        let fix = fix.unwrap();
        match fix {
            DoctorFix::AddField { path, value } => {
                assert_eq!(path, vec!["security", "sandbox", "enabled"]);
                assert_eq!(value, "true");
            }
            _ => panic!("Expected AddField"),
        }
    }

    #[test]
    fn test_parse_fix_cannot() {
        let fix = parse_fix_suggestion("CANNOT_FIX: ambiguous error", "gateway.port");
        assert!(fix.is_none());
    }

    #[test]
    fn test_describe() {
        let fix = DoctorFix::SetField {
            path: vec!["gateway".into(), "port".into()],
            new_value: "18080".into(),
        };
        assert_eq!(fix.describe(), "Set `gateway.port` = 18080");
    }
}
