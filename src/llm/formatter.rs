//! LLM Format Formatter
//!
//! Produces consistently spaced LLM-format output (`--format` mode).
//! Adaptive separators, no brackets for arrays, indented sections.

use crate::llm::types::{DxDocument, DxLlmValue, DxSection, EntryRef};
use indexmap::IndexMap;

/// Produces consistently spaced LLM-format output.
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
            for (id, section) in &doc.sections {
                let section_name = doc
                    .section_names
                    .get(id)
                    .map_or("section", std::string::String::as_str);
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
                                .map_or("section", std::string::String::as_str);
                            self.format_section(&mut output, section_name, section);
                            output.push('\n');
                        }
                    }
                }
            }
        }

        output.trim_end().to_string()
    }

    fn has_any_with_spaces(values: &[DxLlmValue]) -> bool {
        values.iter().any(|v| match v {
            DxLlmValue::Str(s) => s.contains(' '),
            _ => false,
        })
    }

    fn smart_join(
        &self,
        values: &[DxLlmValue],
        quote_fn: impl Fn(&DxLlmValue) -> String,
    ) -> String {
        let use_commas = Self::has_any_with_spaces(values);
        let sep = if use_commas { ", " } else { " " };
        let items: Vec<String> = values
            .iter()
            .map(|v| {
                let s = quote_fn(v);
                let is_str = matches!(v, DxLlmValue::Str(_));
                if use_commas && is_str && s.contains(',') {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else if !use_commas && is_str && s.contains(' ') {
                    format!("\"{}\"", s.replace('"', "\\\""))
                } else {
                    s
                }
            })
            .collect();
        items.join(sep)
    }

    fn format_context_entry(&self, output: &mut String, key: &str, value: &DxLlmValue) {
        match value {
            DxLlmValue::Obj(fields) => {
                self.format_inline_object(output, key, fields);
            }
            DxLlmValue::Arr(items) => {
                output.push_str(key);
                output.push_str(" = ");
                let serialized = self.smart_join(items, |v| self.serialize_value(v));
                if items.len() == 1 {
                    output.push('[');
                    output.push_str(&serialized);
                    output.push(']');
                } else {
                    output.push_str(&serialized);
                }
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
        output.push_str("(\n");
        for (k, v) in fields {
            output.push_str("  ");
            if let DxLlmValue::Arr(items) = v {
                let serialized = self.smart_join(items, |v| self.serialize_value(v));
                if items.len() == 1 {
                    output.push_str(&format!("{k} = [{serialized}]"));
                } else {
                    output.push_str(&format!("{k} = {serialized}"));
                }
            } else {
                output.push_str(&format!("{} = {}", k, self.serialize_value(v)));
            }
            output.push('\n');
        }
        output.push(')');
    }

    fn format_section(&self, output: &mut String, name: &str, section: &DxSection) {
        output.push_str(name);
        output.push('[');
        output.push_str(&section.schema.join(" "));
        output.push_str("](\n");

        for row in &section.rows {
            let values: Vec<String> = row
                .iter()
                .map(|v| self.serialize_table_value(v, false))
                .collect();
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
            DxLlmValue::Int(i) => format!("{i}"),
            DxLlmValue::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            DxLlmValue::Str(s) => s.replace('\n', " "),
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
                let cleaned = s.replace('\n', " ");
                let needs_quoting = if use_commas {
                    cleaned.contains(',')
                } else {
                    cleaned.contains(' ')
                };
                if needs_quoting {
                    format!("\"{}\"", cleaned.replace('"', "\\\""))
                } else {
                    cleaned
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
        assert!(output.contains("tags = rust fast"), "Output: {output}");
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
        doc.entry_order
            .push(EntryRef::Context("config".to_string()));

        let output = formatter.format(&doc);
        assert!(output.contains("config("), "Output: {output}");
        assert!(output.contains("  host = localhost"), "Output: {output}");
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
        assert!(
            output.contains("users[id name active]("),
            "Output: {output}"
        );
        assert!(output.contains("1 Alpha true"), "Output: {output}");
    }

    #[test]
    fn test_format_table_with_quoted_strings() {
        use crate::llm::types::DxSection;
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
        assert!(output.contains("employees[id name]("), "Output: {output}");
        assert!(output.contains("1 \"James Smith\""), "Output: {output}");
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
        doc.entry_order
            .push(EntryRef::Context("version".to_string()));

        let output = formatter.format(&doc);
        assert!(
            output.contains("name = MyApp\n\nversion = 1.0"),
            "Output: {output}"
        );
    }
}
