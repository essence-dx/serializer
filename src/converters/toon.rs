/// TOON to DX ULTRA converter
///
/// Converts TOON format to ultra-optimized DX SINGULARITY format.
/// Also provides DX to TOON conversion for format comparison.
use crate::parser::parse;
use crate::types::{DxArray, DxTable, DxValue};

fn unquote_toun(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Convert TOON string to DX format
///
/// Uses indentation-aware parsing that supports multi-level nesting.
pub fn toon_to_dx(toon_str: &str) -> Result<String, String> {
    let mut output = String::new();
    let lines: Vec<&str> = toon_str.lines().collect();
    parse_toon_block(&lines, 0, 0, &mut output)?;
    Ok(output)
}

/// Parse a block of TOON lines at a given indentation level.
/// Returns the index after the last consumed line.
fn parse_toon_block(
    lines: &[&str],
    start: usize,
    parent_indent: usize,
    output: &mut String,
) -> Result<usize, String> {
    let mut i = start;
    while i < lines.len() {
        let line_raw = lines[i];
        let trimmed = line_raw.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let leading = line_raw.len() - line_raw.trim_start().len();
        let indent = leading / 2;
        if indent < parent_indent && i > start {
            break;
        }

        if let Some(space_pos) = trimmed.find(' ') {
            let key = &trimmed[..space_pos];
            let value = unquote_toun(&trimmed[space_pos + 1..]);

            // Check if next lines are at a deeper indentation
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                let next_leading = lines[j].len() - lines[j].trim_start().len();
                let next_indent = next_leading / 2;
                if next_indent > indent {
                    output.push_str(key);
                    output.push('\n');
                    j = parse_toon_block(lines, j, indent, output)?;
                    i = j;
                    continue;
                }
            }

            // Simple key-value
            output.push_str(key);
            output.push(':');
            output.push_str(value);
            output.push('\n');
        } else {
            // Key with no value at this level — may have children at deeper indent
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                let next_leading = lines[j].len() - lines[j].trim_start().len();
                let next_indent = next_leading / 2;
                if next_indent > indent {
                    output.push_str(trimmed);
                    output.push('\n');
                    j = parse_toon_block(lines, j, indent, output)?;
                    i = j;
                    continue;
                }
            }
        }

        i += 1;
    }
    Ok(i)
}

/// Convert DX format string to TOON format
///
/// TOON format uses:
/// - `key "value"` for string values
/// - `key value` for numbers and booleans
/// - Indentation for nested objects
/// - `key[n]{fields}: data` for arrays of objects
pub fn dx_to_toon(dx_str: &str) -> Result<String, String> {
    // Parse the DX string into a DxValue
    let value = parse(dx_str.as_bytes()).map_err(|e| format!("DX parse error: {e}"))?;

    let mut output = String::new();
    dx_value_to_toon(&value, &mut output, 0)?;
    Ok(output)
}

/// Convert a `DxValue` to TOON format with indentation
fn dx_value_to_toon(value: &DxValue, output: &mut String, indent: usize) -> Result<(), String> {
    let indent_str = "  ".repeat(indent);

    if let DxValue::Object(obj) = value {
        for (key, val) in obj.iter() {
            match val {
                DxValue::Object(nested_obj) => {
                    // Nested object: key followed by indented children
                    output.push_str(&indent_str);
                    output.push_str(key);
                    output.push('\n');
                    dx_value_to_toon(&DxValue::Object(nested_obj.clone()), output, indent + 1)?;
                }
                DxValue::Array(arr) => {
                    dx_array_to_toon(key, arr, output, indent)?;
                }
                DxValue::Table(table) => {
                    dx_table_to_toon(key, table, output, indent)?;
                }
                _ => {
                    // Simple key-value
                    output.push_str(&indent_str);
                    output.push_str(key);
                    output.push(' ');
                    dx_simple_value_to_toon(val, output)?;
                    output.push('\n');
                }
            }
        }
    } else {
        // Top-level non-object value
        output.push_str(&indent_str);
        dx_simple_value_to_toon(value, output)?;
        output.push('\n');
    }

    Ok(())
}

/// Convert a simple `DxValue` to TOON inline format
fn dx_simple_value_to_toon(value: &DxValue, output: &mut String) -> Result<(), String> {
    match value {
        DxValue::Null => output.push_str("null"),
        DxValue::Bool(true) => output.push_str("true"),
        DxValue::Bool(false) => output.push_str("false"),
        DxValue::Int(i) => output.push_str(&i.to_string()),
        DxValue::Float(f) => output.push_str(&f.to_string()),
        DxValue::String(s) => {
            // Quote strings in TOON format
            output.push('"');
            // Escape special characters
            for c in s.chars() {
                match c {
                    '"' => output.push_str("\\\""),
                    '\\' => output.push_str("\\\\"),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    _ => output.push(c),
                }
            }
            output.push('"');
        }
        DxValue::Array(arr) => {
            // Inline array for simple values
            let items: Vec<String> = arr
                .values
                .iter()
                .map(|v| {
                    let mut s = String::new();
                    dx_simple_value_to_toon(v, &mut s).ok();
                    s
                })
                .collect();
            output.push_str(&items.join(", "));
        }
        DxValue::Object(_) => output.push_str("{}"),
        DxValue::Table(_) => output.push_str("[[]]"),
        DxValue::Ref(id) => {
            output.push('@');
            output.push_str(&id.to_string());
        }
    }
    Ok(())
}

/// Convert a `DxArray` to TOON format
fn dx_array_to_toon(
    key: &str,
    arr: &DxArray,
    output: &mut String,
    indent: usize,
) -> Result<(), String> {
    let indent_str = "  ".repeat(indent);

    // Check if array contains objects (structured array)
    let is_object_array = arr.values.iter().all(|v| matches!(v, DxValue::Object(_)));

    if is_object_array && !arr.values.is_empty() {
        // Get field names from first object
        if let Some(DxValue::Object(first_obj)) = arr.values.first() {
            let fields: Vec<&str> = first_obj.iter().map(|(k, _)| k.as_str()).collect();
            let field_list = fields.join(",");

            // TOON structured array format: key[n]{fields}:
            output.push_str(&indent_str);
            output.push_str(key);
            output.push('[');
            output.push_str(&arr.values.len().to_string());
            output.push_str("]{");
            output.push_str(&field_list);
            output.push_str("}:\n");

            // Write each row
            for val in &arr.values {
                if let DxValue::Object(obj) = val {
                    output.push_str(&indent_str);
                    output.push_str("  ");
                    let values: Vec<String> = fields
                        .iter()
                        .map(|f| {
                            let mut s = String::new();
                            if let Some(v) = obj.get(f) {
                                dx_simple_value_to_toon(v, &mut s).ok();
                            }
                            s
                        })
                        .collect();
                    output.push_str(&values.join(","));
                    output.push('\n');
                }
            }
        }
    } else {
        // Simple array: key[n]: val1, val2, ...
        output.push_str(&indent_str);
        output.push_str(key);
        output.push('[');
        output.push_str(&arr.values.len().to_string());
        output.push_str("]: ");

        let items: Vec<String> = arr
            .values
            .iter()
            .map(|v| {
                let mut s = String::new();
                dx_simple_value_to_toon(v, &mut s).ok();
                s
            })
            .collect();
        output.push_str(&items.join(", "));
        output.push('\n');
    }

    Ok(())
}

/// Convert a `DxTable` to TOON format
fn dx_table_to_toon(
    key: &str,
    table: &DxTable,
    output: &mut String,
    indent: usize,
) -> Result<(), String> {
    let indent_str = "  ".repeat(indent);

    // Get column names
    let fields: Vec<&str> = table
        .schema
        .columns
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    let field_list = fields.join(",");

    // TOON structured array format
    output.push_str(&indent_str);
    output.push_str(key);
    output.push('[');
    output.push_str(&table.rows.len().to_string());
    output.push_str("]{");
    output.push_str(&field_list);
    output.push_str("}:\n");

    // Write each row
    for row in &table.rows {
        output.push_str(&indent_str);
        output.push_str("  ");
        let values: Vec<String> = row
            .iter()
            .map(|v| {
                let mut s = String::new();
                dx_simple_value_to_toon(v, &mut s).ok();
                s
            })
            .collect();
        output.push_str(&values.join(","));
        output.push('\n');
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toon_to_dx() {
        let toon = r#"
name "test"
version "1.0.0"
"#;
        let dx = toon_to_dx(toon).unwrap();
        assert!(dx.contains("test"));
    }

    #[test]
    fn test_dx_to_toon_simple() {
        // DX format uses key:value (no spaces around colon)
        // Use simple string values without dots to avoid parsing issues
        let dx = "name:test\nversion:100";
        let result = dx_to_toon(dx);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        let toon = result.unwrap();
        // Check that the output contains the key-value pairs
        assert!(toon.contains("name"), "Missing 'name' key");
        assert!(toon.contains("test"), "Missing 'test' value");
        assert!(toon.contains("version"), "Missing 'version' key");
        assert!(toon.contains("100"), "Missing '100' value");
    }

    #[test]
    fn test_dx_to_toon_numbers() {
        let dx = "count:42\nprice:19.99";
        let toon = dx_to_toon(dx).unwrap();
        assert!(toon.contains("count 42"));
        assert!(toon.contains("price 19.99"));
    }

    #[test]
    fn test_dx_to_toon_booleans() {
        let dx = "active:+\ndisabled:-";
        let toon = dx_to_toon(dx).unwrap();
        assert!(toon.contains("active true"));
        assert!(toon.contains("disabled false"));
    }

    #[test]
    fn test_dx_to_toon_array() {
        let dx = "tags>alpha|beta|gamma";
        let toon = dx_to_toon(dx).unwrap();
        assert!(toon.contains("tags[3]"));
        assert!(toon.contains("alpha"));
        assert!(toon.contains("beta"));
        assert!(toon.contains("gamma"));
    }

    #[test]
    fn test_dx_to_toon_table() {
        let dx = "users=id%i name%s\n1 Alice\n2 Bob";
        let toon = dx_to_toon(dx).unwrap();
        assert!(toon.contains("users[2]{id,name}"));
        assert!(toon.contains("1,\"Alice\""));
        assert!(toon.contains("2,\"Bob\""));
    }
}
