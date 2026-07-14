//! DX Config File Formatter
//!
//! Produces beautifully formatted LLM-format output for `.dx` config files.
//! Supports two styles:
//! - **YAML-style** (`:` delimiters, comma-separated table data)
//! - **Parens-style** (`()` delimiters, space-separated table data)
//!
//! Both styles align `=` signs and group entries with blank lines.

use crate::llm::types::{DxDocument, DxLlmValue, DxSection, EntryRef};
use indexmap::IndexMap;

/// Config file formatting style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigStyle {
    /// YAML-style: `config:`, `hikes[headers]:`, comma-separated cells
    Yaml,
    /// Parens-style: `config(...)`, `hikes[headers](...)`, space-separated cells
    Parens,
}

/// Produces beautifully formatted DX config file output with aligned `=` signs.
pub struct ConfigFormatter {
    style: ConfigStyle,
}

impl ConfigFormatter {
    /// Create a new formatter with the given style
    #[must_use]
    pub const fn new(style: ConfigStyle) -> Self {
        Self { style }
    }

    /// Format a document in beautifully formatted config style
    #[must_use]
    pub fn format(&self, doc: &DxDocument) -> String {
        let entries = self.collect_entries(doc);
        let max_key_len = self.max_key_length(&entries);

        let mut output = String::new();
        let mut last_was_section = false;

        for entry in &entries {
            match entry {
                ConfigEntry::Context { key, value } => {
                    if last_was_section {
                        output.push('\n');
                    }
                    self.format_context_entry(&mut output, key, value, max_key_len);
                    last_was_section = false;
                }
                ConfigEntry::Section { name, section } => {
                    output.push('\n');
                    self.format_section(&mut output, name, section);
                    output.push('\n');
                    last_was_section = true;
                }
            }
        }

        output.trim_end().to_string()
    }

    /// Collect entries in order, separating context and sections
    fn collect_entries(&self, doc: &DxDocument) -> Vec<ConfigEntry> {
        let mut entries = Vec::new();

        if doc.entry_order.is_empty() {
            // Fallback: context first, then sections
            for (key, value) in &doc.context {
                entries.push(ConfigEntry::Context {
                    key: key.clone(),
                    value: value.clone(),
                });
            }
            for (id, section) in &doc.sections {
                let name = doc
                    .section_names
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.to_string());
                entries.push(ConfigEntry::Section {
                    name,
                    section: section.clone(),
                });
            }
        } else {
            for entry_ref in &doc.entry_order {
                match entry_ref {
                    EntryRef::Context(key) => {
                        if let Some(value) = doc.context.get(key) {
                            entries.push(ConfigEntry::Context {
                                key: key.clone(),
                                value: value.clone(),
                            });
                        }
                    }
                    EntryRef::Section(id) => {
                        if let Some(section) = doc.sections.get(id) {
                            let name = doc
                                .section_names
                                .get(id)
                                .cloned()
                                .unwrap_or_else(|| id.to_string());
                            entries.push(ConfigEntry::Section {
                                name,
                                section: section.clone(),
                            });
                        }
                    }
                }
            }
        }

        entries
    }

    /// Calculate the maximum key length for `=` alignment
    fn max_key_length(&self, entries: &[ConfigEntry]) -> usize {
        entries
            .iter()
            .filter_map(|e| match e {
                ConfigEntry::Context { key, value: _ } => Some(key.len()),
                ConfigEntry::Section { .. } => None,
            })
            .max()
            .unwrap_or(0)
    }

    fn format_context_entry(
        &self,
        output: &mut String,
        key: &str,
        value: &DxLlmValue,
        max_key_len: usize,
    ) {
        match value {
            DxLlmValue::Obj(fields) => {
                self.format_object(output, key, fields, max_key_len);
            }
            DxLlmValue::Arr(items) => {
                self.format_array(output, key, items, max_key_len);
            }
            _ => {
                let padding = " ".repeat(max_key_len - key.len());
                let val = self.serialize_value(value);
                output.push_str(&format!("{key}{padding} = {val}"));
                output.push('\n');
            }
        }
    }

    fn format_object(
        &self,
        output: &mut String,
        key: &str,
        fields: &IndexMap<String, DxLlmValue>,
        _max_key_len: usize,
    ) {
        match self.style {
            ConfigStyle::Yaml => {
                output.push_str(&format!("{key}:\n"));
                for (k, v) in fields {
                    output.push_str("  ");
                    if let DxLlmValue::Arr(items) = v {
                        let serialized = self.join_array_items(items, false);
                        output.push_str(&format!("{k} = {serialized}\n"));
                    } else {
                        output.push_str(&format!("{} = {}\n", k, self.serialize_value(v)));
                    }
                }
            }
            ConfigStyle::Parens => {
                output.push_str(&format!("{key}(\n"));
                for (k, v) in fields {
                    output.push_str("  ");
                    if let DxLlmValue::Arr(items) = v {
                        let serialized = self.join_array_items(items, false);
                        output.push_str(&format!("{k} = {serialized}\n"));
                    } else {
                        output.push_str(&format!("{} = {}\n", k, self.serialize_value(v)));
                    }
                }
                output.push_str(")\n");
            }
        }
    }

    fn format_array(
        &self,
        output: &mut String,
        key: &str,
        items: &[DxLlmValue],
        max_key_len: usize,
    ) {
        let padding = " ".repeat(max_key_len.saturating_sub(key.len()));
        let serialized = self.join_array_items(items, false);
        output.push_str(&format!("{key}{padding} = {serialized}\n"));
    }

    fn format_section(&self, output: &mut String, name: &str, section: &DxSection) {
        match self.style {
            ConfigStyle::Yaml => {
                // Schema uses comma-separated headers
                let schema_str = section.schema.join(", ");
                output.push_str(&format!("{name}[{schema_str}]:\n"));
                let use_commas = true; // YAML always uses commas
                for row in &section.rows {
                    output.push_str("  ");
                    let values: Vec<String> = row
                        .iter()
                        .map(|v| self.serialize_table_value(v, use_commas))
                        .collect();
                    output.push_str(&values.join(", "));
                    output.push('\n');
                }
            }
            ConfigStyle::Parens => {
                // Schema uses space-separated headers
                let schema_str = section.schema.join(" ");
                output.push_str(&format!("{name}[{schema_str}](\n"));
                let use_commas = false; // Parens uses spaces
                for row in &section.rows {
                    output.push_str("  ");
                    let values: Vec<String> = row
                        .iter()
                        .map(|v| self.serialize_table_value(v, use_commas))
                        .collect();
                    output.push_str(&values.join(" "));
                    output.push('\n');
                }
                output.push(')');
            }
        }
    }

    fn join_array_items(&self, items: &[DxLlmValue], force_commas: bool) -> String {
        let has_spaces = items
            .iter()
            .any(|v| matches!(v, DxLlmValue::Str(s) if s.contains(' ')));
        let use_commas = force_commas || has_spaces;
        let sep = if use_commas { ", " } else { " " };

        if items.len() == 1 {
            let s = self.serialize_value(&items[0]);
            return if s.contains(',') || s.contains(' ') {
                format!("\"{}\"", s.replace('"', "\\\""))
            } else {
                s
            };
        }

        let parts: Vec<String> = items
            .iter()
            .map(|v| {
                let s = self.serialize_value(v);
                if use_commas && s.contains(',') {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else if !use_commas && s.contains(' ') {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else {
                    s
                }
            })
            .collect();
        parts.join(sep)
    }

    fn serialize_value(&self, value: &DxLlmValue) -> String {
        match value {
            DxLlmValue::Bool(true) => "true".to_string(),
            DxLlmValue::Bool(false) => "false".to_string(),
            DxLlmValue::Null => "null".to_string(),
            DxLlmValue::Int(i) => format!("{i}"),
            DxLlmValue::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            DxLlmValue::Str(s) => {
                if s.contains(',') {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else {
                    s.clone()
                }
            }
            DxLlmValue::Arr(items) => self.join_array_items(items, false),
            DxLlmValue::Obj(fields) => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, self.serialize_value(v)))
                    .collect();
                fields_str.join(", ")
            }
            DxLlmValue::Ref(key) => format!("^{key}"),
        }
    }

    fn serialize_table_value(&self, value: &DxLlmValue, use_commas: bool) -> String {
        match value {
            DxLlmValue::Bool(true) => "true".to_string(),
            DxLlmValue::Bool(false) => "false".to_string(),
            DxLlmValue::Null => "null".to_string(),
            DxLlmValue::Int(i) => format!("{i}"),
            DxLlmValue::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            DxLlmValue::Str(s) => {
                let needs_quoting = if use_commas {
                    s.contains(',')
                } else {
                    s.contains(' ')
                };
                if needs_quoting {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else {
                    s.clone()
                }
            }
            DxLlmValue::Arr(items) => {
                let serialized: Vec<String> = items
                    .iter()
                    .map(|item| self.serialize_table_value(item, use_commas))
                    .collect();
                let sep = if use_commas { ", " } else { " " };
                serialized.join(sep)
            }
            DxLlmValue::Obj(fields) => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, self.serialize_table_value(v, use_commas)))
                    .collect();
                let sep = if use_commas { ", " } else { " " };
                format!("({})", fields_str.join(sep))
            }
            DxLlmValue::Ref(key) => format!("^{key}"),
        }
    }
}

impl Default for ConfigFormatter {
    fn default() -> Self {
        Self::new(ConfigStyle::Parens)
    }
}

enum ConfigEntry {
    Context { key: String, value: DxLlmValue },
    Section { name: String, section: DxSection },
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn make_test_doc() -> DxDocument {
        let mut doc = DxDocument::new();
        doc.context.insert(
            "task".to_string(),
            DxLlmValue::Str("Our favorite hikes together".to_string()),
        );
        doc.context.insert(
            "location".to_string(),
            DxLlmValue::Str("Boulder".to_string()),
        );
        doc.context.insert(
            "season".to_string(),
            DxLlmValue::Str("spring_2025".to_string()),
        );
        doc.context.insert(
            "friends".to_string(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("ana".to_string()),
                DxLlmValue::Str("luis".to_string()),
                DxLlmValue::Str("sam".to_string()),
            ]),
        );

        let mut config = IndexMap::new();
        config.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        config.insert("port".to_string(), DxLlmValue::Num(8080.0));
        doc.context
            .insert("config".to_string(), DxLlmValue::Obj(config));

        let mut section = DxSection::new(vec![
            "id".to_string(),
            "name".to_string(),
            "distanceKm".to_string(),
            "elevationGain".to_string(),
            "companion".to_string(),
            "wasSunny".to_string(),
        ]);
        section.rows.push(vec![
            DxLlmValue::Num(1.0),
            DxLlmValue::Str("Blue Lake Trail".to_string()),
            DxLlmValue::Num(7.5),
            DxLlmValue::Num(320.0),
            DxLlmValue::Str("ana".to_string()),
            DxLlmValue::Bool(true),
        ]);
        section.rows.push(vec![
            DxLlmValue::Num(2.0),
            DxLlmValue::Str("Ridge Overlook".to_string()),
            DxLlmValue::Num(9.2),
            DxLlmValue::Num(540.0),
            DxLlmValue::Str("luis".to_string()),
            DxLlmValue::Bool(false),
        ]);
        section.rows.push(vec![
            DxLlmValue::Num(3.0),
            DxLlmValue::Str("Wildflower Loop, North".to_string()),
            DxLlmValue::Num(5.1),
            DxLlmValue::Num(180.0),
            DxLlmValue::Str("sam".to_string()),
            DxLlmValue::Bool(true),
        ]);
        doc.sections.insert('h', section);
        doc.section_names.insert('h', "hikes".to_string());

        doc
    }

    #[test]
    fn test_yaml_style_config() {
        let formatter = ConfigFormatter::new(ConfigStyle::Yaml);
        let doc = make_test_doc();
        let output = formatter.format(&doc);
        // eprintln!("=== YAML STYLE ===\n{}", output);

        assert!(output.contains("task     = "));
        assert!(output.contains("location = "));
        assert!(output.contains("season   = "));
        assert!(output.contains("config:"));
        assert!(output.contains("  host = localhost"));
        assert!(
            output.contains("hikes[id, name, distanceKm, elevationGain, companion, wasSunny]:")
        );
        assert!(output.contains("  1, Blue Lake Trail, 7.5, 320, ana, true"));
    }

    #[test]
    fn test_parens_style_config() {
        let formatter = ConfigFormatter::new(ConfigStyle::Parens);
        let doc = make_test_doc();
        let output = formatter.format(&doc);
        // eprintln!("=== PARENS STYLE ===\n{}", output);

        assert!(output.contains("task     = "));
        assert!(output.contains("config("));
        assert!(output.contains("  host = localhost"));
        assert!(output.contains("  port = 8080"));
        assert!(output.contains(")"));
        assert!(output.contains("hikes[id name distanceKm elevationGain companion wasSunny]("));
        assert!(output.contains("  1 \"Blue Lake Trail\" 7.5 320 ana true"));
    }

    #[test]
    fn test_empty_document() {
        let formatter = ConfigFormatter::new(ConfigStyle::Parens);
        let doc = DxDocument::new();
        let output = formatter.format(&doc);
        assert!(output.is_empty());
    }
}
