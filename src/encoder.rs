//! Encoder for DX Machine format
//!
//! Converts Rust data structures into highly optimized DX bytecode.
//! DX ∞: Base62 encoding for integers, auto-increment detection.

use crate::base62::encode_base62;
use crate::error::{DxError, MAX_RECURSION_DEPTH, Result};
use crate::types::{DxArray, DxObject, DxTable, DxValue};
use std::io::Write;

/// Encoder configuration
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct EncoderConfig {
    /// Pretty print (adds spacing)
    pub pretty: bool,
}


/// DX encoder
pub struct Encoder;

impl Encoder {
    /// Encode a value to bytes
    pub fn encode(&self, value: &DxValue) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        self.encode_to_writer(value, &mut output)?;
        Ok(output)
    }

    /// Encode to a writer
    pub fn encode_to_writer<W: Write>(&self, value: &DxValue, writer: &mut W) -> Result<()> {
        if let DxValue::Object(obj) = value {
            self.encode_value(&DxValue::Object(obj.clone()), writer, 0)
        } else {
            let mut obj = DxObject::new();
            obj.insert("value".to_string(), value.clone());
            self.encode_value(&DxValue::Object(obj), writer, 0)
        }
    }

    fn encode_value<W: Write>(&self, value: &DxValue, writer: &mut W, depth: usize) -> Result<()> {
        if depth > MAX_RECURSION_DEPTH {
            return Err(DxError::RecursionLimitExceeded {
                depth,
                max: MAX_RECURSION_DEPTH,
            });
        }
        match value {
            DxValue::Null => write!(writer, "null")?,
            DxValue::Bool(b) => write!(writer, "{b}")?,
            DxValue::Int(i) => {
                if *i < 0 {
                    write!(writer, "-{}", encode_base62((-*i) as u64))?;
                } else {
                    write!(writer, "{}", encode_base62(*i as u64))?;
                }
            }
            DxValue::Float(f) => write!(writer, "{f}")?,
            DxValue::String(s) => {
                if s.contains(' ')
                    || s.contains('=')
                    || s.contains(':')
                    || s.contains('"')
                    || s.contains('\n')
                    || s.contains('(')
                    || s.contains(')')
                {
                    write!(
                        writer,
                        "\"{}\"",
                        s.replace('\\', "\\\\")
                            .replace('"', "\\\"")
                            .replace('\n', "\\n")
                    )?;
                } else {
                    write!(writer, "{s}")?;
                }
            }
            DxValue::Array(arr) => self.encode_array(arr, writer, depth)?,
            DxValue::Object(obj) => self.encode_object(obj, writer, depth)?,
            DxValue::Table(table) => self.encode_table(table, writer, depth)?,
            DxValue::Ref(id) => write!(writer, "@{id}")?,
        }
        Ok(())
    }

    fn encode_array<W: Write>(&self, arr: &DxArray, writer: &mut W, depth: usize) -> Result<()> {
        write!(writer, "[")?;
        for (i, val) in arr.values.iter().enumerate() {
            if i > 0 {
                write!(writer, " ")?;
            }
            self.encode_value(val, writer, depth + 1)?;
        }
        write!(writer, "]")?;
        Ok(())
    }

    fn encode_object<W: Write>(&self, obj: &DxObject, writer: &mut W, depth: usize) -> Result<()> {
        for (key, value) in obj.iter() {
            write!(writer, "{key}=")?;
            self.encode_value(value, writer, depth + 1)?;
            writeln!(writer)?;
        }
        Ok(())
    }

    fn encode_table<W: Write>(&self, table: &DxTable, writer: &mut W, depth: usize) -> Result<()> {
        let col_names: Vec<&str> = table
            .schema
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        if col_names.is_empty() {
            write!(writer, "[]")?;
            return Ok(());
        }
        write!(writer, "[{}](", col_names.join(" "))?;
        for row in &table.rows {
            for (i, val) in row.iter().enumerate() {
                if i > 0 {
                    write!(writer, " ")?;
                }
                self.encode_value(val, writer, depth + 1)?;
            }
            writeln!(writer)?;
        }
        writeln!(writer, ")")?;
        Ok(())
    }
}
