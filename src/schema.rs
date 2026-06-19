//! Schema definition and type hints for DX format

use crate::error::{DxError, Result};

/// Type hints for columns (%i, %s, %f, %b, %x, %#)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeHint {
    /// Integer (%i)
    Int,
    /// String (%s)
    String,
    /// Float (%f)
    Float,
    /// Boolean (%b)
    Bool,
    /// Base62 Integer (%x) - DX ∞
    Base62,
    /// Auto-Increment (%#) - DX ∞
    AutoIncrement,
    /// Auto-detect (no hint)
    Auto,
}

impl TypeHint {
    /// Parse type hint from byte (i, s, f, b, x, #)
    pub fn from_byte(b: u8) -> Result<Self> {
        match b {
            b'i' => Ok(Self::Int),
            b's' => Ok(Self::String),
            b'f' => Ok(Self::Float),
            b'b' => Ok(Self::Bool),
            b'x' => Ok(Self::Base62),
            b'#' => Ok(Self::AutoIncrement),
            _ => Err(DxError::InvalidTypeHint(format!(
                "Unknown type hint: {}",
                b as char
            ))),
        }
    }

    /// Convert to byte for encoding
    #[must_use] 
    pub const fn to_byte(self) -> u8 {
        match self {
            Self::Int => b'i',
            Self::String => b's',
            Self::Base62 => b'x',
            Self::AutoIncrement => b'#',
            Self::Float => b'f',
            Self::Bool => b'b',
            Self::Auto => b'a',
        }
    }

    /// Get type name for display
    #[must_use] 
    pub const fn name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::String => "string",
            Self::Base62 => "base62",
            Self::AutoIncrement => "auto-increment",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Auto => "auto",
        }
    }
}

/// Column definition in a schema
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Column name as it appears in the schema.
    pub name: String,
    /// Optional type hint used while parsing values.
    pub type_hint: TypeHint,
}

impl Column {
    /// Create a column definition with an explicit type hint.
    #[must_use] 
    pub const fn new(name: String, type_hint: TypeHint) -> Self {
        Self { name, type_hint }
    }

    /// Check if this is an anonymous auto-increment column (#)
    #[must_use] 
    pub fn is_anonymous_auto_increment(&self) -> bool {
        self.name == "#" && self.type_hint == TypeHint::AutoIncrement
    }
}

/// Schema for a table (defined by `=`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    /// Table or schema name.
    pub name: String,
    /// Ordered column definitions.
    pub columns: Vec<Column>,
}

impl Schema {
    /// Create an empty schema with the provided name.
    #[must_use] 
    pub const fn new(name: String) -> Self {
        Self {
            name,
            columns: Vec::new(),
        }
    }

    /// Create a schema from a prebuilt ordered column list.
    #[must_use] 
    pub const fn with_columns(name: String, columns: Vec<Column>) -> Self {
        Self { name, columns }
    }

    /// Append a column definition to the schema.
    pub fn add_column(&mut self, name: String, type_hint: TypeHint) {
        self.columns.push(Column::new(name, type_hint));
    }

    /// Parse schema definition like: "id%i name%s km%f active%b"
    pub fn parse_definition(name: String, def: &str) -> Result<Self> {
        let mut schema = Self::new(name);
        let parts: Vec<&str> = def.split_whitespace().collect();

        let mut i = 0;
        while i < parts.len() {
            let part = parts[i];

            // Check if this part has a type hint (e.g., "id%i")
            if let Some(pos) = part.find('%') {
                let col_name = &part[..pos];
                let type_char = part.as_bytes().get(pos + 1).ok_or_else(|| {
                    DxError::SchemaError(format!("Missing type after % in '{part}'"))
                })?;
                let type_hint = TypeHint::from_byte(*type_char)?;
                schema.add_column(col_name.to_string(), type_hint);
            } else {
                // No type hint - check next part
                if i + 1 < parts.len() && parts[i + 1].starts_with('%') {
                    let type_str = &parts[i + 1][1..];
                    let type_hint = if type_str.is_empty() {
                        TypeHint::Auto
                    } else {
                        TypeHint::from_byte(type_str.as_bytes()[0])?
                    };
                    schema.add_column(part.to_string(), type_hint);
                    i += 1; // Skip the type hint part
                } else {
                    // No type hint - auto detect
                    schema.add_column(part.to_string(), TypeHint::Auto);
                }
            }
            i += 1;
        }

        if schema.columns.is_empty() {
            return Err(DxError::SchemaError(
                "Schema must have at least one column".to_string(),
            ));
        }

        Ok(schema)
    }

    /// Get column index by name
    #[must_use] 
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }

    /// Get column by index
    #[must_use] 
    pub fn column(&self, idx: usize) -> Option<&Column> {
        self.columns.get(idx)
    }

    /// Number of columns
    #[must_use] 
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    /// Check if schema is empty
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_schema() {
        let schema =
            Schema::parse_definition("users".to_string(), "id%i name%s age%i active%b score%f")
                .unwrap();

        assert_eq!(schema.name, "users");
        assert_eq!(schema.columns.len(), 5);
        assert_eq!(schema.columns[0].name, "id");
        assert_eq!(schema.columns[0].type_hint, TypeHint::Int);
        assert_eq!(schema.columns[1].name, "name");
        assert_eq!(schema.columns[1].type_hint, TypeHint::String);
    }

    #[test]
    fn test_type_hint_roundtrip() {
        let hints = [
            TypeHint::Int,
            TypeHint::String,
            TypeHint::Float,
            TypeHint::Bool,
        ];

        for hint in hints {
            let byte = hint.to_byte();
            let parsed = TypeHint::from_byte(byte).unwrap();
            assert_eq!(hint, parsed);
        }
    }
}
