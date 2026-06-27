//! LLM Format Formatter
//!
//! Produces consistently spaced/indented LLM-format output (`--format` mode).
//! Still LLM format (not human format), just with improved readability:
//! - Spaces around `=` for top-level key-value pairs
//! - Blank lines between entries
//! - Indented section rows (4 spaces)
//! - Space-separated structural tokens in table headers

use crate::llm::types::{EntryRef, DxDocument, DxLlmValue, DxSection};
use indexmap::IndexMap;

/// Produces consistently spaced/indented LLM-format output.
pub struct LlmFormatter;

impl LlmFormatter {
    /// Format a document in formatted LLM format
    #[must_use]
    pub fn format(&self, doc: &DxDocument) -> String {
        let mut output = String::new();

        if doc.entry_order.is_empty() {
            for (key, value) in &doc.context {
                self.format_context_entry(&mut output, key, value);
                output.push_str("\n\n");
            }
            for (_id, section) in &doc.sections {
                let section_name = doc
                    .section_names
                    .iter()
                    .next()
                    .map(|(_, n)| n.as_str())
                    .unwrap_or("section");
                self.format_section(&mut output, section_name, section);
                output.push('\n');
            }
        } else {
            for entry_ref in &doc.entry_order {
                match entry_ref {
                    EntryRef::Context(key) => {
                        if let Some(value) = doc.context.get(key) {
                            self.format_context_entry(&mut output, key, value);
                            output.push_str("\n\n");
                        }
                    }
                    EntryRef::Section(id) => {
                        if let Some(section) = doc.sections.get(id) {
                            let section_name = doc
                                .section_names
                                .get(id)
                                .map(|s| s.as_str())
                                .unwrap_or("section");
                            self.format_section(&mut output, section_name, section);
                            output.push('\n');
                        }
                    }
                }
            }
        }

        output.trim_end().to_string()
    }

    fn format_context_entry(&self, output: &mut String, key: &str, value: &DxLlmValue) {
        match value {
            DxLlmValue::Obj(fields) => {
                self.format_inline_object(output, key, fields);
            }
            DxLlmValue::Arr(items) => {
                output.push_str(key);
                output.push_str(" = [");
                let items_str: Vec<String> =
                    items.iter().map(|v| self.serialize_value(v)).collect();
                output.push_str(&items_str.join(" "));
                output.push(']');
            }
            _ => {
                output.push_str(key);
                output.push_str(" = ");
                output.push_str(&self.serialize_value(value));
            }
        }
    }

    fn format_inline_object(
        &self,
        output: &mut String,
        key: &str,
        fields: &IndexMap<String, DxLlmValue>,
    ) {
        output.push_str(key);
        output.push('(');
        let parts: Vec<String> = fields
            .iter()
            .map(|(k, v)| {
                if let DxLlmValue::Arr(items) = v {
                    let items_str: Vec<String> =
                        items.iter().map(|i| self.serialize_value(i)).collect();
                    format!("{} = [{}]", k, items_str.join(" "))
                } else {
                    format!("{}={}", k, self.serialize_value(v))
                }
            })
            .collect();
        output.push_str(&parts.join(" "));
        output.push(')');
    }

    fn format_section(&self, output: &mut String, name: &str, section: &DxSection) {
        output.push_str(name);
        output.push_str(" [");
        output.push_str(&section.schema.join(" "));
        output.push_str("] (\n");

        for row in &section.rows {
            output.push_str("    ");
            let values: Vec<String> =
                row.iter().map(|v| self.serialize_table_value(v)).collect();
            output.push_str(&values.join(" "));
            output.push('\n');
        }

        output.push(')');
    }

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
                if s.contains(' ') {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else {
                    s.clone()
                }
            }
            DxLlmValue::Arr(items) => {
                let serialized: Vec<String> =
                    items.iter().map(|item| self.serialize_value(item)).collect();
                serialized.join(",")
            }
            DxLlmValue::Obj(fields) => {
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, self.serialize_value(v)))
                    .collect();
                format!("[{}]", fields_str.join(","))
            }
            DxLlmValue::Ref(key) => format!("^{key}"),
        }
    }

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
                let fields_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, self.serialize_table_value(v)))
                    .collect();
                format!("({})", fields_str.join(","))
            }
            DxLlmValue::Ref(key) => format!("^{key}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::{DxLlmValue, DxSection};
    use indexmap::IndexMap;

    #[test]
    fn test_format_empty_document() {
        let formatter = LlmFormatter;
        let doc = DxDocument::new();
        let output = formatter.format(&doc);
        assert!(output.is_empty());
    }

    #[test]
    fn test_format_context_key_value() {
        let formatter = LlmFormatter;
        let mut doc = DxDocument::new();
        doc.context
            .insert("name".to_string(), DxLlmValue::Str("Test".to_string()));
        doc.context
            .insert("count".to_string(), DxLlmValue::Num(42.0));
        doc.entry_order.push(EntryRef::Context("name".to_string()));
        doc.entry_order.push(EntryRef::Context("count".to_string()));

        let output = formatter.format(&doc);
        assert!(output.contains("name = Test"), "Output: {output}");
        assert!(output.contains("count = 42"), "Output: {output}");
    }

    #[test]
    fn test_format_array() {
        let formatter = LlmFormatter;
        let mut doc = DxDocument::new();
        doc.context.insert(
            "tags".to_string(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("rust".to_string()),
                DxLlmValue::Str("fast".to_string()),
            ]),
        );
        doc.entry_order.push(EntryRef::Context("tags".to_string()));

        let output = formatter.format(&doc);
        assert!(
            output.contains("tags = [rust fast]"),
            "Output: {output}"
        );
    }

    #[test]
    fn test_format_inline_object() {
        let formatter = LlmFormatter;
        let mut doc = DxDocument::new();
        let mut fields = IndexMap::new();
        fields.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        fields.insert("port".to_string(), DxLlmValue::Num(8080.0));
        doc.context
            .insert("config".to_string(), DxLlmValue::Obj(fields));
        doc.entry_order.push(EntryRef::Context("config".to_string()));

        let output = formatter.format(&doc);
        // Top-level object uses key(...) with no spaces around = inside
        assert!(output.contains("config("), "Output: {output}");
        assert!(output.contains("host=localhost"), "Output: {output}");
        assert!(output.contains("port=8080"), "Output: {output}");
    }

    #[test]
    fn test_format_table_section() {
        let formatter = LlmFormatter;
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
        doc.sections.insert('d', section);
        doc.section_names.insert('d', "users".to_string());
        doc.entry_order.push(EntryRef::Section('d'));

        let output = formatter.format(&doc);
        assert!(output.contains("users [id name active] ("), "Output: {output}");
        assert!(output.contains("    1 Alpha true"), "Output: {output}");
    }

    #[test]
    fn test_format_table_with_quoted_strings() {
        let formatter = LlmFormatter;
        let mut doc = DxDocument::new();
        let mut section = DxSection::new(vec!["id".to_string(), "name".to_string()]);
        section.rows.push(vec![
            DxLlmValue::Num(1.0),
            DxLlmValue::Str("James Smith".to_string()),
        ]);
        doc.sections.insert('e', section);
        doc.section_names.insert('e', "employees".to_string());
        doc.entry_order.push(EntryRef::Section('e'));

        let output = formatter.format(&doc);
        assert!(
            output.contains("employees [id name] ("),
            "Output: {output}"
        );
        assert!(
            output.contains("1 \"James Smith\""),
            "Output: {output}"
        );
    }

    #[test]
    fn test_format_blank_lines_between_entries() {
        let formatter = LlmFormatter;
        let mut doc = DxDocument::new();
        doc.context
            .insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
        doc.context
            .insert("version".to_string(), DxLlmValue::Str("1.0".to_string()));
        doc.entry_order.push(EntryRef::Context("name".to_string()));
        doc.entry_order.push(EntryRef::Context("version".to_string()));

        let output = formatter.format(&doc);
        // There should be a blank line between entries
        assert!(output.contains("name = MyApp\n\nversion = 1.0"), "Output: {output}");
    }
}
