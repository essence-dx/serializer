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

/// Convert a DxValue to a serde_json::Value
fn dx_value_to_json_value(value: &crate::types::DxValue) -> Result<serde_json::Value, String> {
    match value {
        crate::types::DxValue::Null => Ok(serde_json::Value::Null),
        crate::types::DxValue::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        crate::types::DxValue::Int(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
        crate::types::DxValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .ok_or_else(|| "Invalid float value".to_string()),
        crate::types::DxValue::String(s) => Ok(serde_json::Value::String(s.clone())),
        crate::types::DxValue::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.values.iter().map(dx_value_to_json_value).collect();
            Ok(serde_json::Value::Array(items?))
        }
        crate::types::DxValue::Object(obj) => {
            let mut map = serde_json::Map::new();
            for (k, v) in obj.iter() {
                map.insert(k.clone(), dx_value_to_json_value(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        crate::types::DxValue::Table(table) => {
            let mut rows = Vec::new();
            for row in &table.rows {
                let mut obj = serde_json::Map::new();
                for (i, col) in table.schema.columns.iter().enumerate() {
                    if let Some(val) = row.get(i) {
                        obj.insert(col.name.clone(), dx_value_to_json_value(val)?);
                    }
                }
                rows.push(serde_json::Value::Object(obj));
            }
            Ok(serde_json::Value::Array(rows))
        }
        crate::types::DxValue::Ref(id) => Ok(serde_json::Value::String(format!("@{id}"))),
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

/// Convert DX format string to JSON using the DxDocument model.
///
/// Tries the LLM parser first; falls back to the old `DxValue` parser.
/// Set `pretty` to `false` for compact (single-line) JSON output.
pub fn dx_to_json_doc(dx_str: &str, pretty: bool) -> Result<String, String> {
    let doc = match crate::llm::llm_to_document(dx_str) {
        Ok(doc) => doc,
        Err(_) => {
            let parsed = crate::parser::parse(dx_str.as_bytes())
                .map_err(|e| format!("DX parse error: {e}"))?;
            return if pretty {
                let json = dx_value_to_json_value(&parsed)?;
                serde_json::to_string_pretty(&json)
                    .map_err(|e| format!("JSON serialization error: {e}"))
            } else {
                let json = dx_value_to_json_value(&parsed)?;
                serde_json::to_string(&json)
                    .map_err(|e| format!("JSON serialization error: {e}"))
            };
        }
    };
    let json_value = dx_document_to_json_value(&doc);
    if pretty {
        serde_json::to_string_pretty(&json_value)
            .map_err(|e| format!("JSON serialization error: {e}"))
    } else {
        serde_json::to_string(&json_value)
            .map_err(|e| format!("JSON serialization error: {e}"))
    }
}

/// Convert `DxDocument` to a `serde_json::Value`.
fn dx_document_to_json_value(doc: &crate::llm::types::DxDocument) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (k, v) in &doc.context {
        map.insert(k.clone(), dx_llm_value_to_json(v));
    }
    for (id, section) in &doc.sections {
        let name = doc.section_names.get(id).cloned().unwrap_or_else(|| id.to_string());
        let rows: Vec<serde_json::Value> = section.rows.iter().map(|row| {
            let mut obj = serde_json::Map::new();
            for (i, col) in section.schema.iter().enumerate() {
                if let Some(val) = row.get(i) {
                    obj.insert(col.clone(), dx_llm_value_to_json(val));
                }
            }
            serde_json::Value::Object(obj)
        }).collect();
        map.insert(name, serde_json::Value::Array(rows));
    }
    serde_json::Value::Object(map)
}

/// Convert a `DxLlmValue` to a `serde_json::Value`.
fn dx_llm_value_to_json(value: &crate::llm::types::DxLlmValue) -> serde_json::Value {
    use crate::llm::types::DxLlmValue;
    match value {
        DxLlmValue::Null => serde_json::Value::Null,
        DxLlmValue::Bool(b) => serde_json::Value::Bool(*b),
        DxLlmValue::Num(n) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::String(n.to_string())),
        DxLlmValue::Str(s) => serde_json::Value::String(s.clone()),
        DxLlmValue::Arr(items) => {
            serde_json::Value::Array(items.iter().map(dx_llm_value_to_json).collect())
        }
        DxLlmValue::Obj(fields) => {
            let mut map = serde_json::Map::new();
            for (k, v) in fields {
                map.insert(k.clone(), dx_llm_value_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        DxLlmValue::Ref(r) => serde_json::Value::String(format!("^{r}")),
    }
}

/// Update `dx_to_json()` to try the LLM parser first, with fallback.
pub fn dx_to_json(dx_str: &str) -> Result<String, String> {
    dx_to_json_doc(dx_str, true)
}

/// Convert DX format string to compact (single-line) JSON.
pub fn dx_to_json_min(dx_str: &str) -> Result<String, String> {
    dx_to_json_doc(dx_str, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_json() {
        let json = r#"{"name": "test", "version": "1.0.0"}"#;
        let dx = json_to_dx(json).unwrap();
        assert!(dx.contains("test"));
        assert!(dx.contains("1.0.0"));
    }

    #[test]
    fn test_array_json() {
        let json = r#"{"items": ["a", "b", "c"]}"#;
        let dx = json_to_dx(json).unwrap();
        assert!(dx.contains("a") && dx.contains("b") && dx.contains("c"));
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

    #[test]
    fn test_dx_to_json_simple() {
        let dx = "name:test\nversion:100";
        let json = dx_to_json(dx).unwrap();
        assert!(json.contains("name"));
        assert!(json.contains("\"test\""));
        assert!(json.contains("version"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_dx_to_json_round_trip() {
        let original = r#"{"name":"test","count":42,"active":true}"#;
        let dx = json_to_dx(original).unwrap();
        let result = dx_to_json(&dx).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.as_object().unwrap().len() > 0);
    }

    #[test]
    fn test_dx_to_json_nested() {
        let dx = "server.host:localhost\nserver.port:8080\nenabled:+";
        let json = dx_to_json(dx).unwrap();
        assert!(json.contains("server") || json.contains("host"));
    }

    #[test]
    fn test_dx_to_json_array() {
        let dx = "tags>alpha|beta|gamma";
        let json = dx_to_json(dx).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["tags"][0], "alpha");
    }

    #[test]
    fn test_dx_to_json_min_compact() {
        let dx = "name:test\ncount:42";
        let json = dx_to_json_min(dx).unwrap();
        assert!(!json.contains('\n'));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_dx_to_json_doc_llm_format() {
        let dx = "name=test\nversion=1.0.0";
        let json = dx_to_json_doc(dx, true).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["name"], "test");
        assert_eq!(parsed["version"], "1.0.0");
    }
}
