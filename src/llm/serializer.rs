//! DX Serializer LLM Format
//!
//! Serializes `DxDocument` to the adaptive LLM format with spaces around `=`.
//! Uses smart separator selection: commas for sentence-heavy data, spaces for simple tokens.
//!
//! ## LLM Format Syntax (Adaptive Separators)
//!
//! ```text
//! # Key-Value Pairs
//! name = MyApp
//! port = 8080
//! description = "Multi word string"
//!
//! # Arrays (adaptive separators, no brackets)
//! tags = rust performance serialization    # space-sep for simple tokens
//! editors = neovim, zed, "firebase studio" # comma-sep when values have spaces
//!
//! # Objects (parentheses, multi-line)
//! config(
//!   host = localhost
//!   port = 5432
//!   debug = true
//! )
//!
//! # Tables (wrapped dataframes - deterministic parsing, adaptive separators)
//! users[id, name, email](
//!   1, Alice, alice@ex.com
//!   2, Bob, bob@ex.com
//! )
//!
//! # Multi-word values don't need quotes with commas
//! employees[id, name, dept](
//!   1, James Smith, Engineering
//!   2, Mary Johnson, Research and Development
//! )
//! ```
//!
//! ## Why DX Beats TOON
//!
//! 1. Deterministic parsing - Wrapped dataframes `[headers](rows)` eliminate ambiguity
//! 2. Adaptive separators - commas for sentences, spaces for simple tokens
//! 3. Quoted strings for ambiguous values
//! 4. Mental model alignment - `()` objects, `[headers](rows)` tables

use crate::llm::types::{DxDocument, DxLlmValue, DxSection};
use indexmap::IndexMap;

/// Configuration options for the serializer
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SerializerConfig {
    /// Compact mode: single-line sections (rows space-separated on one line)
    pub compact: bool,
}

/// Serialize `DxDocument` to Dx Serializer format
pub struct LlmSerializer {
    config: SerializerConfig,
}

impl LlmSerializer {
    /// Create a new serializer with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: SerializerConfig::default(),
        }
    }

    /// Create a new serializer with custom configuration
    #[must_use]
    pub const fn with_config(config: SerializerConfig) -> Self {
        Self { config }
    }

    /// Serialize `DxDocument` to Dx Serializer format string
    #[must_use]
    pub fn serialize(&self, doc: &DxDocument) -> String {
        let mut output = String::new();

        // If entry_order is populated, use it to maintain original order
        if doc.entry_order.is_empty() {
            // Fallback: serialize in default order (context then sections)
            for (key, value) in &doc.context {
                match value {
                    DxLlmValue::Obj(_) | DxLlmValue::Arr(_) => {
                        let entry = self.serialize_context_entry(key, value);
                        output.push_str(&entry);
                        output.push('\n');
                    }
                    _ => {
                        output.push_str(&format!("{} = {}", key, self.serialize_single_value(value)));
                        output.push('\n');
                    }
                }
            }

            for (id, section) in &doc.sections {
                let section_name_string;
                let section_name = if let Some(name) = doc.section_names.get(id) {
                    name.as_str()
                } else {
                    section_name_string = id.to_string();
                    &section_name_string
                };
                output.push_str(&self.serialize_section_with_name(section_name, section));
                output.push('\n');
            }
        } else {
            for entry_ref in &doc.entry_order {
                match entry_ref {
                    crate::llm::types::EntryRef::Context(key) => {
                        if let Some(value) = doc.context.get(key) {
                            match value {
                                DxLlmValue::Obj(_) | DxLlmValue::Arr(_) => {
                                    let entry = self.serialize_context_entry(key, value);
                                    output.push_str(&entry);
                                    output.push('\n');
                                }
                                _ => {
                                    output.push_str(&format!(
                                        "{} = {}",
                                        key,
                                        self.serialize_single_value(value)
                                    ));
                                    output.push('\n');
                                }
                            }
                        }
                    }
                    crate::llm::types::EntryRef::Section(id) => {
                        if let Some(section) = doc.sections.get(id) {
                            let section_name_string;
                            let section_name = if let Some(name) = doc.section_names.get(id) {
                                name.as_str()
                            } else {
                                section_name_string = id.to_string();
                                &section_name_string
                            };
                            output
                                .push_str(&self.serialize_section_with_name(section_name, section));
                            output.push('\n');
                        }
                    }
                }
            }
        }

        output.trim_end().to_string()
    }

    const fn schema_separator(&self) -> &'static str {
        ", "
    }

    /// Check if any value in a slice has spaces (sentence-heavy detection)
    fn has_any_with_spaces(values: &[DxLlmValue]) -> bool {
        values.iter().any(|v| match v {
            DxLlmValue::Str(s) => s.contains(' '),
            _ => false,
        })
    }

    /// Join values with smart separator: comma if any has space, space otherwise
    fn smart_join(&self, values: &[DxLlmValue], quote_fn: impl Fn(&DxLlmValue) -> String) -> String {
        let use_commas = Self::has_any_with_spaces(values);
        let sep = if use_commas { ", " } else { " " };
        let items: Vec<String> = values.iter().map(|v| {
            let s = quote_fn(v);
            let is_str = matches!(v, DxLlmValue::Str(_));
            if use_commas && is_str && s.contains(',') {
                format!("\"{}\"", s.replace('"', "\\\""))
            } else if !use_commas && is_str && s.contains(' ') {
                format!("\"{}\"", s.replace('"', "\\\""))
            } else {
                s
            }
        }).collect();
        items.join(sep)
    }

    /// Serialize a context entry in Dx Serializer format
    fn serialize_context_entry(&self, key: &str, value: &DxLlmValue) -> String {
        match value {
            DxLlmValue::Arr(items) => {
                let serialized = self.smart_join(items, |v| self.serialize_value(v));
                // Single-element arrays need brackets to disambiguate from scalars
                if items.len() == 1 {
                    format!("{} = [{}]", key, serialized)
                } else {
                    format!("{} = {}", key, serialized)
                }
            }
            DxLlmValue::Obj(fields) => self.serialize_inline_object(key, fields),
            _ => {
                format!("{} = {}", key, self.serialize_value(value))
            }
        }
    }

    /// Serialize an object in multi-line format: name(\n  key = value\n  key2 = value2\n)
    fn serialize_inline_object(&self, key: &str, fields: &IndexMap<String, DxLlmValue>) -> String {
        let fields_str: Vec<String> = fields
            .iter()
            .map(|(k, v)| {
                if let DxLlmValue::Arr(items) = v {
                    let serialized = self.smart_join(items, |v| self.serialize_value(v));
                    if items.len() == 1 {
                        format!("  {} = [{}]", k, serialized)
                    } else {
                        format!("  {} = {}", k, serialized)
                    }
                } else {
                    format!("  {} = {}", k, self.serialize_value(v))
                }
            })
            .collect();
        format!("{}(\n{}\n)", key, fields_str.join("\n"))
    }

    /// Serialize a table section using wrapped dataframe format with indented rows
    /// Format: name[col1, col2, col3](\n  row1\n  row2\n)
    /// Uses smart separator (comma if any cell has space, space otherwise).
    fn serialize_section_with_name(&self, section_name: &str, section: &DxSection) -> String {
        let mut output = String::new();

        let schema_str = section.schema.join(self.schema_separator());
        output.push_str(&format!("{section_name}[{schema_str}]("));

        if !section.rows.is_empty() {
            output.push('\n');
            let use_commas = section.rows.iter().any(|row|
                row.iter().any(|v| matches!(v, DxLlmValue::Str(s) if s.contains(' ')))
            );
            for row in &section.rows {
                output.push_str("  ");
                let values: Vec<String> =
                    row.iter().map(|v| self.serialize_table_value(v, use_commas)).collect();
                let sep = if use_commas { ", " } else { " " };
                let row_str = values.join(sep);
                output.push_str(&row_str);
                output.push('\n');
            }
        }

        output.push(')');
        output
    }

    /// Serialize a table value. If `use_commas`, quote values containing commas.
    /// Otherwise, quote values containing spaces.
    fn serialize_table_value(&self, value: &DxLlmValue, use_commas: bool) -> String {
        match value {
            DxLlmValue::Bool(true) => "true".to_string(),
            DxLlmValue::Bool(false) => "false".to_string(),
            DxLlmValue::Null => "null".to_string(),
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

    /// Serialize a single value — raw, no quoting (caller handles quoting).
    fn serialize_value(&self, value: &DxLlmValue) -> String {
        match value {
            DxLlmValue::Bool(true) => "true".to_string(),
            DxLlmValue::Bool(false) => "false".to_string(),
            DxLlmValue::Null => "null".to_string(),
            DxLlmValue::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            DxLlmValue::Str(s) => s.clone(),
            DxLlmValue::Arr(items) => {
                let serialized: Vec<String> = items
                    .iter()
                    .map(|item| self.serialize_value(item))
                    .collect();
                serialized.join(", ")
            }
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

    /// Serialize a single value for root-level context, quoting if it contains commas.
    fn serialize_single_value(&self, value: &DxLlmValue) -> String {
        let raw = self.serialize_value(value);
        if raw.contains(',') {
            format!("\"{}\"", raw.replace('"', "\\\""))
        } else {
            raw
        }
    }
}

impl Default for LlmSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to serialize a document
#[must_use]
pub fn serialize(doc: &DxDocument) -> String {
    LlmSerializer::new().serialize(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_empty() {
        let serializer = LlmSerializer::new();
        let doc = DxDocument::new();
        let output = serializer.serialize(&doc);
        assert!(output.is_empty());
    }

    #[test]
    fn test_serialize_simple_values() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.context
            .insert("name".to_string(), DxLlmValue::Str("Test".to_string()));
        doc.context
            .insert("count".to_string(), DxLlmValue::Num(42.0));

        let output = serializer.serialize(&doc);
        assert!(output.contains("count = 42"), "Output was: {output}");
        assert!(output.contains("name = Test"), "Output was: {output}");
    }

    #[test]
    fn test_serialize_booleans() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.context
            .insert("active".to_string(), DxLlmValue::Bool(true));
        doc.context
            .insert("deleted".to_string(), DxLlmValue::Bool(false));

        let output = serializer.serialize(&doc);
        assert!(output.contains("active = true"), "Output was: {output}");
        assert!(output.contains("deleted = false"), "Output was: {output}");
    }

    #[test]
    fn test_serialize_array_simple() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.context.insert(
            "friends".to_string(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("ana".to_string()),
                DxLlmValue::Str("luis".to_string()),
                DxLlmValue::Str("sam".to_string()),
            ]),
        );

        let output = serializer.serialize(&doc);
        assert!(
            output.contains("friends = ana luis sam"),
            "Output was: {output}"
        );
    }

    #[test]
    fn test_serialize_array_with_spaces() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.context.insert(
            "friends".to_string(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("ana".to_string()),
                DxLlmValue::Str("bob smith".to_string()),
                DxLlmValue::Str("sam".to_string()),
            ]),
        );

        let output = serializer.serialize(&doc);
        // Uses commas because one value has a space
        assert!(
            output.contains("friends = ana, bob smith, sam"),
            "Output was: {output}"
        );
    }

    #[test]
    fn test_serialize_table() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();

        let mut section = DxSection::new(vec![
            "id".to_string(),
            "name".to_string(),
            "active".to_string(),
        ]);
        section.rows.push(vec![
            DxLlmValue::Num(1.0),
            DxLlmValue::Str("Alpha".to_string()),
            DxLlmValue::Bool(true),
        ]);
        section.rows.push(vec![
            DxLlmValue::Num(2.0),
            DxLlmValue::Str("Beta".to_string()),
            DxLlmValue::Bool(false),
        ]);
        doc.sections.insert('d', section);

        let output = serializer.serialize(&doc);
        assert!(
            output.contains("d[id, name, active]("),
            "Output was: {output}"
        );
        assert!(output.contains("  1 Alpha true"), "Output was: {output}");
        assert!(output.contains("  2 Beta false"), "Output was: {output}");
        assert!(output.contains(')'), "Output was: {output}");
    }

    #[test]
    fn test_serialize_table_with_spaces_in_text() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();

        let mut section = DxSection::new(vec![
            "id".to_string(),
            "name".to_string(),
            "dept".to_string(),
        ]);
        section.rows.push(vec![
            DxLlmValue::Num(1.0),
            DxLlmValue::Str("James Smith".to_string()),
            DxLlmValue::Str("Engineering".to_string()),
        ]);
        section.rows.push(vec![
            DxLlmValue::Num(2.0),
            DxLlmValue::Str("Mary Johnson".to_string()),
            DxLlmValue::Str("Research and Development".to_string()),
        ]);
        doc.sections.insert('e', section);

        let output = serializer.serialize(&doc);
        assert!(
            output.contains("  1, James Smith, Engineering"),
            "Output was: {output}"
        );
        assert!(
            output.contains("  2, Mary Johnson, Research and Development"),
            "Output was: {output}"
        );
    }

    #[test]
    fn test_serialize_null() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.context.insert("value".to_string(), DxLlmValue::Null);

        let output = serializer.serialize(&doc);
        assert!(output.contains("value = null"), "Output was: {output}");
    }

    #[test]
    fn test_serialize_single_string_with_spaces() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.context.insert(
            "task".to_string(),
            DxLlmValue::Str("Our favorite hikes together".to_string()),
        );

        let output = serializer.serialize(&doc);
        // Single strings with spaces don't need quotes (no ambiguity)
        assert!(
            output.contains("task = Our favorite hikes together"),
            "Output was: {output}"
        );
    }

    #[test]
    fn test_serialize_string_with_comma() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.context.insert(
            "desc".to_string(),
            DxLlmValue::Str("hello, world".to_string()),
        );

        let output = serializer.serialize(&doc);
        // Strings with commas need quotes to avoid being parsed as array
        assert!(
            output.contains("desc = \"hello, world\""),
            "Output was: {output}"
        );
    }

    #[test]
    fn test_serialize_inline_object() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();

        let mut fields = IndexMap::new();
        fields.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        fields.insert("port".to_string(), DxLlmValue::Num(8080.0));
        doc.context
            .insert("config".to_string(), DxLlmValue::Obj(fields));

        let output = serializer.serialize(&doc);
        // Multi-line object format
        assert!(output.contains("config("), "Output was: {output}");
        assert!(output.contains("  host = localhost"), "Output was: {output}");
        assert!(output.contains("  port = 8080"), "Output was: {output}");
        assert!(output.contains(')'), "Output was: {output}");
    }

    #[test]
    fn test_serialize_inline_object_with_nested_array() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();

        let mut fields = IndexMap::new();
        fields.insert("name".to_string(), DxLlmValue::Str("test".to_string()));
        fields.insert(
            "tags".to_string(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("rust".to_string()),
                DxLlmValue::Str("fast".to_string()),
            ]),
        );
        doc.context
            .insert("item".to_string(), DxLlmValue::Obj(fields));

        let output = serializer.serialize(&doc);
        assert!(output.contains("item("), "Output was: {output}");
        assert!(
            output.contains("  tags = rust fast"),
            "Output was: {output}"
        );
    }
}
