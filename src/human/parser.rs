//! Human format parser
//!
//! Parses beautiful human-readable format back to `DxDocument`.
//!
//! ## Human Format Syntax (v3 with Groups & Tables)
//!
//! ```text
//! # Parenthesized groups (3+ children)
//! project(
//!   name    = dx-os
//!   version = 1.0.0
//! )
//!
//! # Tables with schema (auto-detects space or comma per row)
//! recipes[name,group,doc,script](
//!   build,all,"Build all workspace crates","cargo build --workspace"
//! )
//!
//! aliases[name target](
//!   b  build
//!   br build-release
//! )
//!
//! # Scalars with dot paths (leaf values)
//! name                = dx-os
//! version             = 1.0.0
//! forge.repository    = https://dx.vercel.app/user/repo
//!
//! # Arrays with count: key[n]:
//! workspace.paths[2]:
//! - @/www
//! - @/backend
//! ```

use crate::llm::types::{DxDocument, DxLlmValue, DxSection};
use indexmap::IndexMap;
use thiserror::Error;

/// Parse errors for Human format
#[derive(Debug, Error)]
pub enum HumanParseError {
    /// A legacy section header could not be parsed.
    #[error("Invalid section header: {msg}")]
    InvalidSectionHeader {
        /// Human-readable parse failure.
        msg: String,
    },

    /// A key-value line was malformed.
    #[error("Invalid key-value pair: {msg}")]
    InvalidKeyValue {
        /// Human-readable parse failure.
        msg: String,
    },

    /// Table syntax could not be parsed at the reported line.
    #[error("Invalid table format at line {line}: {msg}")]
    InvalidTable {
        /// One-based line number where the table failed.
        line: usize,
        /// Human-readable parse failure.
        msg: String,
    },

    /// Parser found content that did not match any supported human-format form.
    #[error("Unexpected content: {msg}")]
    UnexpectedContent {
        /// Human-readable parse failure.
        msg: String,
    },

    /// Input exceeded the configured serializer safety limit.
    #[error("Input too large: {size} bytes exceeds maximum of {max} bytes")]
    InputTooLarge {
        /// Actual input size in bytes.
        size: usize,
        /// Maximum accepted input size in bytes.
        max: usize,
    },

    /// Table row count exceeded the configured serializer safety limit.
    #[error("Table too large: {rows} rows exceeds maximum of {max} rows")]
    TableTooLarge {
        /// Actual parsed row count.
        rows: usize,
        /// Maximum accepted row count.
        max: usize,
    },
}

/// Parse human-readable format back to `DxDocument`
pub struct HumanParser {}

impl HumanParser {
    #[allow(dead_code)] // Methods reserved for future table parsing features
    /// Create a new parser
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Parse human format string into `DxDocument`
    ///
    /// Supports v2 format with:
    /// - Dot notation for nested paths: `forge.repository = value`
    /// - Array syntax: `key[n]:` followed by `- item` lines
    /// - Leaf inlining: dots in keys are preserved as-is
    ///
    /// # Errors
    ///
    /// Returns `HumanParseError::InputTooLarge` if input exceeds `MAX_INPUT_SIZE` (100 MB).
    pub fn parse(&self, input: &str) -> Result<DxDocument, HumanParseError> {
        // Security: Check input size before parsing
        if input.len() > crate::error::MAX_INPUT_SIZE {
            return Err(HumanParseError::InputTooLarge {
                size: input.len(),
                max: crate::error::MAX_INPUT_SIZE,
            });
        }

        let mut doc = DxDocument::new();
        let lines: Vec<&str> = input.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Skip empty lines and comment headers (═══)
            if line.is_empty() || line.starts_with("# ═") || line.starts_with('#') {
                i += 1;
                continue;
            }

            // Track what was added before this iteration
            let context_keys_before: Vec<String> = doc.context.keys().cloned().collect();
            let section_ids_before: Vec<char> = doc.sections.keys().copied().collect();

            // Check for table with schema: name[col1,col2,...](...)
            if let Some((table_name, schema_str, remainder)) = self.parse_table_header(line) {
                let schema: Vec<String> = if schema_str.contains(',') {
                    schema_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                } else {
                    schema_str
                        .split_whitespace()
                        .map(std::string::ToString::to_string)
                        .collect()
                };

                let _lines_after = if remainder.is_empty() {
                    i += 1;
                    &lines[i..]
                } else {
                    &[]
                };

                if remainder == ")" && !schema.is_empty() {
                    // Single-line table: name[headers]() - empty table
                    let section = DxSection::new(schema);
                    doc.section_names
                        .insert(table_name.chars().next().unwrap_or('t'), table_name.clone());
                    doc.sections
                        .insert(table_name.chars().next().unwrap_or('t'), section);
                    self.track_new_entries(&mut doc, &context_keys_before, &section_ids_before);
                    i += 1;
                    continue;
                }

                let mut rows = Vec::new();
                let mut row_lines: Vec<String> = Vec::new();
                let mut paren_depth = 0;
                let mut consumed = 0;

                if remainder.starts_with('(') {
                    paren_depth = 1;
                    for ch in remainder[1..].chars() {
                        if ch == '(' {
                            paren_depth += 1;
                        } else if ch == ')' {
                            paren_depth -= 1;
                        }
                    }
                }

                if paren_depth == 0 && !remainder.is_empty() {
                    if remainder.ends_with(')') {
                        let inner = remainder
                            [remainder.find('(').map_or(0, |p| p + 1)..remainder.len() - 1]
                            .trim();
                        if !inner.is_empty() {
                            row_lines.push(inner.to_string());
                        }
                    }
                } else if paren_depth > 0 {
                    if !remainder.is_empty() {
                        let paren_start = remainder.find('(').map_or(0, |p| p + 1);
                        let rest_inner = remainder[paren_start..].trim();
                        if !rest_inner.is_empty() {
                            row_lines.push(rest_inner.to_string());
                        }
                    }
                    let line_offset = usize::from(!remainder.is_empty()); // skip header line if already on same line
                    let mut in_quoted_string = false;
                    while consumed < lines.len().saturating_sub(i + line_offset) {
                        let rline = lines[i + line_offset + consumed].trim();
                        if rline.is_empty() && row_lines.is_empty() {
                            consumed += 1;
                            continue;
                        }
                        for ch in rline.chars() {
                            if ch == '(' {
                                paren_depth += 1;
                            } else if ch == ')' {
                                paren_depth -= 1;
                            }
                        }
                        // Count quotes to detect multi-line quoted strings
                        let quote_count = rline.chars().filter(|&c| c == '"').count();
                        if quote_count % 2 == 1 {
                            in_quoted_string = !in_quoted_string;
                        }
                        if paren_depth == 0 && !rline.is_empty() && !in_quoted_string {
                            let clean = rline.trim_end_matches(')').trim();
                            if !clean.is_empty() {
                                row_lines.push(clean.to_string());
                            }
                            consumed += 1;
                            break;
                        }
                        if !rline.is_empty() {
                            if in_quoted_string && !row_lines.is_empty() {
                                // Continue the previous quoted string onto this line
                                let last = row_lines.len() - 1;
                                row_lines[last] = [row_lines[last].as_str(), " ", rline].concat();
                            } else {
                                row_lines.push(rline.to_string());
                            }
                        }
                        consumed += 1;
                    }
                }

                for row_str in &row_lines {
                    let row_str = row_str.trim().trim_end_matches(')').trim();
                    if row_str.is_empty() {
                        continue;
                    }

                    let cells: Vec<DxLlmValue> = self.parse_row_cells(row_str, &schema);
                    if !cells.is_empty() {
                        rows.push(cells);
                    }
                }

                if !schema.is_empty() {
                    let mut section = DxSection::new(schema);
                    section.rows = rows;
                    let section_id = table_name.chars().next().unwrap_or('t');
                    doc.section_names.insert(section_id, table_name.clone());
                    doc.sections.insert(section_id, section);
                }

                i += usize::from(remainder.is_empty()) + consumed;
                self.track_new_entries(&mut doc, &context_keys_before, &section_ids_before);
                continue;
            }

            // Check for parenthesized group: name(key = value ...)
            if let Some((ref group_name, inner_content)) =
                self.parse_parenthesized_group(line, &lines[i..])
            {
                let inner_doc = self.parse(&inner_content)?;
                let has_context = !inner_doc.context.is_empty();
                let has_sections = !inner_doc.sections.is_empty();
                if has_context {
                    doc.context
                        .insert(group_name.clone(), DxLlmValue::Obj(inner_doc.context));
                }
                for (id, section) in inner_doc.sections {
                    let full_name = inner_doc
                        .section_names
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| id.to_string());
                    doc.section_names.insert(id, full_name);
                    doc.sections.insert(id, section);
                    // Ensure sections appear in entry_order for formatters
                    let entry_ref = crate::llm::types::EntryRef::Section(id);
                    if !doc.entry_order.contains(&entry_ref) {
                        doc.entry_order.push(entry_ref);
                    }
                }
                if !has_context && !has_sections {
                    doc.context
                        .insert(group_name.clone(), DxLlmValue::Obj(IndexMap::new()));
                }
                i += 1;
                self.track_new_entries(&mut doc, &context_keys_before, &section_ids_before);
                continue;
            }

            // Check for array syntax: key[n]: or key.path[n]:
            if let Some(caps) = self.parse_array_header(line) {
                let (key, _count) = caps;
                i += 1;

                // Collect array items
                let mut items = Vec::new();
                while i < lines.len() {
                    let item_line = lines[i].trim();
                    if item_line.starts_with("- ") {
                        let item_value = item_line.strip_prefix("- ").unwrap_or("").trim();
                        items.push(self.parse_config_value(item_value)?);
                        i += 1;
                    } else if item_line.starts_with('-') && item_line.len() > 1 {
                        let item_value = item_line.strip_prefix('-').unwrap_or("").trim();
                        items.push(self.parse_config_value(item_value)?);
                        i += 1;
                    } else {
                        break;
                    }
                }

                if !items.is_empty() {
                    doc.context.insert(key.clone(), DxLlmValue::Arr(items));
                }

                // Track new entries
                self.track_new_entries(&mut doc, &context_keys_before, &section_ids_before);
                continue;
            }

            // Check for section header (legacy support): [section] or [section:number]
            if let Some(section_name) = self.parse_section_header(line) {
                i += 1;

                // Check if this is a numbered section like [dependencies:1]
                if let Some(colon_pos) = section_name.rfind(':') {
                    let base_name = &section_name[..colon_pos];

                    // Parse this numbered section
                    let (context, consumed) = self.parse_config_section(&lines[i..])?;
                    i += consumed;

                    // Check if we already have a section for this base name
                    if let Some(DxLlmValue::Obj(existing)) = doc.context.get_mut(base_name) {
                        // This is a subsequent numbered section - convert to table
                        // First, get the schema from the first object
                        let mut schema: Vec<String> = existing.keys().cloned().collect();
                        schema.sort();

                        // Create a table section
                        let mut section = DxSection::new(schema.clone());

                        // Add first row from existing object
                        let mut first_row = Vec::new();
                        for col in &schema {
                            first_row.push(existing.get(col).cloned().unwrap_or(DxLlmValue::Null));
                        }
                        section.rows.push(first_row);

                        // Add second row from current context
                        let mut second_row = Vec::new();
                        for col in &schema {
                            second_row.push(context.get(col).cloned().unwrap_or(DxLlmValue::Null));
                        }
                        section.rows.push(second_row);

                        // Remove the object and add as section with full name
                        doc.context.shift_remove(base_name);
                        let section_id = base_name.chars().next().unwrap_or('d');
                        doc.section_names.insert(section_id, base_name.to_string());
                        doc.sections.insert(section_id, section);
                    } else if let Some(section) = doc.sections.values_mut().find(|s| {
                        // Find existing table section for this base name
                        s.schema.iter().all(|col| context.contains_key(col))
                    }) {
                        // Add row to existing table
                        let mut row = Vec::new();
                        for col in &section.schema.clone() {
                            row.push(context.get(col).cloned().unwrap_or(DxLlmValue::Null));
                        }
                        section.rows.push(row);
                    } else {
                        // First numbered section - store as object for now
                        if !context.is_empty() {
                            doc.context
                                .insert(base_name.to_string(), DxLlmValue::Obj(context));
                        }
                    }

                    // Track new entries
                    self.track_new_entries(&mut doc, &context_keys_before, &section_ids_before);
                    continue;
                }

                match section_name.to_lowercase().as_str() {
                    "config" | "configuration" => {
                        let (context, consumed) = self.parse_config_section(&lines[i..])?;
                        for (k, v) in context {
                            doc.context.insert(k, v);
                        }
                        i += consumed;
                    }
                    "references" | "refs" => {
                        let (refs, consumed) = self.parse_references_section(&lines[i..])?;
                        doc.refs = refs;
                        i += consumed;
                    }
                    _ => {
                        // Parse as config-style section (key-value pairs)
                        let (context, consumed) = self.parse_config_section(&lines[i..])?;

                        // Add section data to context as nested object
                        if !context.is_empty() {
                            // Keep section name as-is (no compression)
                            doc.context
                                .insert(section_name.clone(), DxLlmValue::Obj(context));
                        }
                        i += consumed;
                    }
                }

                // Track new entries
                self.track_new_entries(&mut doc, &context_keys_before, &section_ids_before);
                continue;
            }

            // Key-value pair: key = value or key.path = value
            if line.contains('=') && !line.starts_with('[') {
                if let Some((key, value)) = self.parse_key_value(line)? {
                    // Keep the key as-is (with dots) for leaf inlining
                    doc.context.insert(key, value);
                }
                i += 1;

                // Track new entries
                self.track_new_entries(&mut doc, &context_keys_before, &section_ids_before);
                continue;
            }

            // Legacy array syntax: key: followed by - items
            if line.ends_with(':') && !line.contains('=') && !line.contains('[') {
                let key = line.trim_end_matches(':').trim().to_string();
                i += 1;

                // Collect array items
                let mut items = Vec::new();
                while i < lines.len() {
                    let item_line = lines[i].trim();
                    if item_line.starts_with("- ") {
                        let item_value = item_line.strip_prefix("- ").unwrap_or("").trim();
                        items.push(self.parse_config_value(item_value)?);
                        i += 1;
                    } else if item_line.starts_with('-') && item_line.len() > 1 {
                        let item_value = item_line.strip_prefix('-').unwrap_or("").trim();
                        items.push(self.parse_config_value(item_value)?);
                        i += 1;
                    } else {
                        break;
                    }
                }

                if !items.is_empty() {
                    doc.context.insert(key, DxLlmValue::Arr(items));
                }
                continue;
            }

            i += 1;
        }

        Ok(doc)
    }

    /// Track new entries added since the last snapshot
    fn track_new_entries(
        &self,
        doc: &mut DxDocument,
        context_keys_before: &[String],
        section_ids_before: &[char],
    ) {
        // Find new context keys
        for key in doc.context.keys() {
            if !context_keys_before.contains(key) {
                let entry_ref = crate::llm::types::EntryRef::Context(key.clone());
                if !doc.entry_order.contains(&entry_ref) {
                    doc.entry_order.push(entry_ref);
                }
            }
        }

        // Find new section IDs
        for id in doc.sections.keys() {
            if !section_ids_before.contains(id) {
                let entry_ref = crate::llm::types::EntryRef::Section(*id);
                if !doc.entry_order.contains(&entry_ref) {
                    doc.entry_order.push(entry_ref);
                }
            }
        }
    }

    /// Parse array header: key[n]: or key.path[n]:
    /// Returns (key, count) if matched
    fn parse_array_header(&self, line: &str) -> Option<(String, usize)> {
        let line = line.trim();

        // Match pattern: key[n]: or key.path[n]:
        if !line.ends_with(':') {
            return None;
        }

        let without_colon = &line[..line.len() - 1];

        // Find the [n] part
        if let Some(bracket_start) = without_colon.rfind('[') {
            if let Some(bracket_end) = without_colon.rfind(']') {
                if bracket_end > bracket_start {
                    let key = without_colon[..bracket_start].trim();
                    let count_str = &without_colon[bracket_start + 1..bracket_end];

                    if let Ok(count) = count_str.parse::<usize>() {
                        return Some((key.to_string(), count));
                    }
                }
            }
        }

        None
    }

    /// Parse section header: [`section_name`]
    fn parse_section_header(&self, line: &str) -> Option<String> {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].trim().to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }
        None
    }

    /// Parse config section with key-value pairs and arrays
    fn parse_config_section(
        &self,
        lines: &[&str],
    ) -> Result<(IndexMap<String, DxLlmValue>, usize), HumanParseError> {
        let mut context = IndexMap::new();
        let mut consumed = 0;
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Stop at next section or empty line followed by section
            if line.starts_with('[') || line.starts_with("# ═") {
                break;
            }

            consumed += 1;
            i += 1;

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check for array syntax: key: or key[n]:
            if line.ends_with(':') && !line.contains('=') {
                let key = line.trim_end_matches(':').trim();

                // Check if it's array with count: key[n]:
                let array_key = if let Some(bracket_pos) = key.rfind('[') {
                    if let Some(bracket_end) = key.rfind(']') {
                        if bracket_end > bracket_pos {
                            key[..bracket_pos].trim().to_string()
                        } else {
                            key.to_string()
                        }
                    } else {
                        key.to_string()
                    }
                } else {
                    key.to_string()
                };

                // Collect array items
                let mut items = Vec::new();
                while i < lines.len() {
                    let item_line = lines[i].trim();
                    if item_line.starts_with("- ") {
                        let item_value = item_line.strip_prefix("- ").unwrap_or("").trim();
                        items.push(self.parse_config_value(item_value)?);
                        consumed += 1;
                        i += 1;
                    } else if item_line.starts_with('-') && item_line.len() > 1 {
                        let item_value = item_line.strip_prefix('-').unwrap_or("").trim();
                        items.push(self.parse_config_value(item_value)?);
                        consumed += 1;
                        i += 1;
                    } else {
                        break;
                    }
                }

                if !items.is_empty() {
                    context.insert(array_key, DxLlmValue::Arr(items));
                }
                continue;
            }

            // Parse key = value
            if let Some((key, value)) = self.parse_key_value(line)? {
                // Keep key as-is (no compression)
                context.insert(key, value);
            }
        }

        Ok((context, consumed))
    }

    /// Parse references section
    fn parse_references_section(
        &self,
        lines: &[&str],
    ) -> Result<(IndexMap<String, String>, usize), HumanParseError> {
        let mut refs = IndexMap::new();
        let mut consumed = 0;

        for line in lines {
            let line = line.trim();

            // Stop at next section
            if line.starts_with('[') || line.starts_with("# ═") {
                break;
            }

            consumed += 1;

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key = "value"
            if let Some((key, DxLlmValue::Str(s))) = self.parse_key_value(line)? {
                refs.insert(key, s);
            }
        }

        Ok((refs, consumed))
    }

    /// Parse key-value pair: key = "value" or key = value
    /// Preserves dots in keys for leaf inlining support
    fn parse_key_value(&self, line: &str) -> Result<Option<(String, DxLlmValue)>, HumanParseError> {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            return Ok(None);
        }

        // Find the = separator
        let eq_pos = match line.find('=') {
            Some(pos) => pos,
            None => return Ok(None),
        };

        let key = line[..eq_pos].trim().to_string();
        let mut value_str = line[eq_pos + 1..].trim();

        // Remove trailing comment (# ref: ...)
        if let Some(comment_pos) = value_str.find("  #") {
            value_str = value_str[..comment_pos].trim();
        }

        let value = self.parse_config_value(value_str)?;
        // Keep key as-is (with dots) - no compression for leaf inlining
        Ok(Some((key, value)))
    }

    /// Parse a config value (string, number, bool, null, array)
    /// V2: Also supports comma-separated arrays without brackets
    fn parse_config_value(&self, s: &str) -> Result<DxLlmValue, HumanParseError> {
        let s = s.trim();

        // Quoted string (JSON-style escaping: \\ → \, \" → ")
        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            let inner = &s[1..s.len() - 1];
            let mut unescaped = String::with_capacity(inner.len());
            let mut escape = false;
            for ch in inner.chars() {
                if escape {
                    match ch {
                        '"' => unescaped.push('"'),
                        '\\' => unescaped.push('\\'),
                        _ => {
                            unescaped.push('\\');
                            unescaped.push(ch);
                        }
                    }
                    escape = false;
                } else if ch == '\\' {
                    escape = true;
                } else {
                    unescaped.push(ch);
                }
            }
            return Ok(DxLlmValue::Str(unescaped));
        }

        // Boolean (support true/false, yes/no, +/-)
        if s == "true" || s == "yes" || s == "+" {
            return Ok(DxLlmValue::Bool(true));
        }
        if s == "false" || s == "no" || s == "-" {
            return Ok(DxLlmValue::Bool(false));
        }

        // Null (support both "null" and "none" for compatibility)
        if s == "null" || s == "none" {
            return Ok(DxLlmValue::Null);
        }

        // Array with brackets — auto-detect separator
        if s.starts_with('[') && s.ends_with(']') {
            let inner = s[1..s.len() - 1].trim();
            if inner.is_empty() {
                return Ok(DxLlmValue::Arr(vec![]));
            }
            let items: Vec<DxLlmValue> = if inner.contains(',') {
                inner
                    .split(',')
                    .map(|item| self.parse_config_value(item.trim()))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                inner
                    .split_whitespace()
                    .map(|item| self.parse_config_value(item))
                    .collect::<Result<Vec<_>, _>>()?
            };
            return Ok(DxLlmValue::Arr(items));
        }

        // Number
        if let Ok(n) = s.parse::<f64>() {
            return Ok(DxLlmValue::Num(n));
        }

        // V2: Comma-separated array without brackets (e.g., "frontend/www, frontend/mobile")
        if s.contains(", ") && !s.starts_with('"') {
            let items: Vec<DxLlmValue> = s
                .split(", ")
                .map(|item| DxLlmValue::Str(item.trim().to_string()))
                .collect();
            if items.len() > 1 {
                return Ok(DxLlmValue::Arr(items));
            }
        }

        // Default to string (unquoted)
        Ok(DxLlmValue::Str(s.to_string()))
    }

    /// Parse data section with table
    #[allow(dead_code)] // Reserved for future table parsing features
    fn parse_data_section(
        &self,
        lines: &[&str],
        section_name: &str,
    ) -> Result<(DxSection, usize), HumanParseError> {
        let mut consumed = 0;
        let mut table_lines: Vec<&str> = Vec::new();
        let mut in_table = false;

        for line in lines {
            let trimmed = line.trim();

            // Stop at next section
            if trimmed.starts_with('[') && !in_table {
                break;
            }

            // Stop at section header
            if trimmed.starts_with("# ═") {
                break;
            }

            consumed += 1;

            // Skip empty lines and summary lines before table
            if trimmed.is_empty() {
                if in_table {
                    // End of table
                    break;
                }
                continue;
            }

            // Skip summary lines
            if trimmed.starts_with("Total:") {
                continue;
            }

            // Detect table start
            if trimmed.starts_with('┌') || trimmed.starts_with('+') || trimmed.starts_with('|') {
                in_table = true;
            }

            if in_table {
                table_lines.push(trimmed);
                // Detect table end
                if trimmed.starts_with('└') || (trimmed.starts_with('+') && table_lines.len() > 2)
                {
                    break;
                }
            }
        }

        let section = self.parse_table(&table_lines, section_name)?;
        Ok((section, consumed))
    }

    /// Parse Unicode or ASCII table
    #[allow(dead_code)] // Reserved for future table parsing features
    fn parse_table(&self, lines: &[&str], context: &str) -> Result<DxSection, HumanParseError> {
        if lines.is_empty() {
            return Ok(DxSection::new(vec![]));
        }

        // Detect table style
        let is_unicode = lines[0].contains('┌') || lines[0].contains('│');
        let is_markdown = lines[0].starts_with('|') && !lines[0].contains('┌');

        let (schema, rows) = if is_unicode {
            self.parse_unicode_table(lines, context)?
        } else if is_markdown {
            self.parse_markdown_table(lines, context)?
        } else {
            self.parse_ascii_table(lines, context)?
        };

        let mut section = DxSection::new(schema);
        for row in rows {
            section.rows.push(row);
        }

        // Security: Check table row count limit
        if section.rows.len() > crate::error::MAX_TABLE_ROWS {
            return Err(HumanParseError::TableTooLarge {
                rows: section.rows.len(),
                max: crate::error::MAX_TABLE_ROWS,
            });
        }

        Ok(section)
    }

    /// Parse Unicode box-drawn table
    /// V2: Also handles wrapped rows with continuation indicators
    #[allow(dead_code)] // Reserved for future table parsing features
    fn parse_unicode_table(
        &self,
        lines: &[&str],
        _context: &str,
    ) -> Result<(Vec<String>, Vec<Vec<DxLlmValue>>), HumanParseError> {
        let mut schema = Vec::new();
        let mut rows = Vec::new();
        let mut header_found = false;
        let mut separator_found = false;
        let mut current_row_cells: Option<Vec<String>> = None;

        for line in lines {
            let line = line.trim();

            // Skip top border and separator lines
            if line.starts_with('┌') || line.starts_with('├') || line.starts_with('└') {
                if line.starts_with('├') {
                    separator_found = true;
                }
                continue;
            }

            // Parse row with │ separators
            if line.starts_with('│') && line.ends_with('│') {
                let cells: Vec<&str> = line[3..line.len() - 3].split('│').map(str::trim).collect();

                if !header_found {
                    // This is the header row - keep column names as-is
                    schema = cells.iter().map(std::string::ToString::to_string).collect();
                    header_found = true;
                } else if separator_found {
                    // Check if this is a continuation row (first cell is empty or has ↓)
                    let is_continuation = cells
                        .first()
                        .is_some_and(|c| c.is_empty() || *c == "↓" || c.trim().is_empty());

                    if is_continuation {
                        if let Some(ref mut current) = current_row_cells {
                            // Append to current row cells
                            for (i, cell) in cells.iter().enumerate() {
                                if i < current.len() && !cell.is_empty() && *cell != "↓" {
                                    if !current[i].is_empty() {
                                        current[i].push(' ');
                                    }
                                    current[i].push_str(cell);
                                }
                            }
                        }
                    } else {
                        // Finalize previous row if exists
                        if let Some(prev_cells) = current_row_cells.take() {
                            let row: Vec<DxLlmValue> = prev_cells
                                .iter()
                                .map(|cell| self.parse_cell_value(cell))
                                .collect();
                            rows.push(row);
                        }
                        // Start new row
                        current_row_cells =
                            Some(cells.iter().map(std::string::ToString::to_string).collect());
                    }
                }
            }
        }

        // Finalize last row
        if let Some(last_cells) = current_row_cells {
            let row: Vec<DxLlmValue> = last_cells
                .iter()
                .map(|cell| self.parse_cell_value(cell))
                .collect();
            rows.push(row);
        }

        Ok((schema, rows))
    }

    /// Parse ASCII table
    #[allow(dead_code)] // Reserved for future table parsing features
    fn parse_ascii_table(
        &self,
        lines: &[&str],
        _context: &str,
    ) -> Result<(Vec<String>, Vec<Vec<DxLlmValue>>), HumanParseError> {
        let mut schema = Vec::new();
        let mut rows = Vec::new();
        let mut header_found = false;
        let mut separator_count = 0;

        for line in lines {
            let line = line.trim();

            // Skip border lines
            if line.starts_with('+') {
                separator_count += 1;
                continue;
            }

            // Parse row with | separators
            if line.starts_with('|') && line.ends_with('|') {
                let cells: Vec<&str> = line[1..line.len() - 1].split('|').map(str::trim).collect();

                if !header_found {
                    schema = cells.iter().map(std::string::ToString::to_string).collect();
                    header_found = true;
                } else if separator_count >= 2 {
                    let row: Vec<DxLlmValue> = cells
                        .iter()
                        .map(|cell| self.parse_cell_value(cell))
                        .collect();
                    rows.push(row);
                }
            }
        }

        Ok((schema, rows))
    }

    /// Parse Markdown table
    #[allow(dead_code)] // Reserved for future table parsing features
    fn parse_markdown_table(
        &self,
        lines: &[&str],
        _context: &str,
    ) -> Result<(Vec<String>, Vec<Vec<DxLlmValue>>), HumanParseError> {
        let mut schema = Vec::new();
        let mut rows = Vec::new();
        let mut header_found = false;
        let mut separator_found = false;

        for line in lines {
            let line = line.trim();

            // Skip separator line (| --- | --- |)
            if line.contains("---") {
                separator_found = true;
                continue;
            }

            // Parse row with | separators
            if line.starts_with('|') && line.ends_with('|') {
                let cells: Vec<&str> = line[1..line.len() - 1].split('|').map(str::trim).collect();

                if !header_found {
                    schema = cells.iter().map(std::string::ToString::to_string).collect();
                    header_found = true;
                } else if separator_found {
                    let row: Vec<DxLlmValue> = cells
                        .iter()
                        .map(|cell| self.parse_cell_value(cell))
                        .collect();
                    rows.push(row);
                }
            }
        }

        Ok((schema, rows))
    }

    /// Parse table cell value
    ///
    /// Recognizes special symbols:
    /// - ✓ → boolean true
    /// - ✗ → boolean false
    /// - — → null
    #[allow(dead_code)] // Reserved for future table parsing features
    fn parse_cell_value(&self, s: &str) -> DxLlmValue {
        let s = s.trim();

        // Boolean true
        if s == "✓" || s == "true" {
            return DxLlmValue::Bool(true);
        }

        // Boolean false
        if s == "✗" || s == "false" {
            return DxLlmValue::Bool(false);
        }

        // Null
        if s == "—" || s == "null" || s == "-" && s.len() == 1 {
            return DxLlmValue::Null;
        }

        // Array
        if s.starts_with('[') && s.ends_with(']') {
            let inner = s[1..s.len() - 1].trim();
            if inner.is_empty() {
                return DxLlmValue::Arr(vec![]);
            }
            let items: Vec<DxLlmValue> = inner
                .split(',')
                .map(|item| self.parse_cell_value(item.trim()))
                .collect();
            return DxLlmValue::Arr(items);
        }

        // Number
        if let Ok(n) = s.parse::<f64>() {
            return DxLlmValue::Num(n);
        }

        // Default to string
        DxLlmValue::Str(s.to_string())
    }

    /// Convert section name to single-character ID
    /// Supports both V1 short names and V2 full names
    #[allow(dead_code)] // Reserved for future serialization features
    fn section_name_to_id(&self, name: &str) -> char {
        match name.to_lowercase().as_str() {
            // V2 full names
            "assets" => 'a',
            "builds" => 'b',
            "config" | "configuration" => 'c',
            "data" => 'd',
            "events" => 'e',
            "forge" => 'f',
            "groups" => 'g',
            "hikes" => 'h',
            "items" => 'i',
            "jobs" => 'j',
            "keys" => 'k',
            "logs" => 'l',
            "metrics" => 'm',
            "nodes" => 'n',
            "orders" => 'o',
            "products" => 'p',
            "queries" => 'q',
            "resources" => 'r',
            "services" => 's',
            "tasks" => 't',
            "users" => 'u',
            "versions" => 'v',
            "workflows" => 'w',
            "extensions" => 'x',
            "yields" => 'y',
            "zones" => 'z',
            // Default: use first character
            _ => name.chars().next().unwrap_or('x').to_ascii_lowercase(),
        }
    }

    /// Parse a table header: name[col1,col2,...](...) or name[col1 col2 ...](...)
    /// Returns (`table_name`, `schema_string`, `remainder_after_bracket_parent`)
    fn parse_table_header(&self, line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim();
        let name_end = trimmed.find('[')?;
        if name_end == 0 {
            return None;
        }
        let name = trimmed[..name_end].trim().to_string();
        if name.is_empty() {
            return None;
        }
        let bracket_end = trimmed[name_end..].find(']')?;
        let schema_str = trimmed[name_end + 1..name_end + bracket_end]
            .trim()
            .to_string();
        if schema_str.is_empty() {
            return None;
        }
        let after_bracket = trimmed[name_end + bracket_end + 1..].trim();

        // Must be followed by '(' to distinguish from array syntax key[n]:
        if !after_bracket.starts_with('(') {
            return None;
        }

        Some((name, schema_str, after_bracket.to_string()))
    }

    /// Parse a parenthesized group: name(key = value ...) or name (key = value ...)
    /// Handles multi-line content inside matching parentheses.
    fn parse_parenthesized_group(
        &self,
        line: &str,
        remaining_lines: &[&str],
    ) -> Option<(String, String)> {
        let trimmed = line.trim();
        let paren_open = trimmed.find('(')?;
        if paren_open == 0 {
            return None;
        }
        // Skip if this is a table (has [...] before (...))
        if trimmed[..paren_open].contains('[') {
            return None;
        }
        let name = trimmed[..paren_open].trim().to_string();
        if name.is_empty() {
            return None;
        }

        let after_open = trimmed[paren_open + 1..].trim();
        let mut depth = 1;
        let mut inner = String::new();

        for ch in after_open.chars() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            inner.push(ch);
        }

        if depth == 0 {
            if !inner.is_empty() {
                return Some((name, inner));
            }
            return Some((name, String::new()));
        }

        // Multi-line: collect remaining lines
        inner.push('\n');
        for rline in remaining_lines.iter().skip(1) {
            let rline = rline.trim();
            for ch in rline.chars() {
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                inner.push(ch);
            }
            if depth == 0 {
                break;
            }
            inner.push('\n');
        }

        if inner.trim().is_empty() {
            Some((name, String::new()))
        } else {
            Some((name, inner))
        }
    }

    /// Parse row cells with auto-detected space vs comma separator
    fn parse_row_cells(&self, row_str: &str, schema: &[String]) -> Vec<DxLlmValue> {
        let row_str = row_str.trim();
        if row_str.is_empty() {
            return vec![];
        }

        // Auto-detect separator: comma if the row contains commas with quoted strings or structured patterns
        let has_commas = row_str.contains(',');
        let _has_quoted_comma = row_str.contains("\",") || row_str.contains(",\"");
        let use_comma = if has_commas {
            // Check if commas look like field separators (followed by space or quote)
            let comma_count = row_str.matches(',').count();
            let space_count = row_str.split_whitespace().count();
            // If comma count is close to schema length, use commas
            if comma_count + 1 == schema.len() || schema.len() > 1 {
                true
            } else {
                comma_count > space_count / 2
            }
        } else {
            false
        };

        if use_comma {
            self.parse_row_cells_comma(row_str, schema)
        } else {
            self.parse_row_cells_space(row_str, schema)
        }
    }

    /// Parse comma-separated row cells
    fn parse_row_cells_comma(&self, row_str: &str, schema: &[String]) -> Vec<DxLlmValue> {
        let mut cells = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for ch in row_str.chars() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                    current.push(ch);
                }
                ',' if !in_quotes => {
                    if let Ok(val) = self.parse_config_value(current.trim()) {
                        cells.push(val);
                    }
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        if !current.is_empty() || cells.len() < schema.len() {
            if let Ok(val) = self.parse_config_value(current.trim()) {
                cells.push(val);
            }
        }

        // Fall back to space parsing if comma parsing produced wrong count
        if cells.len() != schema.len() && schema.len() > 1 {
            return self.parse_row_cells_space(row_str, schema);
        }
        cells
    }

    /// Parse space-separated row cells (handles quoted values)
    fn parse_row_cells_space(&self, row_str: &str, _schema: &[String]) -> Vec<DxLlmValue> {
        let mut cells = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for ch in row_str.chars() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                    current.push(ch);
                }
                ' ' | '\t' if !in_quotes => {
                    if !current.is_empty() {
                        if let Ok(val) = self.parse_config_value(current.trim()) {
                            cells.push(val);
                        }
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        if !current.is_empty() {
            if let Ok(val) = self.parse_config_value(current.trim()) {
                cells.push(val);
            }
        }

        cells
    }
}

impl Default for HumanParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let parser = HumanParser::new();
        let doc = parser.parse("").unwrap();
        assert!(doc.is_empty());
    }

    #[test]
    fn test_parse_section_header() {
        let parser = HumanParser::new();
        assert_eq!(
            parser.parse_section_header("[config]"),
            Some("config".to_string())
        );
        assert_eq!(
            parser.parse_section_header("[data]"),
            Some("data".to_string())
        );
        assert_eq!(parser.parse_section_header("not a header"), None);
    }

    #[test]
    fn test_parse_simple_key_value() {
        let parser = HumanParser::new();
        let input = r"
name = dx
version = 0.0.1
";
        let doc = parser.parse(input).unwrap();
        assert_eq!(doc.context.get("name").unwrap().as_str(), Some("dx"));
        assert_eq!(doc.context.get("version").unwrap().as_str(), Some("0.0.1"));
    }

    #[test]
    fn test_parse_dotted_keys() {
        // Leaf inlining: dots in keys are preserved
        let parser = HumanParser::new();
        let input = r"
forge.repository = https://example.com
style.path = @/style
js.dependencies.react = 19.0.1
";
        let doc = parser.parse(input).unwrap();
        assert_eq!(
            doc.context.get("forge.repository").unwrap().as_str(),
            Some("https://example.com")
        );
        assert_eq!(
            doc.context.get("style.path").unwrap().as_str(),
            Some("@/style")
        );
        assert_eq!(
            doc.context.get("js.dependencies.react").unwrap().as_str(),
            Some("19.0.1")
        );
    }

    #[test]
    fn test_parse_array_with_count() {
        let parser = HumanParser::new();
        let input = r"
editors.items[3]:
- neovim
- zed
- vscode
";
        let doc = parser.parse(input).unwrap();
        let items = doc.context.get("editors.items").unwrap();
        if let DxLlmValue::Arr(arr) = items {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].as_str(), Some("neovim"));
            assert_eq!(arr[1].as_str(), Some("zed"));
            assert_eq!(arr[2].as_str(), Some("vscode"));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_parse_array_header() {
        let parser = HumanParser::new();

        assert_eq!(
            parser.parse_array_header("editors.items[3]:"),
            Some(("editors.items".to_string(), 3))
        );
        assert_eq!(
            parser.parse_array_header("workspace.paths[2]:"),
            Some(("workspace.paths".to_string(), 2))
        );
        assert_eq!(parser.parse_array_header("key:"), None);
        assert_eq!(parser.parse_array_header("key = value"), None);
    }

    #[test]
    fn test_parse_mixed_format() {
        let parser = HumanParser::new();
        let input = r"
name = dx
version = 0.0.1
forge.repository = https://example.com

workspace.paths[2]:
- @/www
- @/backend

editors.default = neovim
";
        let doc = parser.parse(input).unwrap();

        assert_eq!(doc.context.get("name").unwrap().as_str(), Some("dx"));
        assert_eq!(doc.context.get("version").unwrap().as_str(), Some("0.0.1"));
        assert_eq!(
            doc.context.get("forge.repository").unwrap().as_str(),
            Some("https://example.com")
        );
        assert_eq!(
            doc.context.get("editors.default").unwrap().as_str(),
            Some("neovim")
        );

        let paths = doc.context.get("workspace.paths").unwrap();
        if let DxLlmValue::Arr(arr) = paths {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0].as_str(), Some("@/www"));
            assert_eq!(arr[1].as_str(), Some("@/backend"));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_parse_cell_values() {
        let parser = HumanParser::new();

        assert_eq!(parser.parse_cell_value("✓"), DxLlmValue::Bool(true));
        assert_eq!(parser.parse_cell_value("✗"), DxLlmValue::Bool(false));
        assert_eq!(parser.parse_cell_value("—"), DxLlmValue::Null);
        assert_eq!(parser.parse_cell_value("42"), DxLlmValue::Num(42.0));
        assert_eq!(
            parser.parse_cell_value("hello"),
            DxLlmValue::Str("hello".to_string())
        );
    }

    #[test]
    fn test_section_name_to_id() {
        let parser = HumanParser::new();
        assert_eq!(parser.section_name_to_id("data"), 'd');
        assert_eq!(parser.section_name_to_id("forge"), 'f');
        assert_eq!(parser.section_name_to_id("assets"), 'a');
        assert_eq!(parser.section_name_to_id("unknown"), 'u');
    }
}
