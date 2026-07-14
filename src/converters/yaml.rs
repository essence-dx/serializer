/// YAML to DX ULTRA converter
use crate::converters::json::{json_to_document, json_to_dx};
use crate::llm::types::DxDocument;
use crate::types::DxValue;

/// Convert YAML string to DX ULTRA format
///
/// Strategy: Convert YAML → JSON → DX
/// This leverages existing JSON converter with optimization
pub fn yaml_to_dx(yaml_str: &str) -> Result<String, String> {
    // Parse YAML
    let value: serde_json::Value =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML parse error: {e}"))?;

    // Convert to JSON string
    let json_str =
        serde_json::to_string(&value).map_err(|e| format!("JSON conversion error: {e}"))?;

    // Use JSON converter
    json_to_dx(&json_str)
}

/// Convert YAML string into the shared DX document model.
///
/// Strategy: YAML → JSON → DxDocument
pub fn yaml_to_document(yaml_str: &str) -> Result<DxDocument, String> {
    let value: serde_json::Value =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML parse error: {e}"))?;
    let json_str =
        serde_json::to_string(&value).map_err(|e| format!("JSON conversion error: {e}"))?;
    json_to_document(&json_str)
}

/// Convert DX format string to YAML using the DxDocument model.
///
/// Tries the LLM parser first; falls back to the old `DxValue` parser.
pub fn dx_to_yaml_doc(dx_str: &str) -> Result<String, String> {
    match crate::llm::llm_to_document(dx_str) {
        Ok(doc) => {
            let mut output = String::new();
            dx_document_to_yaml_impl(&doc, &mut output, 0)?;
            Ok(output)
        }
        Err(_) => {
            let value = crate::parser::parse(dx_str.as_bytes())
                .map_err(|e| format!("DX parse error: {e}"))?;
            let mut output = String::new();
            dx_value_to_yaml_impl(&value, &mut output, 0)?;
            Ok(output)
        }
    }
}

/// Convert a DxDocument to YAML format.
fn dx_document_to_yaml_impl(
    doc: &crate::llm::types::DxDocument,
    output: &mut String,
    indent: usize,
) -> Result<(), String> {
    let indent_str = "  ".repeat(indent);
    for (i, (k, v)) in doc.context.iter().enumerate() {
        if i > 0 {
            output.push('\n');
            output.push_str(&indent_str);
        }
        output.push_str(k);
        output.push_str(": ");
        dx_llm_value_to_yaml(v, output, indent + 1)?;
    }
    for (id, section) in &doc.sections {
        let name = doc.section_names.get(id).cloned().unwrap_or_else(|| id.to_string());
        if !doc.context.is_empty() {
            output.push('\n');
        }
        for row in &section.rows {
            output.push_str(&indent_str);
            output.push_str(&name);
            output.push_str(":\n");
            for (j, col) in section.schema.iter().enumerate() {
                output.push_str(&"  ".repeat(indent + 1));
                output.push_str(col);
                output.push_str(": ");
                if let Some(val) = row.get(j) {
                    dx_llm_value_to_yaml(val, output, indent + 2)?;
                }
                output.push('\n');
            }
        }
    }
    Ok(())
}

/// Convert a DxLlmValue to YAML inline.
fn dx_llm_value_to_yaml(
    value: &crate::llm::types::DxLlmValue,
    output: &mut String,
    indent: usize,
) -> Result<(), String> {
    use crate::llm::types::DxLlmValue;
    match value {
        DxLlmValue::Null => output.push_str("null"),
        DxLlmValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        DxLlmValue::Num(n) => output.push_str(&n.to_string()),
        DxLlmValue::Str(s) => {
            if s.contains(':') || s.contains('#') || s.contains('\n')
                || s.starts_with(' ') || s.ends_with(' ')
            {
                output.push('"');
                output.push_str(&s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"));
                output.push('"');
            } else {
                output.push_str(s);
            }
        }
        DxLlmValue::Arr(items) => {
            if items.is_empty() {
                output.push_str("[]");
            } else {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        output.push('\n');
                        output.push_str(&"  ".repeat(indent));
                    }
                    output.push_str("- ");
                    dx_llm_value_to_yaml(item, output, indent + 1)?;
                }
            }
        }
        DxLlmValue::Obj(fields) => {
            for (i, (k, v)) in fields.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                    output.push_str(&"  ".repeat(indent));
                }
                output.push_str(k);
                output.push_str(": ");
                dx_llm_value_to_yaml(v, output, indent + 1)?;
            }
        }
        DxLlmValue::Ref(r) => {
            output.push('"'); output.push('^'); output.push_str(r); output.push('"');
        }
    }
    Ok(())
}

/// Convert DX format string to YAML (auto-detect parser).
pub fn dx_to_yaml(dx_str: &str) -> Result<String, String> {
    dx_to_yaml_doc(dx_str)
}

fn dx_value_to_yaml_impl(value: &DxValue, output: &mut String, indent: usize) -> Result<(), String> {
    let indent_str = "  ".repeat(indent);

    match value {
        DxValue::Null => output.push_str("null"),
        DxValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        DxValue::Int(i) => output.push_str(&i.to_string()),
        DxValue::Float(f) => output.push_str(&f.to_string()),
        DxValue::String(s) => {
            if s.contains(':') || s.contains('#') || s.contains('\n')
                || s.starts_with(' ') || s.ends_with(' ')
            {
                output.push('"');
                output.push_str(&s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"));
                output.push('"');
            } else {
                output.push_str(s);
            }
        }
        DxValue::Array(arr) => {
            if arr.values.is_empty() {
                output.push_str("[]");
            } else {
                for (i, item) in arr.values.iter().enumerate() {
                    if i > 0 {
                        output.push('\n');
                        output.push_str(&indent_str);
                    }
                    output.push_str("- ");
                    dx_value_to_yaml_impl(item, output, indent + 1)?;
                }
            }
        }
        DxValue::Object(obj) => {
            for (i, (k, v)) in obj.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                    output.push_str(&indent_str);
                }
                output.push_str(k);
                output.push_str(": ");
                if matches!(v, DxValue::Object(_) | DxValue::Array(_)) {
                    output.push('\n');
                    output.push_str(&"  ".repeat(indent + 1));
                }
                dx_value_to_yaml_impl(v, output, indent + 1)?;
            }
        }
        DxValue::Table(table) => {
            for (i, row) in table.rows.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                    output.push_str(&indent_str);
                }
                output.push_str("- ");
                for (j, col) in table.schema.columns.iter().enumerate() {
                    if j > 0 {
                        output.push('\n');
                        output.push_str(&"  ".repeat(indent + 1));
                    }
                    output.push_str(&col.name);
                    output.push_str(": ");
                    if let Some(val) = row.get(j) {
                        dx_value_to_yaml_impl(val, output, indent + 2)?;
                    }
                }
            }
        }
        DxValue::Ref(id) => {
            output.push_str(&format!("\"@{id}\""));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_to_dx() {
        let yaml = r"
name: test
version: 1.0.0
items:
  - a
  - b
  - c
";
        let dx = yaml_to_dx(yaml).unwrap();
        assert!(dx.contains("test"));
    }

    #[test]
    fn test_yaml_to_document() {
        let yaml = r"
name: dx-os
version: 1.0.0
";
        let doc = yaml_to_document(yaml).unwrap();
        assert_eq!(doc.get_path("name").unwrap().as_str(), Some("dx-os"));
        assert_eq!(doc.get_path("version").unwrap().as_str(), Some("1.0.0"));
    }

    #[test]
    fn test_dx_to_yaml_simple() {
        let dx = "name:test\nversion:100";
        let yaml = dx_to_yaml(dx).unwrap();
        assert!(yaml.contains("name: test") || yaml.contains("name:test"));
        assert!(yaml.contains("version:"));
        assert!(yaml.contains("100"));
    }

    #[test]
    fn test_dx_to_yaml_doc_llm_format() {
        let dx = "name=test\nversion=1.0.0";
        let yaml = dx_to_yaml_doc(dx).unwrap();
        assert!(yaml.contains("name:"));
        assert!(yaml.contains("test"));
    }
}
