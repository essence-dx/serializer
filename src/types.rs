//! Core data types for DX format
//!
//! This module defines the core value types for the DX **machine format** (binary).
//! The machine format is optimized for zero-copy deserialization and runtime performance.
//!
//! # Format Comparison
//!
//! DX provides two serialization formats:
//!
//! | Format | Type | Use Case | Performance |
//! |--------|------|----------|-------------|
//! | **Machine** | [`DxValue`] | Binary, zero-copy, runtime | Fastest parsing |
//! | **LLM** | [`DxLlmValue`](crate::llm::DxLlmValue) | Text, token-efficient, LLM context | 73%+ token savings |
//!
//! # When to Use This Module
//!
//! Use [`DxValue`] and the machine format when:
//! - You need maximum parsing and serialization performance
//! - You're working with binary data or network protocols
//! - You want zero-copy deserialization for large datasets
//! - You're building runtime data structures
//!
//! Use [`DxLlmValue`](crate::llm::DxLlmValue) when:
//! - You're preparing data for LLM context windows
//! - You need human-readable output
//! - Token efficiency is more important than raw speed
//!
//! # Thread Safety
//!
//! All types in this module implement `Send + Sync` and can be safely shared
//! between threads. See the compile-time assertions at the bottom of this module.

use rustc_hash::FxHashMap;
use std::fmt;

/// The core value type for the DX **machine format** (binary, zero-copy).
///
/// `DxValue` represents all possible values in the DX binary format, optimized
/// for maximum parsing performance and zero-copy deserialization.
///
/// # Relationship to `DxLlmValue`
///
/// DX provides two value types for different use cases:
///
/// - **`DxValue`** (this type): For the binary machine format. Use when you need
///   maximum performance, zero-copy parsing, or runtime data structures.
///
/// - **[`DxLlmValue`](crate::llm::DxLlmValue)**: For the text LLM format. Use when
///   preparing data for LLM context windows or when token efficiency matters.
///
/// The two types have different variant sets because they serve different purposes:
/// - `DxValue` has separate `Int` and `Float` variants for type precision
/// - `DxLlmValue` has a single `Num` variant since LLMs don't distinguish
/// - `DxValue` has `Object` and `Table` for structured data
/// - `DxLlmValue` has `Ref` for reference pointers in the Dx Serializer format
///
/// # When to Use `DxValue`
///
/// Choose `DxValue` when:
/// - **Performance is critical**: Binary format parses faster than text
/// - **Zero-copy is needed**: Large datasets can be memory-mapped
/// - **Type precision matters**: Separate `Int`/`Float` types preserve precision
/// - **Building runtime structures**: Objects and Tables provide efficient access
///
/// # Examples
///
/// ## Creating Values
///
/// ```rust
/// use serializer::DxValue;
///
/// // Primitive values
/// let null = DxValue::Null;
/// let boolean = DxValue::Bool(true);
/// let integer = DxValue::Int(42);
/// let float = DxValue::Float(3.14);
/// let string = DxValue::String("hello".to_string());
/// ```
///
/// ## Working with Arrays
///
/// ```rust
/// use serializer::{DxValue, DxArray};
///
/// let mut arr = DxArray::new();
/// arr.values.push(DxValue::Int(1));
/// arr.values.push(DxValue::Int(2));
/// arr.values.push(DxValue::Int(3));
///
/// let array_value = DxValue::Array(arr);
/// ```
///
/// ## Working with Objects
///
/// ```rust
/// use serializer::{DxValue, DxObject};
///
/// let mut obj = DxObject::new();
/// obj.insert("name".to_string(), DxValue::String("Alice".to_string()));
/// obj.insert("age".to_string(), DxValue::Int(30));
///
/// let object_value = DxValue::Object(obj);
/// ```
///
/// ## Type Inspection
///
/// ```rust
/// use serializer::DxValue;
///
/// let value = DxValue::Int(42);
/// assert_eq!(value.type_name(), "int");
/// assert_eq!(value.as_int(), Some(42));
/// ```
///
/// # Thread Safety
///
/// `DxValue` implements `Send + Sync` and can be safely shared between threads.
/// This is verified at compile time via static assertions.
///
/// # See Also
///
/// - [`DxLlmValue`](crate::llm::DxLlmValue) - Value type for the LLM text format
/// - [`DxArray`] - Array container type
/// - [`DxObject`] - Object/map container type
/// - [`DxTable`] - Schema-defined table type
#[derive(Debug, Clone, PartialEq)]
pub enum DxValue {
    /// Null value, represented as `~` in DX format.
    Null,
    /// Boolean value, represented as `+` (true) or `-` (false) in DX format.
    Bool(bool),
    /// 64-bit signed integer.
    ///
    /// Note: [`DxLlmValue`](crate::llm::DxLlmValue) uses a single `Num` variant
    /// for all numbers since LLMs don't distinguish integer vs float.
    Int(i64),
    /// 64-bit floating-point number.
    ///
    /// Note: [`DxLlmValue`](crate::llm::DxLlmValue) uses a single `Num` variant
    /// for all numbers since LLMs don't distinguish integer vs float.
    Float(f64),
    /// String value. In machine format, strings don't require quotes.
    String(String),
    /// Array/List of values. See [`DxArray`] for details.
    Array(DxArray),
    /// Object/Map with key-value pairs. See [`DxObject`] for details.
    Object(DxObject),
    /// Table with schema-defined columns. See [`DxTable`] for details.
    ///
    /// Tables are a DX-specific feature for efficient tabular data representation.
    Table(DxTable),
    /// Reference to an anchor by index (`@N` in DX format).
    ///
    /// Anchors allow deduplication of repeated values in the document.
    Ref(usize),
}

/// A DX array (inline or vertical)
#[derive(Debug, Clone, PartialEq)]
pub struct DxArray {
    /// Values stored in insertion order.
    pub values: Vec<DxValue>,
    /// Whether this was a stream (>)
    pub is_stream: bool,
}

impl DxArray {
    /// Create an empty array value.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            values: Vec::new(),
            is_stream: false,
        }
    }

    /// Create an empty array value with preallocated capacity.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            values: Vec::with_capacity(cap),
            is_stream: false,
        }
    }

    /// Create a stream-style array from existing values.
    #[must_use]
    pub const fn stream(values: Vec<DxValue>) -> Self {
        Self {
            values,
            is_stream: true,
        }
    }
}

impl Default for DxArray {
    fn default() -> Self {
        Self::new()
    }
}

/// A DX object (key-value pairs)
#[derive(Debug, Clone, PartialEq)]
pub struct DxObject {
    /// Ordered key-value pairs
    fields: Vec<(String, DxValue)>,
    /// Fast lookup map (key index)
    lookup: FxHashMap<String, usize>,
}

impl DxObject {
    /// Create an empty ordered object.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            lookup: FxHashMap::default(),
        }
    }

    /// Create an empty ordered object with preallocated field capacity.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            fields: Vec::with_capacity(cap),
            lookup: FxHashMap::with_capacity_and_hasher(cap, Default::default()),
        }
    }

    /// Insert or replace a field while preserving insertion order.
    pub fn insert(&mut self, key: String, value: DxValue) {
        if let Some(&idx) = self.lookup.get(&key) {
            self.fields[idx].1 = value;
        } else {
            let idx = self.fields.len();
            self.fields.push((key.clone(), value));
            self.lookup.insert(key, idx);
        }
    }

    /// Get a field value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&DxValue> {
        self.lookup.get(key).map(|&idx| &self.fields[idx].1)
    }

    /// Iterate fields in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &(String, DxValue)> {
        self.fields.iter()
    }

    /// Get all fields as a slice.
    #[must_use]
    pub fn fields(&self) -> &[(String, DxValue)] {
        &self.fields
    }

    /// Check if object is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

impl Default for DxObject {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DxObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, (k, v)) in self.fields.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{k}: {v:?}")?;
        }
        write!(f, "}}")
    }
}

/// A table with schema-defined columns
#[derive(Debug, Clone, PartialEq)]
pub struct DxTable {
    /// Schema that defines column names and type hints.
    pub schema: crate::schema::Schema,
    /// Table rows, each matching the schema column count.
    pub rows: Vec<Vec<DxValue>>,
}

impl DxTable {
    /// Create an empty table for the provided schema.
    #[must_use]
    pub const fn new(schema: crate::schema::Schema) -> Self {
        Self {
            schema,
            rows: Vec::new(),
        }
    }

    /// Add a row after validating its width against the schema.
    pub fn add_row(&mut self, row: Vec<DxValue>) -> Result<(), String> {
        if row.len() != self.schema.columns.len() {
            return Err(format!(
                "Row length {} doesn't match schema length {}",
                row.len(),
                self.schema.columns.len()
            ));
        }
        self.rows.push(row);
        Ok(())
    }

    /// Return the number of rows currently stored in the table.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Return the number of columns declared by the table schema.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.schema.columns.len()
    }
}

impl DxValue {
    /// Check if this value is "empty" for ditto logic
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Get type name for error messages
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
            Self::Table(_) => "table",
            Self::Ref(_) => "ref",
        }
    }

    /// Convert to boolean if possible
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Convert to integer if possible
    #[must_use]
    pub const fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            Self::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// Convert to float if possible
    #[must_use]
    pub const fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Convert to string if possible
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

// =============================================================================
// Thread Safety Compile-Time Assertions
// =============================================================================

// These static assertions verify at compile time that our types are thread-safe.
// If any of these types stop implementing Send or Sync, compilation will fail.

/// Compile-time assertion that a type implements Send
const fn _assert_send<T: Send>() {}

/// Compile-time assertion that a type implements Sync
const fn _assert_sync<T: Sync>() {}

// Verify DxValue is Send + Sync
const _: () = _assert_send::<DxValue>();
const _: () = _assert_sync::<DxValue>();

// Verify DxArray is Send + Sync
const _: () = _assert_send::<DxArray>();
const _: () = _assert_sync::<DxArray>();

// Verify DxObject is Send + Sync
const _: () = _assert_send::<DxObject>();
const _: () = _assert_sync::<DxObject>();

// Verify DxTable is Send + Sync
const _: () = _assert_send::<DxTable>();
const _: () = _assert_sync::<DxTable>();
