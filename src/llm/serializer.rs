//! DX Serializer LLM Format
//!
//! Serializes `DxDocument` to the token-optimized LLM format.
//! 52-73% more token-efficient than JSON.
//!
//! ## LLM Format Syntax (Wrapped Dataframe)
//!
//! ```text
//! # Key-Value Pairs
//! name=MyApp
//! port=8080
//! description="Multi word string"
//!
//! # Arrays (square brackets)
//! tags=[rust performance serialization]
//! editors=[neovim zed "firebase studio"]
//!
//! # Objects (parentheses)
//! config(host=localhost port=5432 debug=true)
//! server(url="https://api.example.com" timeout=30)
//!
//! # Tables (wrapped dataframes - deterministic parsing)
//! users[id name email](
//! 1 Alice alice@ex.com
//! 2 Bob bob@ex.com
//! 3 Carol carol@ex.com
//! )
//!
//! # Multi-word values use quotes
//! employees[id name dept](
//! 1 "James Smith" Engineering
//! 2 "Mary Johnson" "Research and Development"
//! )
//! ```
//!
//! ## Why DX Beats TOON
//!
//! 1. Deterministic parsing - Wrapped dataframes `[headers](rows)` eliminate ambiguity
//! 2. No indentation - TOON requires 2 spaces per row
//! 3. Quoted strings - Standard, predictable, robust (not underscores)
//! 4. Mental model alignment - `[]` arrays, `()` objects, `[headers](rows)` tables

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
                        // Keep dots in key names (don't convert to underscores)
                        let entry = self.serialize_context_entry(key, value);
                        output.push_str(&entry);
                        output.push('\n');
                    }
                    _ => {
                        output.push_str(&format!("{}={}", key, self.serialize_value(value)));
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
                                        "{}={}",
                                        key,
                                        self.serialize_value(value)
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

    const fn inline_value_separator(&self) -> &'static str {
        " "
    }

    const fn schema_separator(&self) -> &'static str {
        " "
    }

    /// Serialize a context entry in Dx Serializer format
    fn serialize_context_entry(&self, key: &str, value: &DxLlmValue) -> String {
        match value {
            DxLlmValue::Arr(items) => {
                let items_str: Vec<String> =
                    items.iter().map(|v| self.serialize_value(v)).collect();
                format!(
                    "{}=[{}]",
                    key,
                    items_str.join(self.inline_value_separator())
                )
            }
            DxLlmValue::Obj(fields) => self.serialize_inline_object(key, fields),
            _ => {
                format!("{}={}", key, self.serialize_value(value))
            }
        }
    }

    /// Serialize an object in inline format: name(key=value key2=value2)
    fn serialize_inline_object(&self, key: &str, fields: &IndexMap<String, DxLlmValue>) -> String {
        let fields_str: Vec<String> = fields
            .iter()
            .map(|(k, v)| {
                if let DxLlmValue::Arr(items) = v {
                    let items_str: Vec<String> = items
                        .iter()
                        .map(|item| self.serialize_value(item))
                        .collect();
                    format!("{}=[{}]", k, items_str.join(self.inline_value_separator()))
                } else {
                    format!("{}={}", k, self.serialize_value(v))
                }
            })
            .collect();
        format!(
            "{}({})",
            key,
            fields_str.join(self.inline_value_separator())
        )
    }

    /// Serialize a table section with string name using wrapped dataframe format
    /// Format: name[col1 col2 col3](rows)
    /// When compact mode is enabled, rows are inlined on a single line.
    fn serialize_section_with_name(&self, section_name: &str, section: &DxSection) -> String {
        let mut output = String::new();

        let schema_str = section.schema.join(self.schema_separator());
        output.push_str(&format!("{section_name}[{schema_str}]("));

        if !section.rows.is_empty() {
            if self.config.compact {
                let row_parts: Vec<String> = section
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|v| self.serialize_table_value(v))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect();
                output.push_str(&row_parts.join(" "));
            } else {
                output.push('\n');
                for row in &section.rows {
                    let values: Vec<String> =
                        row.iter().map(|v| self.serialize_table_value(v)).collect();
                    output.push_str(&values.join(" "));
                    output.push('\n');
                }
            }
        }

        output.push(')');
        output
    }

    /// Serialize a table value for table rows with quotes for multi-word strings
    fn serialize_table_value(&self, value: &DxLlmValue) -> String {
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
                // Use quotes for strings with spaces
                if s.contains(' ') {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else {
                    s.clone()
                }
            }
            DxLlmValue::Arr(items) => {
                let serialized: Vec<String> = items
                    .iter()
                    .map(|item| self.serialize_table_value(item))
                    .collect();
                serialized.join(",")
            }
            DxLlmValue::Obj(fields) => {
                // Inline object in table cell: (key=value,key2=value2)
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, self.serialize_table_value(v)))
                    .collect();
                format!("({})", fields_str.join(","))
            }
            DxLlmValue::Ref(key) => format!("^{key}"),
        }
    }

    /// Serialize a single value with quotes for multi-word strings
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
            DxLlmValue::Str(s) => {
                // Use quotes for strings with spaces
                if s.contains(' ') {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else {
                    s.clone()
                }
            }
            DxLlmValue::Arr(items) => {
                let serialized: Vec<String> = items
                    .iter()
                    .map(|item| self.serialize_value(item))
                    .collect();
                serialized.join(",")
            }
            DxLlmValue::Obj(fields) => {
                // Nested object: [key=value,key2=value2]
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, self.serialize_value(v)))
                    .collect();
                format!("[{}]", fields_str.join(","))
            }
            DxLlmValue::Ref(key) => format!("^{key}"),
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
        assert!(output.contains("count=42"), "Output was: {output}");
        assert!(output.contains("name=Test"), "Output was: {output}");
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
        assert!(output.contains("active=true"), "Output was: {output}");
        assert!(output.contains("deleted=false"), "Output was: {output}");
    }

    #[test]
    fn test_serialize_array() {
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
            output.contains("friends=[ana luis sam]"),
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
        // Wrapped dataframe format
        assert!(
            output.contains("d[id name active]("),
            "Output was: {output}"
        );
        assert!(output.contains("1 Alpha true"), "Output was: {output}");
        assert!(output.contains("2 Beta false"), "Output was: {output}");
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
        // Strings with spaces use quotes
        assert!(
            output.contains("1 \"James Smith\" Engineering"),
            "Output was: {output}"
        );
        assert!(
            output.contains("2 \"Mary Johnson\" \"Research and Development\""),
            "Output was: {output}"
        );
    }

    #[test]
    fn test_serialize_null() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.context.insert("value".to_string(), DxLlmValue::Null);

        let output = serializer.serialize(&doc);
        assert!(output.contains("value=null"), "Output was: {output}");
    }

    #[test]
    fn test_serialize_quoted_string() {
        let serializer = LlmSerializer::new();
        let mut doc = DxDocument::new();
        doc.context.insert(
            "task".to_string(),
            DxLlmValue::Str("Our favorite hikes together".to_string()),
        );

        let output = serializer.serialize(&doc);
        // Strings with spaces use quotes
        assert!(
            output.contains("task=\"Our favorite hikes together\""),
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
        // Strings with spaces use quotes
        assert!(
            output.contains("desc=\"hello, world\""),
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
        // Parentheses for objects
        assert!(output.contains("config("), "Output was: {output}");
        assert!(output.contains("host=localhost"), "Output was: {output}");
        assert!(output.contains("port=8080"), "Output was: {output}");
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
        // Parentheses for objects, square brackets for arrays
        assert!(output.contains("item("), "Output was: {output}");
        assert!(
            output.contains("tags=[rust fast]"),
            "Output was: {output}"
        );
    }
}
