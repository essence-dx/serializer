//! DX Serializer LLM Format Module
//!
//! This module provides support for multiple interconvertible formats:
//!
//! ## DX Serializer Format (Token-Efficient LLM Format)
//!
//! A token-efficient serialization format optimized for LLMs:
//! - `key=value` for simple key-value pairs
//! - `name(key=val key2=val2)` for objects (parentheses, space-separated)
//! - `name[col1 col2 col3](rows)` for tables (wrapped dataframe format)
//! - `key=[item1 item2 item3]` for arrays (square brackets, space-separated)
//! - `true`/`false` for booleans, `null` for null values
//! - Use quotes `"..."` for multi-word strings
//!
//! ## Human Format (Clean TOML-like)
//!
//! Clean, hand-editable format:
//! - `=` for single scalar values
//! - `:` + `-` lines for arrays
//! - `#` for comments
//! - Nested sections like `[stack.js]`
//!
//! ## Machine Format
//!
//! Binary format for runtime using RKYV's zero-copy architecture.
//!
//! The architecture follows a "hub and spoke" model where all formats convert through
//! a common internal representation (`DxDocument`), ensuring consistent round-trip behavior.

pub mod cache_generator;
pub mod config_formatter;
pub mod formatter;
pub mod convert;

pub mod machine_zerocopy;
pub mod parser;
pub mod pretty_printer;
pub mod section_names;
pub mod serializer;
pub mod serializer_output;
pub mod table_wrapper;
pub mod tokens;
pub mod types;

#[cfg(test)]
mod convert_props;
#[cfg(test)]
mod llm_props;

// Re-export main types
pub use cache_generator::{CacheConfig, CacheError, CacheGenerator, CachePaths, CacheResult};
#[cfg(feature = "mmap")]
pub use convert::machine_file_to_document_mmap;
pub use convert::{
    ConvertError, MachineFormat, document_to_compact, document_to_formatted_llm, document_to_human,
    document_to_llm, document_to_llm_with_config, document_to_loose, document_to_machine,
    human_to_document, human_to_llm,
    human_to_machine, human_to_machine_uncompressed, is_llm_format, llm_to_document, llm_to_human,
    llm_to_machine, machine_bytes_to_document, machine_to_document, machine_to_human,
    machine_to_llm, try_document_to_machine_with_compression, try_read_machine_or_sr,
};
pub use config_formatter::{ConfigFormatter, ConfigStyle};
pub use formatter::LlmFormatter;

pub use machine_zerocopy::{ZeroCopyDocument, ZeroCopyError, ZeroCopyMachine};
pub use parser::{LlmParser, ParseError};
pub use pretty_printer::{PrettyPrintError, PrettyPrinter, PrettyPrinterConfig};
pub use section_names::SectionNameDict;
pub use serializer::{LlmSerializer, SerializerConfig};
pub use serializer_output::{
    SerializerOutput, SerializerOutputConfig, SerializerOutputError, SerializerPaths,
    SerializerResult,
};
pub use table_wrapper::{TableWrapper, TableWrapperConfig};
pub use tokens::{ModelType, TokenCounter, TokenInfo};
pub use types::{DxDocument, DxLlmValue, DxSection};
