/// JSON to DX ULTRA converter
///
/// Converts JSON config files to ultra-optimized DX SINGULARITY format.
/// Automatically applies all optimization rules.
use crate::llm::types::{DxDocument, DxLlmValue, DxSection, EntryRef};
use indexmap::IndexMap;
use serde_json::Value;

/// Convert JSON string to DX ULTRA format
pub fn json_to_dx(json_str: &str) -> Result<String, String> {
    let value = parse_json_or_jsonc(json_str)?;

    let mut output = String::new();

    if let Value::Object(obj) = value {
        convert_object(&obj, "", &mut output)?;
    } else {
        return Err("JSON root must be an object".to_string());
    }

    Ok(output)
}

/// Convert a JSON object into the shared DX document model.
///
/// Arrays of uniform objects are converted to wrapped dataframes (`DxSection`),
/// while all other values remain as context entries.
/// This is the machine-cache path used by `SerializerOutput`: JSON is parsed
/// once into `DxDocument`, then the normal RKYV `.machine` writer is reused.
pub fn json_to_document(json_str: &str) -> Result<DxDocument, String> {
    let value = parse_json_or_jsonc(json_str)?;

    let Value::Object(obj) = value else {
        return Err("JSON root must be an object".to_string());
    };

    let mut doc = DxDocument::new();
    let mut next_section_id = 'a';

    for (key, value) in obj {
        if let Some(section) = array_to_section(&value) {
            let section_id = next_section_id;
            doc.sections.insert(section_id, section);
            doc.section_names.insert(section_id, key.clone());
            doc.entry_order.push(EntryRef::Section(section_id));
            next_section_id = char::from_u32(next_section_id as u32 + 1)
                .unwrap_or('z');
        } else {
            doc.context.insert(key.clone(), json_value_to_dx(value));
            doc.entry_order.push(EntryRef::Context(key));
        }
    }

    Ok(doc)
}

/// Try to convert a JSON `Value` into a `DxSection`.
///
/// Returns `Some(DxSection)` if the value is a non-empty array of
/// objects that all share the exact same set of keys (uniform schema).
/// Returns `None` for any other value type.
/// Convert a JSON array of uniform objects into a `DxSection`.
///
/// Returns `Some` if the value is a non-empty array of objects that all
/// share the exact same set of keys (uniform schema). Returns `None`
/// for any other value type.
fn array_to_section(value: &Value) -> Option<DxSection> {
    let Value::Array(items) = value else { return None };
    if items.is_empty() { return None; }

    let objects: Vec<&serde_json::Map<String, Value>> = items
        .iter()
        .map(|item| item.as_object())
        .collect::<Option<Vec<_>>>()?;

    let first_keys: Vec<&String> = objects[0].keys().collect();
    for obj in &objects[1..] {
        let keys: Vec<&String> = obj.keys().collect();
        if keys != first_keys {
            return None;
        }
    }

    let schema: Vec<String> = first_keys.into_iter().cloned().collect();
    let mut section = DxSection::new(schema);

    for obj in objects {
        let mut row = Vec::new();
        for key in &section.schema {
            let val = obj.get(key).cloned().unwrap_or(Value::Null);
            row.push(json_value_to_dx(val));
        }
        section.add_row(row).unwrap();
    }

    Some(section)
}

fn parse_json_or_jsonc(json_str: &str) -> Result<Value, String> {
    match serde_json::from_str(json_str) {
        Ok(value) => Ok(value),
        Err(strict_error) => {
            let sanitized = remove_trailing_commas(&strip_json_comments(json_str));
            serde_json::from_str(&sanitized).map_err(|jsonc_error| {
                format!(
                    "JSON parse error: {strict_error}; JSONC fallback error: {jsonc_error}"
                )
            })
        }
    }
}

fn json_value_to_dx(value: Value) -> DxLlmValue {
    match value {
        Value::String(value) => DxLlmValue::Str(value),
        Value::Number(value) => json_number_to_dx(&value),
        Value::Bool(value) => DxLlmValue::Bool(value),
        Value::Null => DxLlmValue::Null,
        Value::Array(values) => DxLlmValue::Arr(values.into_iter().map(json_value_to_dx).collect()),
        Value::Object(values) => {
            let fields = values
                .into_iter()
                .map(|(key, value)| (key, json_value_to_dx(value)))
                .collect::<IndexMap<_, _>>();
            DxLlmValue::Obj(fields)
        }
    }
}

fn json_number_to_dx(value: &serde_json::Number) -> DxLlmValue {
    const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

    if let Some(value) = value.as_i64() {
        let magnitude = value.unsigned_abs();
        if magnitude <= MAX_SAFE_INTEGER {
            return DxLlmValue::Num(value as f64);
        }
        return DxLlmValue::Str(value.to_string());
    }

    if let Some(value) = value.as_u64() {
        if value <= MAX_SAFE_INTEGER {
            return DxLlmValue::Num(value as f64);
        }
        return DxLlmValue::Str(value.to_string());
    }

    if let Some(value) = value.as_f64() {
        if value.is_finite() {
            return DxLlmValue::Num(value);
        }
    }

    DxLlmValue::Str(value.to_string())
}

fn strip_json_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    for comment_ch in chars.by_ref() {
                        if comment_ch == '\n' {
                            output.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut previous = '\0';
                    for comment_ch in chars.by_ref() {
                        if comment_ch == '\n' {
                            output.push('\n');
                        }
                        if previous == '*' && comment_ch == '/' {
                            break;
                        }
                        previous = comment_ch;
                    }
                    continue;
                }
                _ => {}
            }
        }

        output.push(ch);
    }

    output
}

fn remove_trailing_commas(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in chars.iter().copied().enumerate() {
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            continue;
        }

        if ch == ',' {
            let mut lookahead = index + 1;
            while chars
                .get(lookahead)
                .is_some_and(|next| next.is_whitespace())
            {
                lookahead += 1;
            }

            if matches!(chars.get(lookahead), Some('}' | ']')) {
                continue;
            }
        }

        output.push(ch);
    }

    output
}

/// Convert a JSON object to DX format
fn convert_object(
    obj: &serde_json::Map<String, Value>,
    prefix: &str,
    output: &mut String,
) -> Result<(), String> {
    // Group properties by type
    let mut simple_props = Vec::new();
    let mut arrays = Vec::new();
    let mut tables = Vec::new();
    let mut nested = Vec::new();

    for (key, value) in obj {
        match value {
            Value::String(_) | Value::Number(_) | Value::Bool(_) => {
                simple_props.push((key.clone(), value_to_string(value)));
            }
            Value::Array(arr) => {
                if is_table(arr) {
                    tables.push((key.clone(), arr.clone()));
                } else {
                    arrays.push((key.clone(), arr.clone()));
                }
            }
            Value::Object(nested_obj) => {
                nested.push((key.clone(), nested_obj.clone()));
            }
            Value::Null => {
                simple_props.push((key.clone(), "null".to_string()));
            }
        }
    }

    // Output simple properties (inline if possible)
    if !simple_props.is_empty() {
        let optimized_props: Vec<(String, String)> = simple_props
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if true {
            // Inline format: c.n:dx^v:0.0.1^t:Title
            let prefix_opt = if prefix.is_empty() { "c" } else { prefix };
            output.push_str(prefix_opt);
            output.push('.');
            for (i, (key, val)) in optimized_props.iter().enumerate() {
                if i > 0 {
                    output.push('^');
                }
                output.push_str(key);
                output.push(':');
                output.push_str(val);
            }
            output.push('\n');
        } else {
            // Multi-line format
            let prefix_opt = if prefix.is_empty() { "c" } else { prefix };
            for (key, val) in optimized_props {
                output.push_str(prefix_opt);
                output.push('.');
                output.push_str(&key);
                output.push(':');
                output.push_str(&val);
                output.push('\n');
            }
        }
    }

    // Output arrays with pipe separator
    for (key, arr) in arrays {
        let key_opt = key.clone();
        let items: Vec<String> = arr.iter().map(value_to_string).collect();

        let prefix_opt = if prefix.is_empty() { "" } else { prefix };
        if !prefix_opt.is_empty() {
            output.push_str(prefix_opt);
            output.push('_');
        }
        output.push_str(&key_opt);
        output.push('>');
        output.push_str(&items.join(" "));
        output.push('\n');
    }

    // Output tables
    for (key, arr) in tables {
        output.push('\n');
        let key_opt = key.clone();

        if let Some(Value::Object(first)) = arr.first() {
            let cols: Vec<String> = first.keys().map(|k| k.clone()).collect();

            output.push_str(&key_opt);
            output.push('=');
            output.push_str(&cols.join(" "));
            output.push('\n');

            for item in &arr {
                if let Value::Object(row) = item {
                    let values: Vec<String> = first
                        .keys()
                        .map(|k| value_to_string(row.get(k).unwrap_or(&Value::Null)))
                        .collect();
                    output.push_str(&values.join(" "));
                    output.push('\n');
                }
            }
        }
    }

    // Output nested objects with prefix inheritance
    for (key, nested_obj) in nested {
        output.push('\n');
        let key_opt = key.clone();
        let new_prefix = if prefix.is_empty() {
            key_opt.clone()
        } else {
            format!("{prefix}.{key_opt}")
        };

        convert_object(&nested_obj, &new_prefix, output)?;
    }

    Ok(())
}

/// Check if array is a table (array of objects with same keys)
fn is_table(arr: &[Value]) -> bool {
    if arr.is_empty() {
        return false;
    }

    if let Some(Value::Object(first)) = arr.first() {
        let keys: Vec<&String> = first.keys().collect();

        // Check all items have same structure
        arr.iter().all(|item| {
            if let Value::Object(obj) = item {
                obj.keys().count() == keys.len() && keys.iter().all(|k| obj.contains_key(*k))
            } else {
                false
            }
        })
    } else {
        false
    }
}

/// Convert JSON value to string
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Null => "null".to_string(),
        Value::Array(_) => "[array]".to_string(),
        Value::Object(_) => "[object]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_json() {
        let json = r#"{"name": "test", "version": "1.0.0"}"#;
        let dx = json_to_dx(json).unwrap();
        assert!(dx.contains("n:test"));
        assert!(dx.contains("v:1.0.0"));
    }

    #[test]
    fn test_array_json() {
        let json = r#"{"items": ["a", "b", "c"]}"#;
        let dx = json_to_dx(json).unwrap();
        assert!(dx.contains("i>a|b|c"));
    }

    #[test]
    fn test_json_to_document_preserves_nested_config() {
        let json = r#"{
            "name": "dx-js",
            "scripts": {
                "dev": "bun --watch src/index.tsx"
            },
            "dependencies": {
                "react": "latest"
            },
            "enabled": true
        }"#;

        let doc = json_to_document(json).unwrap();

        assert_eq!(doc.entry_order.len(), 4);
        assert_eq!(doc.get_path("name").unwrap().as_str(), Some("dx-js"));
        assert_eq!(
            doc.get_path("scripts.dev").unwrap().as_str(),
            Some("bun --watch src/index.tsx")
        );
        assert_eq!(
            doc.get_path("dependencies.react").unwrap().as_str(),
            Some("latest")
        );
        assert_eq!(doc.get_path("enabled").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn test_json_to_document_accepts_jsonc_config() {
        let json = r#"{
            "compilerOptions": {
                // Enable latest features
                "lib": ["ESNext",],
                "target": "ESNext"
            },
            "include": ["src",],
        }"#;

        let doc = json_to_document(json).unwrap();

        assert_eq!(
            doc.get_path("compilerOptions.target").unwrap().as_str(),
            Some("ESNext")
        );
        assert_eq!(
            doc.get_path("compilerOptions.lib")
                .unwrap()
                .as_arr()
                .unwrap()[0]
                .as_str(),
            Some("ESNext")
        );
        assert_eq!(
            doc.get_path("include").unwrap().as_arr().unwrap()[0].as_str(),
            Some("src")
        );
    }
}
