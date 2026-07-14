//! Encoder for DX Machine format
//!
//! Converts Rust data structures into highly optimized DX bytecode.
//! DX ∞: Base62 encoding for integers, auto-increment detection.

use crate::base62::encode_base62;
use crate::error::Result;
use crate::types::{DxArray, DxObject, DxTable, DxValue};
use std::io::Write;

/// Encoder configuration
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Pretty print (adds spacing)
    pub pretty: bool,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self { pretty: false }
    }
}

/// DX encoder
pub struct Encoder;

impl Encoder {
    /// Encode a value to bytes
    pub fn encode(&mut self, value: &DxValue) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        self.encode_to_writer(value, &mut output)?;
        Ok(output)
    }

    /// Encode to a writer
    pub fn encode_to_writer<W: Write>(&mut self, value: &DxValue, writer: &mut W) -> Result<()> {
        if let DxValue::Object(obj) = value {
            self.encode_value(&DxValue::Object(obj.clone()), writer)
        } else {
            let mut obj = DxObject::new();
            obj.insert("value".to_string(), value.clone());
            self.encode_value(&DxValue::Object(obj), writer)
        }
    }

    fn encode_value<W: Write>(&mut self, value: &DxValue, writer: &mut W) -> Result<()> {
        match value {
            DxValue::Null => write!(writer, "null")?,
            DxValue::Bool(b) => write!(writer, "{b}")?,
            DxValue::Int(i) => write!(writer, "{}", encode_base62((*i) as u64))?,
            DxValue::Float(f) => write!(writer, "{f}")?,
            DxValue::String(s) => write!(writer, "{s}")?,
            DxValue::Array(arr) => self.encode_array(arr, writer)?,
            DxValue::Object(obj) => self.encode_object(obj, writer)?,
            DxValue::Table(table) => self.encode_table(table, writer)?,
            DxValue::Ref(id) => write!(writer, "@{id}")?,
        }
        Ok(())
    }

    fn encode_array<W: Write>(&mut self, arr: &DxArray, writer: &mut W) -> Result<()> {
        write!(writer, "[")?;
        for (i, val) in arr.values.iter().enumerate() {
            if i > 0 {
                write!(writer, " ")?;
            }
            self.encode_value(val, writer)?;
        }
        write!(writer, "]")?;
        Ok(())
    }

    fn encode_object<W: Write>(&mut self, obj: &DxObject, writer: &mut W) -> Result<()> {
        for (key, value) in obj.iter() {
            write!(writer, "{key}=")?;
            self.encode_value(value, writer)?;
            writeln!(writer)?;
        }
        Ok(())
    }

    fn encode_table<W: Write>(&mut self, table: &DxTable, writer: &mut W) -> Result<()> {
        let col_names: Vec<&str> = table.schema.columns.iter().map(|c| c.name.as_str()).collect();
        if col_names.is_empty() {
            write!(writer, "[]")?;
            return Ok(());
        }
        write!(writer, "[{}](", col_names.join(" "))?;
        for row in &table.rows {
            for (i, val) in row.iter().enumerate() {
                if i > 0 { write!(writer, " ")?; }
                self.encode_value(val, writer)?;
            }
            writeln!(writer)?;
        }
        writeln!(writer, ")")?;
        Ok(())
    }
}
