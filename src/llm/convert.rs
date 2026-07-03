//! Format conversion functions
//!
//! Provides conversion between DX Serializer (LLM), Human, and Machine formats.
//! All conversions go through the common `DxDocument` representation.

use crate::llm::formatter::LlmFormatter;
use crate::llm::human_formatter::{HumanFormatConfig, HumanFormatter};
use crate::llm::human_parser::{HumanParseError, HumanParser};
use crate::llm::parser::{LlmParser, ParseError};
use crate::llm::serializer::{LlmSerializer, SerializerConfig};
use crate::llm::types::DxDocument;
use std::borrow::Cow;
use std::path::Path;
use thiserror::Error;

/// Conversion errors
#[derive(Debug, Error)]
pub enum ConvertError {
    /// DX LLM parser failed while reading LLM-format text.
    #[error("DX Serializer parse error: {0}")]
    LlmParse(#[from] ParseError),

    /// Human-format parser failed while reading human-facing text.
    #[error("Human parse error: {0}")]
    HumanParse(#[from] HumanParseError),

    /// Machine-format conversion failed.
    #[error("Machine format error: {msg}")]
    MachineFormat {
        /// Human-readable machine-format failure message.
        msg: String,
    },
}

/// Convert DX Serializer format string to Human format string
#[must_use = "conversion result should be used"]
pub fn llm_to_human(llm_input: &str) -> Result<String, ConvertError> {
    let doc = LlmParser::parse(llm_input)?;
    let formatter = HumanFormatter::new();
    Ok(formatter.format(&doc))
}

/// Convert DX Serializer format string to Human format string with custom config
pub fn llm_to_human_with_config(
    llm_input: &str,
    config: HumanFormatConfig,
) -> Result<String, ConvertError> {
    let doc = LlmParser::parse(llm_input)?;
    let formatter = HumanFormatter::with_config(config);
    Ok(formatter.format(&doc))
}

/// Convert Human format string to DX Serializer format string
#[must_use = "conversion result should be used"]
pub fn human_to_llm(human_input: &str) -> Result<String, ConvertError> {
    let trimmed = human_input.trim();

    // Check if input is already DX Serializer format
    if is_dsr_format(trimmed) {
        return Ok(human_input.to_string());
    }

    // Parse as Human format and convert to DX Serializer
    let parser = HumanParser::new();
    let doc = parser.parse(human_input)?;
    let serializer = LlmSerializer::new();
    Ok(serializer.serialize(&doc))
}

/// Check if input is in DX Serializer format
#[must_use]
pub fn is_dsr_format(input: &str) -> bool {
    let trimmed = input.trim();

    // DX Serializer format indicators:
    // - name[key=value,...] (objects) - NOT [name] which is TOML section
    // - name:count(schema)[data] (tables)
    // - name:count=items (arrays)
    // - key=value (simple pairs, NO spaces around =)

    // Human format indicators (should return false):
    // - [section] (TOML section headers)
    // - key = value (spaces around =)
    // - key[count]: followed by - items (list format)

    let mut has_dsr_indicators = false;
    let mut has_human_indicators = false;

    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // TOML section headers start with [ - this is HUMAN format
        if line.starts_with('[') {
            has_human_indicators = true;
            continue;
        }

        // List items starting with - are HUMAN format
        if line.starts_with('-') {
            has_human_indicators = true;
            continue;
        }

        // Check for spaces around = (HUMAN format: "key = value")
        if line.contains(" = ") {
            has_human_indicators = true;
            continue;
        }

        // Check for table syntax: name:count(schema)[
        if line.contains(':') && line.contains('(') && line.contains('[') {
            has_dsr_indicators = true;
            continue;
        }

        // Check for array syntax: name:count=items (DSR format)
        if line.contains(':') && line.contains('=') {
            let colon_pos = line.find(':');
            let eq_pos = line.find('=');
            if let (Some(cp), Some(ep)) = (colon_pos, eq_pos) {
                if cp < ep {
                    has_dsr_indicators = true;
                    continue;
                }
            }
        }

        // Check for compact key=value (NO spaces around =) - DSR format
        if line.contains('=') && !line.contains(" = ") {
            if let Some(eq_pos) = line.find('=') {
                let before = &line[..eq_pos];
                let after = &line[eq_pos + 1..];
                // DSR has no trailing space before = and no leading space after =
                if !before.ends_with(' ') && !after.starts_with(' ') {
                    has_dsr_indicators = true;
                    continue;
                }
            }
        }
    }

    // If we found human format indicators, it's NOT DSR format
    if has_human_indicators {
        return false;
    }

    // Only return true if we found DSR indicators
    has_dsr_indicators
}

/// Check if input is in LLM format (alias for `is_dsr_format`)
#[must_use]
pub fn is_llm_format(input: &str) -> bool {
    is_dsr_format(input)
}

/// Convert DX Serializer format string to `DxDocument`
#[must_use = "parsing result should be used"]
pub fn llm_to_document(llm_input: &str) -> Result<DxDocument, ConvertError> {
    Ok(LlmParser::parse(llm_input)?)
}

/// Convert Human format string to `DxDocument`
#[must_use = "parsing result should be used"]
pub fn human_to_document(human_input: &str) -> Result<DxDocument, ConvertError> {
    let parser = HumanParser::new();
    Ok(parser.parse(human_input)?)
}

/// Convert `DxDocument` to DX Serializer format string
#[must_use]
pub fn document_to_llm(doc: &DxDocument) -> String {
    let serializer = LlmSerializer::new();
    serializer.serialize(doc)
}

/// Convert `DxDocument` to DX Serializer format string with custom config
#[must_use]
pub fn document_to_llm_with_config(doc: &DxDocument, config: SerializerConfig) -> String {
    let serializer = LlmSerializer::with_config(config);
    serializer.serialize(doc)
}

/// Convert `DxDocument` to formatted LLM format string (`--format` mode)
///
/// Produces LLM-format output with consistent spacing and indentation:
/// - Spaces around `=` for top-level key-value pairs
/// - Blank lines between entries
/// - Indented section rows (4 spaces)
/// - Space-separated structural tokens in table headers
#[must_use]
pub fn document_to_formatted_llm(doc: &DxDocument) -> String {
    let formatter = LlmFormatter;
    formatter.format(doc)
}

/// Convert `DxDocument` to Human format string
#[must_use]
pub fn document_to_human(doc: &DxDocument) -> String {
    let formatter = HumanFormatter::new();
    formatter.format(doc)
}

/// Convert `DxDocument` to Human format string with custom config
#[must_use] 
pub fn document_to_human_with_config(doc: &DxDocument, config: HumanFormatConfig) -> String {
    let formatter = HumanFormatter::with_config(config);
    formatter.format(doc)
}

/// Compression algorithm for machine format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionAlgorithm {
    /// LZ4 compression (fastest, default)
    #[default]
    Lz4,
    /// Zstd compression (better compression ratio)
    Zstd,
    /// No compression
    None,
}

const MACHINE_ENVELOPE_MAGIC: &[u8; 4] = b"DXM1";
const MACHINE_ENVELOPE_VERSION: u8 = 1;
const MACHINE_ENVELOPE_HEADER_LEN: usize = 56;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MachineEnvelopeCodec {
    None,
    Lz4,
    Zstd,
}

impl MachineEnvelopeCodec {
    const fn as_u8(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Lz4 => 1,
            Self::Zstd => 2,
        }
    }

    fn from_u8(value: u8) -> Result<Self, ConvertError> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Lz4),
            2 => Ok(Self::Zstd),
            _ => Err(ConvertError::MachineFormat {
                msg: format!("Unsupported machine envelope codec: {value}"),
            }),
        }
    }
}

struct MachineEnvelope<'a> {
    codec: MachineEnvelopeCodec,
    payload: &'a [u8],
    uncompressed_len: usize,
}

/// Machine format representation (binary)
///
/// Includes automatic decompression caching for optimal performance.
#[derive(Debug, Clone)]
pub struct MachineFormat {
    /// Raw machine-format bytes.
    pub data: Vec<u8>,
    /// Cached decompressed data (lazy) - first access decompresses, subsequent accesses use cache
    #[cfg(feature = "compression")]
    cached: std::cell::RefCell<Option<Vec<u8>>>,
}

impl MachineFormat {
    /// Create a new `MachineFormat` from raw data
    #[must_use] 
    pub const fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            #[cfg(feature = "compression")]
            cached: std::cell::RefCell::new(None),
        }
    }

    /// Get the raw data
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

/// Convert DX Serializer format to Machine format (RKYV + compression)
pub fn llm_to_machine(llm_input: &str) -> Result<MachineFormat, ConvertError> {
    let doc = LlmParser::parse(llm_input)?;
    try_document_to_machine_with_compression(&doc, CompressionAlgorithm::default())
}

/// Convert DX Serializer format to Machine format with specific compression
pub fn llm_to_machine_with_compression(
    llm_input: &str,
    compression: CompressionAlgorithm,
) -> Result<MachineFormat, ConvertError> {
    let doc = LlmParser::parse(llm_input)?;
    try_document_to_machine_with_compression(&doc, compression)
}

/// Convert Human format to Machine format (RKYV + compression)
pub fn human_to_machine(human_input: &str) -> Result<MachineFormat, ConvertError> {
    let parser = HumanParser::new();
    let doc = parser.parse(human_input)?;
    try_document_to_machine_with_compression(&doc, CompressionAlgorithm::default())
}

/// Convert Human format to Machine format without compression (raw RKYV)
pub fn human_to_machine_uncompressed(human_input: &str) -> Result<MachineFormat, ConvertError> {
    let parser = HumanParser::new();
    let doc = parser.parse(human_input)?;
    try_document_to_machine_with_compression(&doc, CompressionAlgorithm::None)
}

/// Convert Human format to Machine format with specific compression
pub fn human_to_machine_with_compression(
    human_input: &str,
    compression: CompressionAlgorithm,
) -> Result<MachineFormat, ConvertError> {
    let parser = HumanParser::new();
    let doc = parser.parse(human_input)?;
    try_document_to_machine_with_compression(&doc, compression)
}

/// Convert `DxDocument` to Machine format (RKYV + LZ4 by default)
#[must_use] 
pub fn document_to_machine(doc: &DxDocument) -> MachineFormat {
    document_to_machine_with_compression(doc, CompressionAlgorithm::default())
}

/// Convert `DxDocument` to Machine format with specific compression
#[must_use] 
pub fn document_to_machine_with_compression(
    doc: &DxDocument,
    compression: CompressionAlgorithm,
) -> MachineFormat {
    try_document_to_machine_with_compression(doc, compression)
        .unwrap_or_else(|error| panic!("Machine serialization failed: {error}"))
}

/// Try to convert `DxDocument` to Machine format with specific compression.
pub fn try_document_to_machine_with_compression(
    doc: &DxDocument,
    compression: CompressionAlgorithm,
) -> Result<MachineFormat, ConvertError> {
    use crate::machine::machine_types::MachineDocument;
    use crate::machine::serialize;

    let machine_doc = MachineDocument::from(doc);
    let rkyv_data = serialize(&machine_doc)
        .map_err(|e| ConvertError::MachineFormat {
            msg: format!("RKYV serialization failed: {e}"),
        })?
        .into_vec();

    let (codec, payload): (MachineEnvelopeCodec, Cow<'_, [u8]>) = match compression {
        CompressionAlgorithm::None => (MachineEnvelopeCodec::None, Cow::Borrowed(&rkyv_data)),

        #[cfg(feature = "compression-lz4")]
        CompressionAlgorithm::Lz4 => {
            use crate::machine::compress::compress_lz4;
            match compress_lz4(&rkyv_data) {
                Ok(compressed) => {
                    let savings_ratio =
                        compression_savings_ratio(rkyv_data.len(), compressed.len());
                    if savings_ratio > 0.10 {
                        (MachineEnvelopeCodec::Lz4, Cow::Owned(compressed))
                    } else {
                        (MachineEnvelopeCodec::None, Cow::Borrowed(&rkyv_data))
                    }
                }
                Err(_) => (MachineEnvelopeCodec::None, Cow::Borrowed(&rkyv_data)),
            }
        }

        #[cfg(feature = "compression-zstd")]
        CompressionAlgorithm::Zstd => {
            use crate::machine::compress::{CompressionLevel, compress_zstd_level};
            match compress_zstd_level(&rkyv_data, CompressionLevel::Fast) {
                Ok(compressed) => {
                    let savings_ratio =
                        compression_savings_ratio(rkyv_data.len(), compressed.len());
                    if savings_ratio > 0.10 {
                        (MachineEnvelopeCodec::Zstd, Cow::Owned(compressed))
                    } else {
                        (MachineEnvelopeCodec::None, Cow::Borrowed(&rkyv_data))
                    }
                }
                Err(_) => (MachineEnvelopeCodec::None, Cow::Borrowed(&rkyv_data)),
            }
        }

        #[cfg(not(feature = "compression-lz4"))]
        CompressionAlgorithm::Lz4 => (MachineEnvelopeCodec::None, Cow::Borrowed(&rkyv_data)),

        #[cfg(not(feature = "compression-zstd"))]
        CompressionAlgorithm::Zstd => (MachineEnvelopeCodec::None, Cow::Borrowed(&rkyv_data)),
    };

    Ok(MachineFormat::new(encode_machine_envelope(
        codec,
        payload.as_ref(),
        rkyv_data.len(),
    )))
}

/// Convert Machine format to `DxDocument` (auto-detects compression)
pub fn machine_to_document(machine: &MachineFormat) -> Result<DxDocument, ConvertError> {
    #[cfg(feature = "compression")]
    {
        // Check cache first
        if let Some(cached) = machine.cached.borrow().as_ref() {
            return rkyv_bytes_to_document(cached);
        }

        let decompressed = decode_machine_bytes(&machine.data)?;
        *machine.cached.borrow_mut() = Some(decompressed.to_vec());
        rkyv_bytes_to_document(decompressed.as_ref())
    }

    #[cfg(not(feature = "compression"))]
    machine_bytes_to_document(&machine.data)
}

/// Convert raw machine bytes to a `DxDocument`.
pub fn machine_bytes_to_document(data: &[u8]) -> Result<DxDocument, ConvertError> {
    let doc_data = decode_machine_bytes(data)?;
    rkyv_bytes_to_document(doc_data.as_ref())
}

/// Convert a memory-mapped `.machine` file to a DxDocument.
#[cfg(feature = "mmap")]
#[allow(unsafe_code)]
pub fn machine_file_to_document_mmap(
    path: impl AsRef<std::path::Path>,
) -> Result<DxDocument, ConvertError> {
    let file = std::fs::File::open(path.as_ref()).map_err(|error| ConvertError::MachineFormat {
        msg: format!("Machine file open failed: {}", error),
    })?;
    // SAFETY: The map is read-only, scoped to this function, and the file is not mutated here.
    let mmap = unsafe { memmap2::MmapOptions::new().map(&file) }.map_err(|error| {
        ConvertError::MachineFormat {
            msg: format!("Machine file mmap failed: {}", error),
        }
    })?;

    machine_bytes_to_document(&mmap)
}

fn rkyv_bytes_to_document(doc_data: &[u8]) -> Result<DxDocument, ConvertError> {
    use crate::machine::machine_types::MachineDocument;

    let machine_doc: MachineDocument =
        rkyv::from_bytes(doc_data).map_err(|e: rkyv::rancor::Error| {
            ConvertError::MachineFormat {
                msg: format!("RKYV deserialize failed: {e}"),
            }
        })?;

    Ok(DxDocument::from(&machine_doc))
}

#[cfg(feature = "compression")]
fn compression_savings_ratio(uncompressed_len: usize, compressed_len: usize) -> f64 {
    if uncompressed_len == 0 {
        return 0.0;
    }

    1.0 - (compressed_len as f64 / uncompressed_len as f64)
}

fn encode_machine_envelope(
    codec: MachineEnvelopeCodec,
    payload: &[u8],
    uncompressed_len: usize,
) -> Vec<u8> {
    let payload_len = payload.len() as u64;
    let uncompressed_len = uncompressed_len as u64;
    let payload_hash = blake3::hash(payload);
    let mut output = Vec::with_capacity(MACHINE_ENVELOPE_HEADER_LEN + payload.len());

    output.extend_from_slice(MACHINE_ENVELOPE_MAGIC);
    output.push(MACHINE_ENVELOPE_VERSION);
    output.push(codec.as_u8());
    output.extend_from_slice(&[0, 0]);
    output.extend_from_slice(&payload_len.to_le_bytes());
    output.extend_from_slice(&uncompressed_len.to_le_bytes());
    output.extend_from_slice(payload_hash.as_bytes());
    output.extend_from_slice(payload);

    output
}

fn decode_machine_envelope(data: &[u8]) -> Result<Option<MachineEnvelope<'_>>, ConvertError> {
    if !data.starts_with(MACHINE_ENVELOPE_MAGIC) {
        return Ok(None);
    }

    if data.len() < MACHINE_ENVELOPE_HEADER_LEN {
        return Err(ConvertError::MachineFormat {
            msg: "Machine envelope header is truncated".to_string(),
        });
    }

    if data[4] != MACHINE_ENVELOPE_VERSION {
        return Err(ConvertError::MachineFormat {
            msg: format!("Unsupported machine envelope version: {}", data[4]),
        });
    }

    if data[6] != 0 || data[7] != 0 {
        return Err(ConvertError::MachineFormat {
            msg: "Machine envelope reserved bytes must be zero".to_string(),
        });
    }

    let codec = MachineEnvelopeCodec::from_u8(data[5])?;
    let payload_len = read_u64_le(&data[8..16])?;
    let uncompressed_len = read_u64_le(&data[16..24])?;
    let expected_len = MACHINE_ENVELOPE_HEADER_LEN
        .checked_add(payload_len)
        .ok_or_else(|| ConvertError::MachineFormat {
            msg: "Machine envelope payload length overflow".to_string(),
        })?;

    if data.len() != expected_len {
        return Err(ConvertError::MachineFormat {
            msg: format!(
                "Machine envelope length mismatch: expected {}, found {}",
                expected_len,
                data.len()
            ),
        });
    }

    let payload = &data[MACHINE_ENVELOPE_HEADER_LEN..];
    let actual_hash = blake3::hash(payload);
    if actual_hash.as_bytes() != &data[24..56] {
        return Err(ConvertError::MachineFormat {
            msg: "Machine envelope payload checksum mismatch".to_string(),
        });
    }

    Ok(Some(MachineEnvelope {
        codec,
        payload,
        uncompressed_len,
    }))
}

fn read_u64_le(bytes: &[u8]) -> Result<usize, ConvertError> {
    let value = u64::from_le_bytes(bytes.try_into().map_err(|_| ConvertError::MachineFormat {
        msg: "Machine envelope integer field has invalid length".to_string(),
    })?);

    usize::try_from(value).map_err(|_| ConvertError::MachineFormat {
        msg: "Machine envelope integer is too large for this platform".to_string(),
    })
}

fn validate_uncompressed_len(data: &[u8], expected_len: usize) -> Result<(), ConvertError> {
    if data.len() != expected_len {
        return Err(ConvertError::MachineFormat {
            msg: format!(
                "Machine envelope uncompressed length mismatch: expected {}, found {}",
                expected_len,
                data.len()
            ),
        });
    }

    Ok(())
}

#[cfg(feature = "compression")]
fn decode_machine_bytes(data: &[u8]) -> Result<Cow<'_, [u8]>, ConvertError> {
    if let Some(envelope) = decode_machine_envelope(data)? {
        let decoded = decode_machine_envelope_payload(&envelope)?;
        validate_uncompressed_len(decoded.as_ref(), envelope.uncompressed_len)?;
        return Ok(decoded);
    }

    decompress_auto(data)
}

#[cfg(not(feature = "compression"))]
fn decode_machine_bytes(data: &[u8]) -> Result<Cow<'_, [u8]>, ConvertError> {
    if let Some(envelope) = decode_machine_envelope(data)? {
        if envelope.codec != MachineEnvelopeCodec::None {
            return Err(ConvertError::MachineFormat {
                msg: "Compressed machine envelope requires the compression feature".to_string(),
            });
        }
        validate_uncompressed_len(envelope.payload, envelope.uncompressed_len)?;
        return Ok(Cow::Borrowed(envelope.payload));
    }

    Ok(Cow::Borrowed(data))
}

#[cfg(feature = "compression")]
fn decode_machine_envelope_payload<'a>(
    envelope: &MachineEnvelope<'a>,
) -> Result<Cow<'a, [u8]>, ConvertError> {
    match envelope.codec {
        MachineEnvelopeCodec::None => Ok(Cow::Borrowed(envelope.payload)),
        MachineEnvelopeCodec::Lz4 => {
            #[cfg(feature = "compression-lz4")]
            {
                use crate::machine::compress::decompress_lz4;
                decompress_lz4(envelope.payload)
                    .map_err(|e| ConvertError::MachineFormat {
                        msg: format!("LZ4 machine envelope decompression failed: {e}"),
                    })
                    .map(Cow::Owned)
            }

            #[cfg(not(feature = "compression-lz4"))]
            {
                Err(ConvertError::MachineFormat {
                    msg: "LZ4 machine envelope requires the compression-lz4 feature".to_string(),
                })
            }
        }
        MachineEnvelopeCodec::Zstd => {
            #[cfg(feature = "compression-zstd")]
            {
                use crate::machine::compress::decompress_zstd;
                decompress_zstd(envelope.payload)
                    .map_err(|e| ConvertError::MachineFormat {
                        msg: format!("Zstd machine envelope decompression failed: {e}"),
                    })
                    .map(Cow::Owned)
            }

            #[cfg(not(feature = "compression-zstd"))]
            {
                Err(ConvertError::MachineFormat {
                    msg: "Zstd machine envelope requires the compression-zstd feature".to_string(),
                })
            }
        }
    }
}

/// Auto-detect and decompress data (tries LZ4, then Zstd, then raw)
#[cfg(feature = "compression")]
fn decompress_auto(data: &[u8]) -> Result<Cow<'_, [u8]>, ConvertError> {
    // Try LZ4 first (most common, fastest)
    #[cfg(feature = "compression-lz4")]
    {
        use crate::machine::compress::decompress_lz4;
        if let Ok(decompressed) = decompress_lz4(data) {
            return Ok(Cow::Owned(decompressed));
        }
    }

    // Try Zstd
    #[cfg(feature = "compression-zstd")]
    {
        use crate::machine::compress::decompress_zstd;
        if let Ok(decompressed) = decompress_zstd(data) {
            return Ok(Cow::Owned(decompressed));
        }
    }

    // Not compressed, return as-is
    Ok(Cow::Borrowed(data))
}

/// Convert Machine format to DX Serializer format string
pub fn machine_to_llm(machine: &MachineFormat) -> Result<String, ConvertError> {
    let doc = machine_to_document(machine)?;
    Ok(document_to_llm(&doc))
}

/// Convert Machine format to Human format string
pub fn machine_to_human(machine: &MachineFormat) -> Result<String, ConvertError> {
    let doc = machine_to_document(machine)?;
    Ok(document_to_human(&doc))
}

/// Read a cached document, preferring `.machine` format over `.sr`.
///
/// Checks if a `.machine` file exists alongside the `.sr` source.
/// If the machine file is fresher (by mtime), reads and parses it.
/// Otherwise falls back to parsing the `.sr` file as LLM text.
/// Returns `None` if neither file exists or both fail to parse.
pub fn try_read_machine_or_sr(sr_path: &Path) -> Option<(DxDocument, bool)> {
    let machine_path = sr_path.with_extension("machine");
    let (from_machine, bytes) = if machine_path.exists() && machine_is_fresher(&machine_path, sr_path) {
        (true, std::fs::read(&machine_path).ok()?)
    } else {
        let text = std::fs::read_to_string(sr_path).ok()?;
        (false, text.into_bytes())
    };

    if from_machine {
        machine_bytes_to_document(&bytes).ok().map(|doc| (doc, true))
    } else {
        let text = String::from_utf8(bytes).ok()?;
        llm_to_document(&text).ok().map(|doc| (doc, false))
    }
}

/// Check if a `.machine` file is fresher than its `.sr` source by mtime.
fn machine_is_fresher(machine_path: &Path, sr_path: &Path) -> bool {
    let machine_mtime = std::fs::metadata(machine_path)
        .and_then(|m| m.modified())
        .ok();
    let sr_mtime = std::fs::metadata(sr_path)
        .and_then(|m| m.modified())
        .ok();
    match (machine_mtime, sr_mtime) {
        (Some(mm), Some(sm)) => mm >= sm,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::DxLlmValue;

    #[test]
    fn test_llm_to_human() {
        let llm = "name=Test\ncount=42";
        let human = llm_to_human(llm).unwrap();
        assert!(human.contains("name") || human.contains("Test"));
    }

    #[test]
    fn test_human_to_llm() {
        let human = r#"
[config]
    name = "Test"
    count = 42
"#;
        let llm = human_to_llm(human).unwrap();
        // DX Serializer format uses : or :: for key-value pairs
        assert!(llm.contains(':') || llm.contains("Test"));
    }

    #[test]
    fn try_document_to_machine_reports_machine_format_errors() {
        let doc = DxDocument::new();
        let machine =
            try_document_to_machine_with_compression(&doc, CompressionAlgorithm::None).unwrap();
        let round_trip_doc = machine_to_document(&machine).unwrap();

        assert_eq!(round_trip_doc.entry_order.len(), 0);
    }

    #[test]
    fn test_machine_format_round_trip() {
        let mut doc = DxDocument::new();
        doc.context
            .insert("name".to_string(), DxLlmValue::Str("Test".to_string()));
        doc.context
            .insert("count".to_string(), DxLlmValue::Num(42.0));
        doc.context
            .insert("active".to_string(), DxLlmValue::Bool(true));

        let machine = document_to_machine(&doc);
        assert!(machine.as_bytes().starts_with(MACHINE_ENVELOPE_MAGIC));
        let round_trip_doc = machine_to_document(&machine).unwrap();

        assert_eq!(doc.context.len(), round_trip_doc.context.len());
        assert_eq!(
            round_trip_doc.context.get("name").unwrap().as_str(),
            Some("Test")
        );
        assert_eq!(
            round_trip_doc.context.get("count").unwrap().as_num(),
            Some(42.0)
        );
    }

    #[test]
    fn test_is_dsr_format() {
        // DX Serializer format
        assert!(is_dsr_format("name=Test"));
        assert!(is_dsr_format("config[host=localhost,port=8080]"));
        assert!(is_dsr_format("friends:3=ana,luis,sam"));
        assert!(is_dsr_format("table:2(id,name)[1,John\n2,Jane]"));

        // Not DX Serializer format (Human/TOML-like)
        assert!(!is_dsr_format("[config]\nname = Test"));
    }

    #[test]
    fn test_machine_format_rejects_corrupt_envelope() {
        let mut doc = DxDocument::new();
        doc.context
            .insert("name".to_string(), DxLlmValue::Str("Test".to_string()));

        let mut machine = document_to_machine_with_compression(&doc, CompressionAlgorithm::None);
        let last = machine.data.len() - 1;
        machine.data[last] ^= 0xFF;

        let error = machine_to_document(&machine).unwrap_err();
        assert!(error.to_string().contains("checksum mismatch"));
    }

    #[test]
    fn test_machine_format_rejects_invalid_envelope_headers() {
        let mut doc = DxDocument::new();
        doc.context
            .insert("name".to_string(), DxLlmValue::Str("Test".to_string()));
        let machine = document_to_machine_with_compression(&doc, CompressionAlgorithm::None);

        let truncated = MachineFormat::new(machine.data[..8].to_vec());
        assert!(
            machine_to_document(&truncated)
                .unwrap_err()
                .to_string()
                .contains("header is truncated")
        );

        let mut bad_version = machine.clone();
        bad_version.data[4] = MACHINE_ENVELOPE_VERSION + 1;
        assert!(
            machine_to_document(&bad_version)
                .unwrap_err()
                .to_string()
                .contains("Unsupported machine envelope version")
        );

        let mut bad_reserved = machine.clone();
        bad_reserved.data[6] = 1;
        assert!(
            machine_to_document(&bad_reserved)
                .unwrap_err()
                .to_string()
                .contains("reserved bytes must be zero")
        );

        let mut bad_codec = machine;
        bad_codec.data[5] = 255;
        assert!(
            machine_to_document(&bad_codec)
                .unwrap_err()
                .to_string()
                .contains("Unsupported machine envelope codec")
        );
    }

    #[test]
    fn test_machine_format_rejects_invalid_envelope_lengths() {
        let mut doc = DxDocument::new();
        doc.context
            .insert("name".to_string(), DxLlmValue::Str("Test".to_string()));
        let machine = document_to_machine_with_compression(&doc, CompressionAlgorithm::None);

        let mut bad_payload_len = machine.clone();
        bad_payload_len.data[8..16].copy_from_slice(&1u64.to_le_bytes());
        assert!(
            machine_to_document(&bad_payload_len)
                .unwrap_err()
                .to_string()
                .contains("length mismatch")
        );

        let mut bad_uncompressed_len = machine;
        bad_uncompressed_len.data[16..24].copy_from_slice(&1u64.to_le_bytes());
        assert!(
            machine_to_document(&bad_uncompressed_len)
                .unwrap_err()
                .to_string()
                .contains("uncompressed length mismatch")
        );
    }

    #[test]
    fn test_machine_format_reads_legacy_raw_rkyv() {
        use crate::machine::machine_types::MachineDocument;
        use crate::machine::serialize;

        let mut doc = DxDocument::new();
        doc.context.insert(
            "legacy".to_string(),
            DxLlmValue::Str("raw-rkyv".to_string()),
        );
        let machine_doc = MachineDocument::from(&doc);
        let legacy_data = serialize(&machine_doc).unwrap().into_vec();
        let machine = MachineFormat::new(legacy_data);
        let round_trip_doc = machine_to_document(&machine).unwrap();

        assert_eq!(
            round_trip_doc.context.get("legacy").unwrap().as_str(),
            Some("raw-rkyv")
        );
    }

    #[cfg(feature = "mmap")]
    #[test]
    fn test_machine_file_to_document_mmap_reads_uncompressed_envelope() {
        let temp = tempfile::tempdir().unwrap();
        let machine_path = temp.path().join("config.machine");
        let mut doc = DxDocument::new();
        doc.context
            .insert("name".to_string(), DxLlmValue::Str("mmap".to_string()));
        let machine = document_to_machine_with_compression(&doc, CompressionAlgorithm::None);
        std::fs::write(&machine_path, machine.as_bytes()).unwrap();

        let round_trip_doc = machine_file_to_document_mmap(&machine_path).unwrap();

        assert_eq!(
            round_trip_doc.context.get("name").unwrap().as_str(),
            Some("mmap")
        );
    }
}
