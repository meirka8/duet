use crate::predicate::{CommandContext, ContextPredicate};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Individual keybinding mapping a shortcut sequence to a command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Keybinding {
    pub key: String,
    pub command_id: String,
    pub args: Option<Value>,
    pub context: Option<ContextPredicate>,
    pub source: String,
}

/// Diagnostic warning or conflict emitted during keymap parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapDiagnostic {
    Conflict {
        key: String,
        existing_command: String,
        new_command: String,
        detail: String,
    },
    ParseWarning {
        line: usize,
        message: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum KeymapError {
    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Format error: {0}")]
    FormatError(String),
}

/// Container for resolved keybindings and lookup resolution.
#[derive(Debug, Default, Clone)]
pub struct Keymap {
    bindings: Vec<Keybinding>,
}

#[derive(Deserialize)]
struct TomlKeybinding {
    key: String,
    command: String,
    args: Option<Value>,
    context: Option<String>,
}

#[derive(Deserialize)]
struct TomlKeymap {
    #[serde(default, rename = "keybinding")]
    keybindings: Vec<TomlKeybinding>,
}

impl Keymap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bindings(&self) -> &[Keybinding] {
        &self.bindings
    }

    /// Add a keybinding and perform diagnostic conflict detection.
    pub fn add_binding(
        &mut self,
        binding: Keybinding,
        diagnostics: &mut Vec<KeymapDiagnostic>,
    ) {
        let normalized_key = Self::normalize_key(&binding.key);

        for existing in &self.bindings {
            if Self::normalize_key(&existing.key) == normalized_key {
                // Conflict detection: if context overlaps or both are unconditional
                let is_conflict = match (&existing.context, &binding.context) {
                    (None, None) => true,
                    (None, Some(_)) | (Some(_), None) => true,
                    (Some(c1), Some(c2)) => c1 == c2,
                };

                if is_conflict {
                    let diag = KeymapDiagnostic::Conflict {
                        key: normalized_key.clone(),
                        existing_command: existing.command_id.clone(),
                        new_command: binding.command_id.clone(),
                        detail: format!(
                            "Key '{}' is bound to '{}' and '{}'",
                            normalized_key, existing.command_id, binding.command_id
                        ),
                    };
                    log::warn!("[Keymap Conflict] {}", diag_detail(&diag));
                    diagnostics.push(diag);
                }
            }
        }

        self.bindings.push(binding);
    }

    /// Resolve matching keybinding for a given shortcut and execution context.
    pub fn resolve(&self, shortcut: &str, ctx: &CommandContext) -> Option<&Keybinding> {
        let norm = Self::normalize_key(shortcut);
        for binding in &self.bindings {
            if Self::normalize_key(&binding.key) == norm {
                if let Some(ref pred) = binding.context {
                    if ctx.eval(pred) {
                        return Some(binding);
                    }
                } else {
                    return Some(binding);
                }
            }
        }
        None
    }

    /// Parse keymap from TOML string.
    pub fn from_toml(toml_str: &str) -> Result<(Self, Vec<KeymapDiagnostic>), KeymapError> {
        let parsed: TomlKeymap = toml::from_str(toml_str)?;
        let mut keymap = Keymap::new();
        let mut diagnostics = Vec::new();

        for (idx, item) in parsed.keybindings.into_iter().enumerate() {
            let context = if let Some(ctx_str) = item.context {
                match ContextPredicate::parse(&ctx_str) {
                    Ok(pred) => Some(pred),
                    Err(e) => {
                        diagnostics.push(KeymapDiagnostic::ParseWarning {
                            line: idx + 1,
                            message: format!("Failed to parse context predicate '{ctx_str}': {e}"),
                        });
                        None
                    }
                }
            } else {
                None
            };

            let binding = Keybinding {
                key: item.key,
                command_id: item.command,
                args: item.args,
                context,
                source: "toml".to_string(),
            };

            keymap.add_binding(binding, &mut diagnostics);
        }

        Ok((keymap, diagnostics))
    }

    /// Parse keymap from Total Commander keymap definitions format (e.g. `F5=cm_CopyFiles`).
    pub fn from_tc_format(tc_str: &str) -> Result<(Self, Vec<KeymapDiagnostic>), KeymapError> {
        let mut keymap = Keymap::new();
        let mut diagnostics = Vec::new();

        for (line_num, line) in tc_str.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }

            if let Some((raw_key, raw_cmd)) = trimmed.split_once('=') {
                let key = raw_key.trim().to_string();
                let cmd = raw_cmd.trim().to_string();
                if key.is_empty() || cmd.is_empty() {
                    diagnostics.push(KeymapDiagnostic::ParseWarning {
                        line: line_num + 1,
                        message: format!("Invalid key=command pair: '{trimmed}'"),
                    });
                    continue;
                }

                let binding = Keybinding {
                    key: Self::parse_tc_key(&key),
                    command_id: cmd,
                    args: None,
                    context: None,
                    source: "tc".to_string(),
                };

                keymap.add_binding(binding, &mut diagnostics);
            }
        }

        Ok((keymap, diagnostics))
    }

    /// Helper to convert TC key notation (e.g. `C+C` -> `Ctrl+C`, `A+F5` -> `Alt+F5`, `CS+F5` -> `Ctrl+Shift+F5`) to canonical representation.
    pub fn parse_tc_key(tc_key: &str) -> String {
        if tc_key.contains('+') {
            let parts: Vec<&str> = tc_key.split('+').collect();
            if parts.len() == 2 {
                let modifiers = parts[0];
                let key = parts[1];
                let mut mod_str = String::new();
                if modifiers.contains('C') {
                    mod_str.push_str("Ctrl+");
                }
                if modifiers.contains('A') {
                    mod_str.push_str("Alt+");
                }
                if modifiers.contains('S') {
                    mod_str.push_str("Shift+");
                }
                return format!("{mod_str}{key}");
            }
        }
        tc_key.to_string()
    }

    /// Normalize key representations for matching (e.g. "ctrl+c" -> "Ctrl+C").
    pub fn normalize_key(key: &str) -> String {
        let parts: Vec<String> = key
            .split('+')
            .map(|p| {
                let c = p.trim().to_lowercase();
                if c == "ctrl" || c == "control" {
                    "Ctrl".to_string()
                } else if c == "alt" {
                    "Alt".to_string()
                } else if c == "shift" {
                    "Shift".to_string()
                } else {
                    if let Some(first) = c.chars().next() {
                        let uppercase_first = first.to_uppercase().to_string();
                        format!("{}{}", uppercase_first, &c[first.len_utf8()..])
                    } else {
                        c
                    }
                }
            })
            .collect();
        parts.join("+")
    }
}

fn diag_detail(diag: &KeymapDiagnostic) -> &str {
    match diag {
        KeymapDiagnostic::Conflict { detail, .. } => detail,
        KeymapDiagnostic::ParseWarning { message, .. } => message,
    }
}
