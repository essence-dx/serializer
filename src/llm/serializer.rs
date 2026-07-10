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

use crate::llm::types::{DxDocument, DxLlmValue, DxSection, OptimizationLevel};
use indexmap::IndexMap;

/// Threshold: use parens for objects with this many children or more.
/// Below this threshold, use YAML-style (more token-efficient for small objects).
const PARENS_CHILD_THRESHOLD: usize = 8;

/// Configuration options for the serializer
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SerializerConfig {
    /// Compact mode: single-line sections (rows space-separated on one line)
    pub compact: bool,
    /// Optimization level: Low (compact), Medium (auto-select), High (human-readable)
    pub level: OptimizationLevel,
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

    /// Maximum number of children across all objects in the document.
    /// Used by Medium to decide YAML vs parens.
    fn max_object_children(&self, doc: &DxDocument) -> usize {
        let mut max = 0usize;
        for value in doc.context.values() {
            self.max_children_in_value(value, &mut max);
        }
        max
    }

    fn max_children_in_value(&self, value: &DxLlmValue, max: &mut usize) {
        match value {
            DxLlmValue::Obj(fields) => {
                *max = (*max).max(fields.len());
                for v in fields.values() {
                    self.max_children_in_value(v, max);
                }
            }
            DxLlmValue::Arr(items) => {
                for item in items {
                    self.max_children_in_value(item, max);
                }
            }
            _ => {}
        }
    }

    /// Determine whether to use YAML-style (vs parens) for objects.
    fn use_yaml_for_objects(&self, doc: &DxDocument) -> bool {
        match self.config.level {
            OptimizationLevel::Low => false, // compact parens
            OptimizationLevel::Medium => self.max_object_children(doc) < PARENS_CHILD_THRESHOLD,
            OptimizationLevel::High => true, // YAML always for High
        }
    }

    /// Serialize `DxDocument` to Dx Serializer format string
    #[must_use]
    pub fn serialize(&self, doc: &DxDocument) -> String {
        let mut output = String::new();
        let use_yaml = self.use_yaml_for_objects(doc);

        // If entry_order is populated, use it to maintain original order
        if doc.entry_order.is_empty() {
            // Fallback: serialize in default order (context then sections)
            for (key, value) in &doc.context {
                match value {
                    DxLlmValue::Obj(_) | DxLlmValue::Arr(_) => {
                        let entry = self.serialize_context_entry(key, value, use_yaml);
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
                                    let entry = self.serialize_context_entry(key, value, use_yaml);
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
        " "
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
    fn serialize_context_entry(&self, key: &str, value: &DxLlmValue, use_yaml: bool) -> String {
        match value {
            DxLlmValue::Arr(items) => {
                let serialized = self.smart_join(items, |v| self.serialize_value(v));
                if items.len() == 1 {
                    format!("{} = [{}]", key, serialized)
                } else {
                    format!("{} = {}", key, serialized)
                }
            }
            DxLlmValue::Obj(fields) => self.serialize_inline_object(key, fields, use_yaml, 0),
            _ => {
                format!("{} = {}", key, self.serialize_value(value))
            }
        }
    }

    /// Serialize an object. Supports three formats:
    ///
    /// - **YAML-style** (High, Medium/small): `key:\n  field: val\n  field2: val2\n`
    /// - **Parens-style** (Medium/large, Low compact): `key(\n  field = val\n  field2 = val2\n)`
    /// - **Compact** (Low): `key(field=val field2=val2)` (single-line)
    ///
    /// `depth` tracks nesting level for indentation.
    fn serialize_inline_object(
        &self,
        key: &str,
        fields: &IndexMap<String, DxLlmValue>,
        use_yaml: bool,
        depth: usize,
    ) -> String {
        let is_low = self.config.level == OptimizationLevel::Low;
        let indent = "  ".repeat(depth + 1);

        if is_low {
            // Compact single-line: key(field=val field2=val2)
            let items: Vec<String> = fields.iter().map(|(k, v)| {
                match v {
                    DxLlmValue::Obj(nested) => {
                        let nested_str = self.serialize_inline_object(k, nested, false, depth + 1);
                        // For compact, inline the nested as single-line too
                        // Remove first line's indent since we're in compact mode
                        nested_str
                    }
                    DxLlmValue::Arr(arr) => {
                        let vals: Vec<String> = arr.iter().map(|item| self.serialize_value(item)).collect();
                        format!("{}={}", k, vals.join(" "))
                    }
                    _ => format!("{}={}", k, self.serialize_value(v))
                }
            }).collect();
            format!("{}({})", key, items.join(" "))
        } else if use_yaml {
            // YAML-style: key:\n  field: val\n  field2: val2\n
            // Medium uses ': ' (token-efficient), High uses ' = ' (human-readable)
            let separator = if self.config.level == OptimizationLevel::High { " = " } else { ": " };
            let mut out = format!("{}:\n", key);
            for (k, v) in fields {
                match v {
                    DxLlmValue::Obj(nested) => {
                        let nested_str = self.serialize_inline_object(k, nested, true, depth + 1);
                        for line in nested_str.lines() {
                            out.push_str(&format!("{}{}\n", indent, line));
                        }
                    }
                    DxLlmValue::Arr(arr) => {
                        let serialized = self.smart_join(arr, |v| self.serialize_value(v));
                        if arr.len() == 1 {
                            out.push_str(&format!("{}{} = [{}]\n", indent, k, serialized));
                        } else {
                            out.push_str(&format!("{}{} = {}\n", indent, k, serialized));
                        }
                    }
                    _ => {
                        out.push_str(&format!("{}{}{}{}\n", indent, k, separator, self.serialize_value(v)));
                    }
                }
            }
            out.trim_end().to_string()
        } else {
            // Parens-style: key(\n  field = val\n  field2 = val2\n)
            let mut out = format!("{}(\n", key);
            for (k, v) in fields {
                match v {
                    DxLlmValue::Obj(nested) => {
                        let nested_str = self.serialize_inline_object(k, nested, false, depth + 1);
                        for line in nested_str.lines() {
                            out.push_str(&format!("{}{}\n", indent, line));
                        }
                    }
                    DxLlmValue::Arr(arr) => {
                        let serialized = self.smart_join(arr, |v| self.serialize_value(v));
                        if arr.len() == 1 {
                            out.push_str(&format!("{}  {} = [{}]\n", indent, k, serialized));
                        } else {
                            out.push_str(&format!("{}  {} = {}\n", indent, k, serialized));
                        }
                    }
                    _ => {
                        out.push_str(&format!("{}  {} = {}\n", indent, k, self.serialize_value(v)));
                    }
                }
            }
            out.push_str(&format!("{})", "  ".repeat(depth)));
            out
        }
    }

    /// Serialize a table section using wrapped dataframe format.
    /// Low: compact single-line: name[cols](row1 row2)
    /// Medium/High: unindented rows: name[cols](\nrow1\nrow2\n)
    fn serialize_section_with_name(&self, section_name: &str, section: &DxSection) -> String {
        let mut output = String::new();
        let schema_str = section.schema.join(self.schema_separator());

        if self.config.level == OptimizationLevel::Low {
            // Compact: name[cols](row1 val,row2 val)
            output.push_str(&format!("{section_name}[{schema_str}]("));
            let row_strs: Vec<String> = section.rows.iter().map(|row| {
                let values: Vec<String> =
                    row.iter().map(|v| self.serialize_table_value(v, false)).collect();
                values.join(" ")
            }).collect();
            output.push_str(&row_strs.join(","));
            output.push(')');
        } else {
            output.push_str(&format!("{section_name}[{schema_str}]("));
            if !section.rows.is_empty() {
                output.push('\n');
                for row in &section.rows {
                    let values: Vec<String> =
                        row.iter().map(|v| self.serialize_table_value(v, false)).collect();
                    output.push_str(&values.join(" "));
                    output.push('\n');
                }
            }
            output.push(')');
        }
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

    fn default_serializer() -> LlmSerializer {
        LlmSerializer::new()
    }

    fn lp() -> LlmSerializer {
        // Low (compact) mode
        LlmSerializer::with_config(SerializerConfig { level: OptimizationLevel::Low, compact: false })
    }

    fn mp() -> LlmSerializer {
        // Medium (auto-select) mode - default
        LlmSerializer::new()
    }

    fn hp() -> LlmSerializer {
        // High (human-readable) mode
        LlmSerializer::with_config(SerializerConfig { level: OptimizationLevel::High, compact: false })
    }

    fn mk_doc() -> DxDocument {
        DxDocument::new()
    }

    #[test]
    fn test_serialize_empty() {
        let doc = mk_doc();
        assert!(default_serializer().serialize(&doc).is_empty());
    }

    #[test]
    fn test_serialize_simple_values() {
        let mut doc = mk_doc();
        doc.context.insert("name".to_string(), DxLlmValue::Str("Test".to_string()));
        doc.context.insert("count".to_string(), DxLlmValue::Num(42.0));
        let output = default_serializer().serialize(&doc);
        assert!(output.contains("count = 42"), "Output was: {output}");
        assert!(output.contains("name = Test"), "Output was: {output}");
    }

    #[test]
    fn test_serialize_booleans() {
        let mut doc = mk_doc();
        doc.context.insert("active".to_string(), DxLlmValue::Bool(true));
        doc.context.insert("deleted".to_string(), DxLlmValue::Bool(false));
        let output = default_serializer().serialize(&doc);
        assert!(output.contains("active = true"), "Output was: {output}");
        assert!(output.contains("deleted = false"), "Output was: {output}");
    }

    #[test]
    fn test_serialize_array_simple() {
        let mut doc = mk_doc();
        doc.context.insert("friends".to_string(), DxLlmValue::Arr(vec![
            DxLlmValue::Str("ana".to_string()),
            DxLlmValue::Str("luis".to_string()),
            DxLlmValue::Str("sam".to_string()),
        ]));
        let output = default_serializer().serialize(&doc);
        assert!(output.contains("friends = ana luis sam"), "Output was: {output}");
    }

    #[test]
    fn test_serialize_array_with_spaces() {
        let mut doc = mk_doc();
        doc.context.insert("friends".to_string(), DxLlmValue::Arr(vec![
            DxLlmValue::Str("ana".to_string()),
            DxLlmValue::Str("bob smith".to_string()),
            DxLlmValue::Str("sam".to_string()),
        ]));
        let output = default_serializer().serialize(&doc);
        assert!(output.contains("friends = ana, bob smith, sam"), "Output was: {output}");
    }

    // --- Table tests ---

    fn sample_section() -> DxSection {
        let mut s = DxSection::new(vec!["id".to_string(), "name".to_string(), "active".to_string()]);
        s.rows.push(vec![DxLlmValue::Num(1.0), DxLlmValue::Str("Alpha".to_string()), DxLlmValue::Bool(true)]);
        s.rows.push(vec![DxLlmValue::Num(2.0), DxLlmValue::Str("Beta".to_string()), DxLlmValue::Bool(false)]);
        s
    }

    fn sample_section_text() -> DxSection {
        let mut s = DxSection::new(vec!["id".to_string(), "name".to_string(), "dept".to_string()]);
        s.rows.push(vec![DxLlmValue::Num(1.0), DxLlmValue::Str("James Smith".to_string()), DxLlmValue::Str("Engineering".to_string())]);
        s.rows.push(vec![DxLlmValue::Num(2.0), DxLlmValue::Str("Mary Johnson".to_string()), DxLlmValue::Str("Research and Development".to_string())]);
        s
    }

    #[test]
    fn test_serialize_table_low() {
        let mut doc = mk_doc();
        doc.sections.insert('d', sample_section());
        let output = lp().serialize(&doc);
        // Low: compact single-line rows joined by comma
        assert!(output.contains("d[id name active]("), "Output: {output}");
        assert!(output.contains("1 Alpha true,2 Beta false"), "Output: {output}");
    }

    #[test]
    fn test_serialize_table_medium() {
        let mut doc = mk_doc();
        doc.sections.insert('d', sample_section());
        let output = mp().serialize(&doc);
        // Medium: unindented rows, space-separated values
        assert!(output.contains("d[id name active]("), "Output: {output}");
        assert!(output.contains("1 Alpha true"), "Output: {output}");
        assert!(output.contains("2 Beta false"), "Output: {output}");
    }

    #[test]
    fn test_serialize_table_with_text() {
        let mut doc = mk_doc();
        doc.sections.insert('e', sample_section_text());
        let output = mp().serialize(&doc);
        assert!(output.contains("1 \"James Smith\" Engineering"), "Output: {output}");
        assert!(output.contains("2 \"Mary Johnson\" \"Research and Development\""), "Output: {output}");
    }

    #[test]
    fn test_serialize_null() {
        let mut doc = mk_doc();
        doc.context.insert("value".to_string(), DxLlmValue::Null);
        let output = default_serializer().serialize(&doc);
        assert!(output.contains("value = null"), "Output was: {output}");
    }

    #[test]
    fn test_serialize_single_string_with_spaces() {
        let mut doc = mk_doc();
        doc.context.insert("task".to_string(), DxLlmValue::Str("Our favorite hikes together".to_string()));
        let output = default_serializer().serialize(&doc);
        assert!(output.contains("task = Our favorite hikes together"), "Output: {output}");
    }

    #[test]
    fn test_serialize_string_with_comma() {
        let mut doc = mk_doc();
        doc.context.insert("desc".to_string(), DxLlmValue::Str("hello, world".to_string()));
        let output = default_serializer().serialize(&doc);
        assert!(output.contains("desc = \"hello, world\""), "Output: {output}");
    }

    // --- Object format tests ---

    fn sample_object() -> IndexMap<String, DxLlmValue> {
        let mut f = IndexMap::new();
        f.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        f.insert("port".to_string(), DxLlmValue::Num(8080.0));
        f
    }

    #[test]
    fn test_object_low_compact() {
        let mut doc = mk_doc();
        doc.context.insert("config".to_string(), DxLlmValue::Obj(sample_object()));
        let output = lp().serialize(&doc);
        // Low: single-line compact: config(host=localhost port=8080)
        assert!(output.contains("config("), "Output: {output}");
        assert!(!output.contains('\n'), "Low should not have newlines in objects: {output}");
    }

    #[test]
    fn test_object_medium_yaml() {
        let mut doc = mk_doc();
        doc.context.insert("config".to_string(), DxLlmValue::Obj(sample_object()));
        let output = mp().serialize(&doc);
        // Medium with 2 fields (< threshold): YAML-style with ':'
        assert!(output.contains("config:"), "Output: {output}");
        assert!(output.contains("host: localhost"), "Output: {output}");
        assert!(output.contains("port: 8080"), "Output: {output}");
    }

    #[test]
    fn test_object_medium_parens_large() {
        let mut doc = mk_doc();
        // Create an object with PARENS_CHILD_THRESHOLD fields to trigger parens
        let mut big = IndexMap::new();
        for i in 0..PARENS_CHILD_THRESHOLD {
            big.insert(format!("k{i}"), DxLlmValue::Num(i as f64));
        }
        doc.context.insert("big".to_string(), DxLlmValue::Obj(big));
        let output = mp().serialize(&doc);
        // Medium with 8+ fields: parens
        assert!(output.contains("big("), "Output: {output}");
    }

    #[test]
    fn test_object_high_yaml() {
        let mut doc = mk_doc();
        doc.context.insert("config".to_string(), DxLlmValue::Obj(sample_object()));
        let output = hp().serialize(&doc);
        // High: YAML-style with ' = '
        assert!(output.contains("config:"), "Output: {output}");
        assert!(output.contains("host = localhost"), "Output: {output}");
        assert!(output.contains("port = 8080"), "Output: {output}");
    }

    #[test]
    fn test_nested_object_medium_yaml() {
        let mut doc = mk_doc();
        let mut features = IndexMap::new();
        features.insert("debug".to_string(), DxLlmValue::Bool(true));
        features.insert("cache".to_string(), DxLlmValue::Bool(false));
        let mut fields = IndexMap::new();
        fields.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        fields.insert("features".to_string(), DxLlmValue::Obj(features));
        doc.context.insert("config".to_string(), DxLlmValue::Obj(fields));
        let output = mp().serialize(&doc);
        assert!(output.contains("config:"), "Output: {output}");
        assert!(output.contains("host: localhost"), "Output: {output}");
        assert!(output.contains("features:"), "Output: {output}");
        assert!(output.contains("debug: true"), "Output: {output}");
    }

    #[test]
    fn test_nested_object_medium_parens() {
        let mut doc = mk_doc();
        // Make the outer object large enough to trigger parens
        let mut features = IndexMap::new();
        features.insert("debug".to_string(), DxLlmValue::Bool(true));
        features.insert("cache".to_string(), DxLlmValue::Bool(false));
        let mut fields = IndexMap::new();
        for i in 0..PARENS_CHILD_THRESHOLD {
            fields.insert(format!("k{i}"), DxLlmValue::Num(i as f64));
        }
        fields.insert("features".to_string(), DxLlmValue::Obj(features));
        doc.context.insert("big".to_string(), DxLlmValue::Obj(fields));
        let output = mp().serialize(&doc);
        assert!(output.contains("big("), "Output: {output}");
        assert!(output.contains("  features("), "Output: {output}");
    }

    #[test]
    fn test_object_with_nested_array() {
        let mut fields = IndexMap::new();
        fields.insert("name".to_string(), DxLlmValue::Str("test".to_string()));
        fields.insert("tags".to_string(), DxLlmValue::Arr(vec![
            DxLlmValue::Str("rust".to_string()),
            DxLlmValue::Str("fast".to_string()),
        ]));
        let mut doc = mk_doc();
        doc.context.insert("item".to_string(), DxLlmValue::Obj(fields));

        // Test Medium (YAML)
        let output = mp().serialize(&doc);
        assert!(output.contains("item:"), "Output: {output}");
        assert!(output.contains("tags = rust fast"), "Output: {output}");

        // Test High
        let output = hp().serialize(&doc);
        assert!(output.contains("item:"), "Output: {output}");
        assert!(output.contains("tags = rust fast"), "Output: {output}");

        // Test Low
        let output = lp().serialize(&doc);
        assert!(output.contains("item("), "Output: {output}");
    }

    #[test]
    fn test_low_compact_all_in_one_line() {
        let mut doc = mk_doc();
        doc.context.insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
        let mut fields = IndexMap::new();
        fields.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        fields.insert("port".to_string(), DxLlmValue::Num(8080.0));
        doc.context.insert("config".to_string(), DxLlmValue::Obj(fields));

        let mut s = DxSection::new(vec!["id".to_string(), "val".to_string()]);
        s.rows.push(vec![DxLlmValue::Num(1.0), DxLlmValue::Str("x".to_string())]);
        doc.sections.insert('d', s);

        let output = lp().serialize(&doc);
        // Low: everything compact, minimal newlines
        assert!(output.contains("config(host=localhost port=8080)"),
            "Low mode should be compact single-line. Output: {output}");
        assert!(output.contains("d[id val](1 x)"),
            "Low mode tables should be single-line. Output: {output}");
    }

    #[test]
    fn test_high_yaml_with_equals() {
        let mut doc = mk_doc();
        let mut fields = IndexMap::new();
        fields.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        fields.insert("port".to_string(), DxLlmValue::Num(8080.0));
        doc.context.insert("config".to_string(), DxLlmValue::Obj(fields));

        let output = hp().serialize(&doc);
        // High: YAML-style with ' = ' (human-readable)
        assert!(output.contains("config:\n  host = localhost\n  port = 8080"),
            "High mode should use YAML-style with ' = '. Output: {output}");
    }
}
