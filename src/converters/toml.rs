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

/// Convert DX format string to TOML using the DxDocument model.
///
/// Tries the LLM parser first; falls back to the old `DxValue` parser.
pub fn dx_to_toml_doc(dx_str: &str) -> Result<String, String> {
    match crate::llm::llm_to_document(dx_str) {
        Ok(doc) => {
            let mut output = String::new();
            for (k, v) in &doc.context {
                if !matches!(v, crate::llm::types::DxLlmValue::Obj(_) | crate::llm::types::DxLlmValue::Arr(_)) {
                    output.push_str(k);
                    output.push_str(" = ");
                    dx_llm_value_to_toml(v, &mut output)?;
                    output.push('\n');
                }
            }
            for (k, v) in &doc.context {
                if let crate::llm::types::DxLlmValue::Obj(nested) = v {
                    output.push('\n');
                    output.push('[');
                    output.push_str(k);
                    output.push_str("]\n");
                    for (nk, nv) in nested {
                        output.push_str(nk);
                        output.push_str(" = ");
                        dx_llm_value_to_toml(nv, &mut output)?;
                        output.push('\n');
                    }
                }
            }
            Ok(output)
        }
        Err(_) => dx_to_toml_old(dx_str),
    }
}

fn dx_llm_value_to_toml(value: &crate::llm::types::DxLlmValue, output: &mut String) -> Result<(), String> {
    match value {
        crate::llm::types::DxLlmValue::Null => output.push_str("\"\""),
        crate::llm::types::DxLlmValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        crate::llm::types::DxLlmValue::Num(n) => output.push_str(&n.to_string()),
        crate::llm::types::DxLlmValue::Str(s) => {
            output.push('"');
            output.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
            output.push('"');
        }
        crate::llm::types::DxLlmValue::Arr(items) => {
            output.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 { output.push_str(", "); }
                dx_llm_value_to_toml(item, output)?;
            }
            output.push(']');
        }
        crate::llm::types::DxLlmValue::Obj(_) => output.push_str("{}"),
        crate::llm::types::DxLlmValue::Ref(r) => {
            output.push('"'); output.push('^'); output.push_str(r); output.push('"');
        }
    }
    Ok(())
}

/// Convert DX format string to TOML (auto-detect parser).
pub fn dx_to_toml(dx_str: &str) -> Result<String, String> {
    dx_to_toml_doc(dx_str)
}

/// Convert DX format string to TOML using the old DxValue parser.
fn dx_to_toml_old(dx_str: &str) -> Result<String, String> {
    let value = crate::parser::parse(dx_str.as_bytes())
        .map_err(|e| format!("DX parse error: {e}"))?;
    let mut output = String::new();

    match &value {
        DxValue::Object(obj) => {
            for (k, v) in obj.iter() {
                if !matches!(v, DxValue::Object(_) | DxValue::Array(_) | DxValue::Table(_)) {
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
                        if i > 0 { output.push_str(", "); }
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
        DxValue::Null => output.push_str("\"\""),
        DxValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        DxValue::Int(i) => output.push_str(&i.to_string()),
        DxValue::Float(f) => output.push_str(&f.to_string()),
        DxValue::String(s) => {
            output.push('"');
            output.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
            output.push('"');
        }
        DxValue::Array(arr) => {
            output.push('[');
            for (i, item) in arr.values.iter().enumerate() {
                if i > 0 { output.push_str(", "); }
                dx_value_to_toml_value(item, output)?;
            }
            output.push(']');
        }
        DxValue::Object(_) => output.push_str("{}"),
        DxValue::Table(_) => output.push_str("[[]]"),
        DxValue::Ref(id) => {
            output.push('"'); output.push('@'); output.push_str(&id.to_string()); output.push('"');
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
