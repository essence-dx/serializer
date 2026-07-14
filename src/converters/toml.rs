/// TOML to DX ULTRA converter
use crate::converters::json::{json_to_document, json_to_dx};
use crate::llm::types::DxDocument;
use crate::types::DxValue;

/// Convert TOML string to DX ULTRA format
///
/// Strategy: Convert TOML → JSON → DX
/// This leverages existing JSON converter with optimization
pub fn toml_to_dx(toml_str: &str) -> Result<String, String> {
    // Parse TOML
    let value: toml::Value =
        toml::from_str(toml_str).map_err(|e| format!("TOML parse error: {e}"))?;

    // Convert to serde_json::Value
    let json_str =
        serde_json::to_string(&value).map_err(|e| format!("JSON conversion error: {e}"))?;

    // Use JSON converter
    json_to_dx(&json_str)
}

/// Convert TOML string into the shared DX document model.
pub fn toml_to_document(toml_str: &str) -> Result<DxDocument, String> {
    let value: toml::Value =
        toml::from_str(toml_str).map_err(|e| format!("TOML parse error: {e}"))?;
    let json_str =
        serde_json::to_string(&value).map_err(|e| format!("JSON conversion error: {e}"))?;

    json_to_document(&json_str)
}

/// Convert DX format string to TOML using the `DxDocument` model.
///
/// Tries the LLM parser first; falls back to the old `DxValue` parser.
pub fn dx_to_toml_doc(dx_str: &str) -> Result<String, String> {
    match crate::llm::llm_to_document(dx_str) {
        Ok(doc) => {
            let mut output = String::new();
            // First pass: simple values and arrays
            for (k, v) in &doc.context {
                match v {
                    crate::llm::types::DxLlmValue::Obj(_) => {} // handled in second pass
                    crate::llm::types::DxLlmValue::Arr(_) => {
                        output.push_str(k);
                        output.push_str(" = ");
                        dx_llm_value_to_toml(v, &mut output)?;
                        output.push('\n');
                    }
                    _ => {
                        output.push_str(k);
                        output.push_str(" = ");
                        dx_llm_value_to_toml(v, &mut output)?;
                        output.push('\n');
                    }
                }
            }
            // Second pass: nested objects (TOML tables), recursively
            for (k, v) in &doc.context {
                if let crate::llm::types::DxLlmValue::Obj(nested) = v {
                    write_toml_table(k, nested, &[], &mut output)?;
                }
            }
            // Third pass: DxSection tables
            for (id, section) in &doc.sections {
                let name = doc
                    .section_names
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.to_string());
                output.push('\n');
                output.push_str("[[");
                output.push_str(&name);
                output.push_str("]]\n");
                for row in &section.rows {
                    for (j, col) in section.schema.iter().enumerate() {
                        output.push_str(col);
                        output.push_str(" = ");
                        if let Some(val) = row.get(j) {
                            dx_llm_value_to_toml(val, &mut output)?;
                        }
                        output.push('\n');
                    }
                }
            }
            Ok(output)
        }
        Err(_) => dx_to_toml_old(dx_str),
    }
}

fn write_toml_table(
    name: &str,
    fields: &indexmap::IndexMap<String, crate::llm::types::DxLlmValue>,
    parents: &[String],
    output: &mut String,
) -> Result<(), String> {
    let mut table_path: Vec<String> = parents.to_vec();
    table_path.push(name.to_string());
    let header = table_path.join(".");

    output.push('\n');
    output.push('[');
    output.push_str(&header);
    output.push_str("]\n");

    for (nk, nv) in fields {
        if let crate::llm::types::DxLlmValue::Obj(nested) = nv {
            write_toml_table(nk, nested, &table_path, output)?;
        } else {
            output.push_str(nk);
            output.push_str(" = ");
            dx_llm_value_to_toml(nv, output)?;
            output.push('\n');
        }
    }
    Ok(())
}

fn escape_toml_string(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '\r' => escaped.push_str("\\r"),
            c => escaped.push(c),
        }
    }
    escaped
}

fn dx_llm_value_to_toml(
    value: &crate::llm::types::DxLlmValue,
    output: &mut String,
) -> Result<(), String> {
    match value {
        crate::llm::types::DxLlmValue::Null => {} // TOML has no null; omit
        crate::llm::types::DxLlmValue::Bool(b) => {
            output.push_str(if *b { "true" } else { "false" });
        }
        crate::llm::types::DxLlmValue::Int(i) => output.push_str(&i.to_string()),
        crate::llm::types::DxLlmValue::Num(n) => output.push_str(&n.to_string()),
        crate::llm::types::DxLlmValue::Str(s) => {
            output.push('"');
            output.push_str(&escape_toml_string(s));
            output.push('"');
        }
        crate::llm::types::DxLlmValue::Arr(items) => {
            output.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                dx_llm_value_to_toml(item, output)?;
            }
            output.push(']');
        }
        crate::llm::types::DxLlmValue::Obj(_) => {} // handled by caller
        crate::llm::types::DxLlmValue::Ref(r) => {
            output.push('"');
            output.push('^');
            output.push_str(r);
            output.push('"');
        }
    }
    Ok(())
}

/// Convert DX format string to TOML (auto-detect parser).
pub fn dx_to_toml(dx_str: &str) -> Result<String, String> {
    dx_to_toml_doc(dx_str)
}

/// Convert DX format string to TOML using the old `DxValue` parser.
fn dx_to_toml_old(dx_str: &str) -> Result<String, String> {
    let value =
        crate::parser::parse(dx_str.as_bytes()).map_err(|e| format!("DX parse error: {e}"))?;
    let mut output = String::new();

    match &value {
        DxValue::Object(obj) => {
            for (k, v) in obj.iter() {
                if !matches!(
                    v,
                    DxValue::Object(_) | DxValue::Array(_) | DxValue::Table(_)
                ) {
                    output.push_str(k);
                    output.push_str(" = ");
                    dx_value_to_toml_value(v, &mut output)?;
                    output.push('\n');
                }
            }

            for (k, v) in obj.iter() {
                if let DxValue::Object(nested) = v {
                    output.push('\n');
                    output.push('[');
                    output.push_str(k);
                    output.push_str("]\n");
                    for (nk, nv) in nested.iter() {
                        output.push_str(nk);
                        output.push_str(" = ");
                        dx_value_to_toml_value(nv, &mut output)?;
                        output.push('\n');
                    }
                }
            }

            for (k, v) in obj.iter() {
                if let DxValue::Array(arr) = v {
                    output.push_str(k);
                    output.push_str(" = [");
                    for (i, item) in arr.values.iter().enumerate() {
                        if i > 0 {
                            output.push_str(", ");
                        }
                        dx_value_to_toml_value(item, &mut output)?;
                    }
                    output.push_str("]\n");
                }
            }
        }
        _ => return Err("TOML root must be an object".to_string()),
    }

    Ok(output)
}

fn dx_value_to_toml_value(value: &DxValue, output: &mut String) -> Result<(), String> {
    match value {
        DxValue::Null => {} // TOML has no null; omit
        DxValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        DxValue::Int(i) => output.push_str(&i.to_string()),
        DxValue::Float(f) => output.push_str(&f.to_string()),
        DxValue::String(s) => {
            output.push('"');
            output.push_str(&escape_toml_string(s));
            output.push('"');
        }
        DxValue::Array(arr) => {
            output.push('[');
            for (i, item) in arr.values.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                dx_value_to_toml_value(item, output)?;
            }
            output.push(']');
        }
        DxValue::Object(obj) => {
            output.push_str("{ ");
            for (i, (k, v)) in obj.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(k);
                output.push_str(": ");
                dx_value_to_toml_value(v, output)?;
            }
            output.push_str(" }");
        }
        DxValue::Table(table) => {
            output.push_str("[ ");
            for row in &table.rows {
                for (j, _col) in table.schema.columns.iter().enumerate() {
                    if j > 0 {
                        output.push(' ');
                    }
                    if let Some(val) = row.get(j) {
                        dx_value_to_toml_value(val, output)?;
                    }
                }
                output.push(';');
            }
            output.push_str(" ]");
        }
        DxValue::Ref(id) => {
            output.push('"');
            output.push('@');
            output.push_str(&id.to_string());
            output.push('"');
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_to_dx() {
        let toml = r#"
name = "test"
version = "1.0.0"
items = ["a", "b", "c"]
"#;
        let dx = toml_to_dx(toml).unwrap();
        assert!(dx.contains("test"));
    }

    #[test]
    fn test_toml_to_document() {
        let doc = toml_to_document(
            r#"
name = "bun"

[install]
cache = true
"#,
        )
        .unwrap();

        assert_eq!(doc.get_path("name").unwrap().as_str(), Some("bun"));
        assert_eq!(doc.get_path("install.cache").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn test_dx_to_toml_simple() {
        let dx = "name:test\nversion:100";
        let toml = dx_to_toml(dx).unwrap();
        assert!(toml.contains("name ="));
        assert!(toml.contains("\"test\""));
        assert!(toml.contains("version ="));
        assert!(toml.contains("100"));
    }

    #[test]
    fn test_dx_to_toml_doc_llm_format() {
        let dx = "name=test\nversion=1.0.0";
        let toml = dx_to_toml_doc(dx).unwrap();
        assert!(toml.contains("name ="));
        assert!(toml.contains("\"test\""));
    }
}
