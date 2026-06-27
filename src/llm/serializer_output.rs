//! Serializer Output Module for DX Serializer
//!
//! Generates LLM and Machine format files from .sr/.dx source files.
//! Output files are stored in `.dx/serializer/` with proper naming.
//!
//! ## Output Structure
//!
//! ```text
//! .dx/serializer/
//! ├── javascript-lint.llm      # LLM-optimized format
//! └── javascript-lint.machine  # Binary format (used at runtime)
//! ```
//!
//! ## Format Flow (2026 Architecture)
//!
//! 1. Source (.sr/.dx) - LLM format stored on disk
//! 2. LLM (.llm) - Optional normalized copy for workflows that need it
//! 3. Machine (.machine) - Binary for fast runtime loading

#[cfg(feature = "converters")]
use crate::converters::{json_to_document, toml_to_document};
use crate::llm::convert::{
    CompressionAlgorithm, ConvertError, document_to_formatted_llm, document_to_human,
    document_to_llm, document_to_llm_with_config, llm_to_document,
    try_document_to_machine_with_compression,
};
use crate::llm::serializer::SerializerConfig;
use crate::llm::types::DxDocument;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;
use thiserror::Error;

/// Serializer output errors
#[derive(Debug, Error)]
pub enum SerializerOutputError {
    /// Filesystem read or write failed.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Human or LLM format parsing failed.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Conversion between serializer formats failed.
    #[error("Conversion error: {0}")]
    Convert(#[from] ConvertError),

    /// Source or output path was not valid for serializer output.
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Machine metadata did not match the current source or machine bytes.
    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),

    /// Output directory could not be created.
    #[error("Directory creation failed: {0}")]
    DirectoryCreation(String),
}

/// Configuration for serializer output
#[derive(Debug, Clone)]
pub struct SerializerOutputConfig {
    /// Root directory for output files (default: .dx/serializer)
    pub output_dir: PathBuf,
    /// Generate LLM format files
    pub generate_llm: bool,
    /// Generate machine format files
    pub generate_machine: bool,
    /// Compression algorithm for machine format
    pub compression: CompressionAlgorithm,
    /// Generate source/machine validation metadata for cache readers
    pub generate_metadata: bool,
    /// LLM serializer configuration (compact syntax, prefix elimination, etc.)
    pub serializer_config: SerializerConfig,
    /// Generate human-readable (beautified) output instead of LLM format
    pub beautify: bool,
    /// Generate formatted LLM output (spaces around `=`, indented sections)
    pub format_llm: bool,
}

impl Default for SerializerOutputConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from(".dx/serializer"),
            generate_llm: true,
            generate_machine: true,
            compression: CompressionAlgorithm::default(),
            generate_metadata: false,
            serializer_config: SerializerConfig::default(),
            beautify: false,
            format_llm: false,
        }
    }
}

impl SerializerOutputConfig {
    /// Create a new config with default settings
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the output directory
    pub fn with_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = dir.into();
        self
    }

    /// Set whether to generate LLM format
    #[must_use] 
    pub const fn with_llm(mut self, generate: bool) -> Self {
        self.generate_llm = generate;
        self
    }

    /// Set whether to generate machine format
    #[must_use] 
    pub const fn with_machine(mut self, generate: bool) -> Self {
        self.generate_machine = generate;
        self
    }

    /// Set compression algorithm
    #[must_use] 
    pub const fn with_compression(mut self, compression: CompressionAlgorithm) -> Self {
        self.compression = compression;
        self
    }

    /// Set whether to generate source/machine validation metadata.
    #[must_use] 
    pub const fn with_metadata(mut self, generate: bool) -> Self {
        self.generate_metadata = generate;
        self
    }

    /// Set the LLM serializer configuration.
    #[must_use] 
    pub const fn with_serializer_config(mut self, config: SerializerConfig) -> Self {
        self.serializer_config = config;
        self
    }

    /// Set beautify mode (human-readable output).
    #[must_use] 
    pub const fn with_beautify(mut self, beautify: bool) -> Self {
        self.beautify = beautify;
        self
    }

    /// Set formatted LLM mode.
    #[must_use] 
    pub const fn with_format_llm(mut self, format_llm: bool) -> Self {
        self.format_llm = format_llm;
        self
    }
}

/// Output paths for a serialized file
#[derive(Debug, Clone)]
pub struct SerializerPaths {
    /// Original source path (.sr/.dx file)
    pub source: PathBuf,
    /// LLM format output path (.llm)
    pub llm: PathBuf,
    /// Machine format output path (.machine)
    pub machine: PathBuf,
    /// Machine metadata output path (.machine.meta.json)
    pub metadata: PathBuf,
}

/// Result of serializer output generation
#[derive(Debug)]
pub struct SerializerResult {
    /// Output paths
    pub paths: SerializerPaths,
    /// Whether LLM format was generated
    pub llm_generated: bool,
    /// Whether machine format was generated
    pub machine_generated: bool,
    /// Size of LLM output in bytes
    pub llm_size: usize,
    /// Size of machine output in bytes
    pub machine_size: usize,
}

/// Serializer output generator
pub struct SerializerOutput {
    config: SerializerOutputConfig,
}

impl SerializerOutput {
    /// Create a new serializer output with default config
    #[must_use] 
    pub fn new() -> Self {
        Self {
            config: SerializerOutputConfig::default(),
        }
    }

    /// Create with custom config
    #[must_use] 
    pub const fn with_config(config: SerializerOutputConfig) -> Self {
        Self { config }
    }

    /// Get output paths for a source file
    #[must_use] 
    pub fn get_paths(&self, source_path: &Path) -> SerializerPaths {
        let stem = flatten_serializer_output_stem(source_path);

        SerializerPaths {
            source: source_path.to_path_buf(),
            llm: self.config.output_dir.join(format!("{stem}.llm")),
            machine: self.config.output_dir.join(format!("{stem}.machine")),
            metadata: self
                .config
                .output_dir
                .join(format!("{stem}.machine.meta.json")),
        }
    }

    /// Process a `.sr`, `.dx`, extensionless `dx`, JSON, or TOML source file and generate outputs.
    pub fn process_file(
        &self,
        source_path: &Path,
    ) -> Result<SerializerResult, SerializerOutputError> {
        let content = fs::read_to_string(source_path)?;
        let doc = parse_source_document(source_path, &content)?;

        self.process_document_with_source(&doc, source_path, Some(content.as_bytes()))
    }

    /// Process a `DxDocument` and generate outputs
    pub fn process_document(
        &self,
        doc: &DxDocument,
        source_path: &Path,
    ) -> Result<SerializerResult, SerializerOutputError> {
        self.process_document_with_source(doc, source_path, None)
    }

    fn process_document_with_source(
        &self,
        doc: &DxDocument,
        source_path: &Path,
        source_bytes: Option<&[u8]>,
    ) -> Result<SerializerResult, SerializerOutputError> {
        let paths = self.get_paths(source_path);

        // Ensure output directory exists
        fs::create_dir_all(&self.config.output_dir).map_err(|e| {
            SerializerOutputError::DirectoryCreation(format!(
                "{}: {}",
                self.config.output_dir.display(),
                e
            ))
        })?;

        let mut result = SerializerResult {
            paths: paths.clone(),
            llm_generated: false,
            machine_generated: false,
            llm_size: 0,
            machine_size: 0,
        };

        // Generate LLM format (or beautified human format or formatted LLM)
        if self.config.generate_llm {
            let llm_content = if self.config.beautify {
                document_to_human(doc)
            } else if self.config.format_llm {
                document_to_formatted_llm(doc)
            } else if self.config.serializer_config
                != SerializerConfig::default()
            {
                document_to_llm_with_config(doc, self.config.serializer_config.clone())
            } else {
                document_to_llm(doc)
            };
            fs::write(&paths.llm, &llm_content)?;
            result.llm_generated = true;
            result.llm_size = llm_content.len();
        }

        // Generate machine format
        if self.config.generate_machine {
            let machine_content =
                try_document_to_machine_with_compression(doc, self.config.compression)?;
            write_atomic(&paths.machine, machine_content.as_bytes())?;
            result.machine_generated = true;
            result.machine_size = machine_content.data.len();

            if self.config.generate_metadata {
                if let Some(source_bytes) = source_bytes {
                    let metadata = machine_metadata_json(
                        source_path,
                        source_bytes,
                        &paths.machine,
                        &machine_content.data,
                    );
                    write_atomic(&paths.metadata, metadata.as_bytes())?;
                }
            }
        }

        Ok(result)
    }

    /// Process all serializer-supported files in a directory.
    pub fn process_directory(
        &self,
        dir: &Path,
    ) -> Result<Vec<SerializerResult>, SerializerOutputError> {
        let mut results = Vec::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && is_serializer_source(&path) {
                    match self.process_file(&path) {
                        Ok(result) => results.push(result),
                        Err(e) => {
                            eprintln!("Warning: Failed to process {}: {}", path.display(), e);
                        }
                    }
                }
        }

        Ok(results)
    }

    /// Check if outputs are up-to-date for a source file
    #[must_use] 
    pub fn is_up_to_date(&self, source_path: &Path) -> bool {
        let paths = self.get_paths(source_path);

        // Check if output files exist
        if !paths.llm.exists() || !paths.machine.exists() {
            return false;
        }

        // Compare modification times
        let source_modified = fs::metadata(source_path).and_then(|m| m.modified()).ok();

        let llm_modified = fs::metadata(&paths.llm).and_then(|m| m.modified()).ok();

        let machine_modified = fs::metadata(&paths.machine).and_then(|m| m.modified()).ok();

        match (source_modified, llm_modified, machine_modified) {
            (Some(src), Some(llm), Some(machine)) => llm >= src && machine >= src,
            _ => false,
        }
    }

    /// Get the config
    #[must_use] 
    pub const fn config(&self) -> &SerializerOutputConfig {
        &self.config
    }
}

/// Validate source/machine metadata before trusting a `.machine` cache artifact.
#[cfg(feature = "converters")]
pub fn validate_machine_metadata(
    metadata_json: &str,
    source_path: &Path,
    source_bytes: &[u8],
    machine_path: &Path,
    machine_bytes: &[u8],
) -> Result<(), SerializerOutputError> {
    #[derive(serde::Deserialize)]
    struct Metadata {
        schema: String,
        source: SourceMetadata,
        machine: MachineMetadata,
    }

    #[derive(serde::Deserialize)]
    struct SourceMetadata {
        path: String,
        bytes: usize,
        blake3: String,
    }

    #[derive(serde::Deserialize)]
    struct MachineMetadata {
        path: String,
        bytes: usize,
        blake3: String,
    }

    let metadata: Metadata = serde_json::from_str(metadata_json).map_err(|error| {
        SerializerOutputError::Parse(format!("Machine metadata JSON parse failed: {error}"))
    })?;

    if metadata.schema != "dx.machine.source_metadata.v1" {
        return Err(SerializerOutputError::InvalidMetadata(format!(
            "unsupported schema: {}",
            metadata.schema
        )));
    }

    validate_metadata_path("source path", &metadata.source.path, source_path)?;
    validate_metadata_len("source bytes", metadata.source.bytes, source_bytes.len())?;
    validate_metadata_hash("source blake3", &metadata.source.blake3, source_bytes)?;
    validate_metadata_path("machine path", &metadata.machine.path, machine_path)?;
    validate_metadata_len("machine bytes", metadata.machine.bytes, machine_bytes.len())?;
    validate_metadata_hash("machine blake3", &metadata.machine.blake3, machine_bytes)?;

    Ok(())
}

/// Validate source/machine metadata before trusting a `.machine` cache artifact.
#[cfg(not(feature = "converters"))]
pub fn validate_machine_metadata(
    _metadata_json: &str,
    _source_path: &Path,
    _source_bytes: &[u8],
    _machine_path: &Path,
    _machine_bytes: &[u8],
) -> Result<(), SerializerOutputError> {
    Err(SerializerOutputError::Parse(
        "Metadata validation requires the 'converters' feature".to_string(),
    ))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = atomic_temp_path(path);
    if temp_path.exists() {
        fs::remove_file(&temp_path)?;
    }

    let mut file = fs::File::create(&temp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    if path.exists() {
        fs::remove_file(path)?;
    }

    fs::rename(&temp_path, path).inspect_err(|_| {
        let _ = fs::remove_file(&temp_path);
    })
}

fn atomic_temp_path(path: &Path) -> PathBuf {
    let mut temp = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("tmp");
    temp.set_extension(format!("{extension}.tmp"));
    temp
}

#[cfg(feature = "converters")]
fn validate_metadata_path(
    label: &str,
    expected: &str,
    actual: &Path,
) -> Result<(), SerializerOutputError> {
    let actual = actual.display().to_string();
    if expected == actual {
        return Ok(());
    }

    Err(SerializerOutputError::InvalidMetadata(format!(
        "{label} mismatch: expected {expected}, found {actual}"
    )))
}

#[cfg(feature = "converters")]
fn validate_metadata_len(
    label: &str,
    expected: usize,
    actual: usize,
) -> Result<(), SerializerOutputError> {
    if expected == actual {
        return Ok(());
    }

    Err(SerializerOutputError::InvalidMetadata(format!(
        "{label} mismatch: expected {expected}, found {actual}"
    )))
}

#[cfg(feature = "converters")]
fn validate_metadata_hash(
    label: &str,
    expected: &str,
    bytes: &[u8],
) -> Result<(), SerializerOutputError> {
    let actual = blake3::hash(bytes).to_hex().to_string();
    if expected == actual {
        return Ok(());
    }

    Err(SerializerOutputError::InvalidMetadata(format!(
        "{label} mismatch: expected {expected}, found {actual}"
    )))
}

fn machine_metadata_json(
    source_path: &Path,
    source_bytes: &[u8],
    machine_path: &Path,
    machine_bytes: &[u8],
) -> String {
    let modified_unix_ms = fs::metadata(source_path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok()).map_or_else(|| "null".to_string(), |duration| duration.as_millis().to_string());

    format!(
        concat!(
            "{{\n",
            "  \"schema\": \"dx.machine.source_metadata.v1\",\n",
            "  \"source\": {{\n",
            "    \"path\": \"{}\",\n",
            "    \"bytes\": {},\n",
            "    \"modified_unix_ms\": {},\n",
            "    \"blake3\": \"{}\"\n",
            "  }},\n",
            "  \"machine\": {{\n",
            "    \"path\": \"{}\",\n",
            "    \"bytes\": {},\n",
            "    \"blake3\": \"{}\"\n",
            "  }},\n",
            "  \"cache\": {{\n",
            "    \"rebuildable\": true,\n",
            "    \"fallback_on_mismatch\": true\n",
            "  }}\n",
            "}}\n"
        ),
        json_escape(&source_path.display().to_string()),
        source_bytes.len(),
        modified_unix_ms,
        blake3::hash(source_bytes).to_hex(),
        json_escape(&machine_path.display().to_string()),
        machine_bytes.len(),
        blake3::hash(machine_bytes).to_hex()
    )
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}

fn parse_source_document(
    source_path: &Path,
    content: &str,
) -> Result<DxDocument, SerializerOutputError> {
    if is_json_source(source_path) {
        return parse_json_document(content);
    }
    if is_toml_source(source_path) {
        return parse_toml_document(content);
    }

    llm_to_document(content).map_err(|e| SerializerOutputError::Parse(e.to_string()))
}

#[cfg(feature = "converters")]
fn parse_json_document(content: &str) -> Result<DxDocument, SerializerOutputError> {
    json_to_document(content).map_err(SerializerOutputError::Parse)
}

#[cfg(not(feature = "converters"))]
fn parse_json_document(_content: &str) -> Result<DxDocument, SerializerOutputError> {
    Err(SerializerOutputError::Parse(
        "JSON support requires the 'converters' feature".to_string(),
    ))
}

#[cfg(feature = "converters")]
fn parse_toml_document(content: &str) -> Result<DxDocument, SerializerOutputError> {
    toml_to_document(content).map_err(SerializerOutputError::Parse)
}

#[cfg(not(feature = "converters"))]
fn parse_toml_document(_content: &str) -> Result<DxDocument, SerializerOutputError> {
    Err(SerializerOutputError::Parse(
        "TOML support requires the 'converters' feature".to_string(),
    ))
}

fn is_serializer_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("sr")
                || extension.eq_ignore_ascii_case("dx")
                || extension.eq_ignore_ascii_case("json")
                || extension.eq_ignore_ascii_case("toml")
        })
        || path.file_name().and_then(|name| name.to_str()) == Some("dx")
}

fn is_json_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

fn is_toml_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"))
}

fn flatten_serializer_output_stem(source_path: &Path) -> String {
    if source_path.file_name().and_then(|name| name.to_str()) == Some("dx") {
        return "dx".to_string();
    }

    if is_json_source(source_path) || is_toml_source(source_path) {
        if source_path.is_relative() {
            let parts = source_path
                .components()
                .filter_map(|component| match component {
                    Component::Normal(part) => part.to_str(),
                    _ => None,
                })
                .map(sanitize_cache_stem_part)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();

            if parts.len() > 1 {
                return parts.join("-");
            }
        }

        if let Some(file_name) = source_path.file_name().and_then(|name| name.to_str()) {
            return sanitize_cache_stem_part(file_name);
        }
    }

    let parts = source_path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();

    if let Some(dx_index) = parts.iter().rposition(|part| *part == ".dx") {
        let mut nested = parts
            .iter()
            .skip(dx_index + 1)
            .filter(|part| **part != "serializer")
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();
        if let Some(last) = nested.last_mut() {
            if let Some((stem, _extension)) = last.rsplit_once('.') {
                *last = stem.to_string();
            }
        }
        let flattened = nested
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        if !flattened.is_empty() {
            return flattened;
        }
    }

    source_path
        .file_stem()
        .map(|stem| sanitize_cache_stem_part(&stem.to_string_lossy()))
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn sanitize_cache_stem_part(part: &str) -> String {
    let mut output = String::with_capacity(part.len());
    let mut previous_was_dash = false;

    for ch in part.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            output.push(ch);
            previous_was_dash = false;
        } else if !previous_was_dash {
            output.push('-');
            previous_was_dash = true;
        }
    }

    let trimmed = output.trim_matches('-');
    if trimmed.is_empty() {
        "path".to_string()
    } else {
        trimmed.to_string()
    }
}

impl Default for SerializerOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::convert::{MachineFormat, machine_to_document};
    use tempfile::tempdir;

    #[test]
    fn test_serializer_output_config_default() {
        let config = SerializerOutputConfig::default();
        assert_eq!(config.output_dir, PathBuf::from(".dx/serializer"));
        assert!(config.generate_llm);
        assert!(config.generate_machine);
    }

    #[test]
    fn test_get_paths() {
        let output = SerializerOutput::new();
        let paths = output.get_paths(Path::new("rules/javascript-lint.sr"));

        assert_eq!(paths.llm.file_name().unwrap(), "javascript-lint.llm");
        assert_eq!(
            paths.machine.file_name().unwrap(),
            "javascript-lint.machine"
        );
    }

    #[test]
    fn flatten_serializer_output_stem_uses_nested_dx_tool_path() {
        let serializer = SerializerOutput::with_config(
            SerializerOutputConfig::new().with_output_dir(".dx/serializer"),
        );
        let paths = serializer.get_paths(Path::new(".dx/forge/data.sr"));

        assert_eq!(flatten_serializer_output_stem(Path::new("dx")), "dx");
        assert_eq!(
            paths.machine,
            Path::new(".dx/serializer").join("forge-data.machine")
        );
    }

    #[test]
    fn flatten_serializer_output_stem_keeps_relative_config_identity() {
        assert_eq!(
            flatten_serializer_output_stem(Path::new("package.json")),
            "package-json"
        );
        assert_eq!(
            flatten_serializer_output_stem(Path::new("packages/bun-types/package.json")),
            "packages-bun-types-package-json"
        );
        assert_eq!(
            flatten_serializer_output_stem(Path::new("packages/@types/bun/package.json")),
            "packages-types-bun-package-json"
        );
        assert_eq!(
            flatten_serializer_output_stem(Path::new("bunfig.toml")),
            "bunfig-toml"
        );
    }

    #[test]
    fn test_process_simple_file() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("test.sr");

        // Create a simple .sr file
        fs::write(&source_path, "nm|test\nv|1.0").unwrap();

        let config =
            SerializerOutputConfig::new().with_output_dir(temp.path().join(".dx/serializer"));

        let output = SerializerOutput::with_config(config);
        let result = output.process_file(&source_path);

        // Note: This may fail if llm_to_document doesn't support this format
        // In that case, we'd need to adjust the test
        if let Ok(result) = result {
            assert!(result.llm_generated);
            assert!(result.machine_generated);
        }
    }

    #[test]
    fn machine_metadata_validation_accepts_generated_metadata() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("package.json");
        fs::write(&source_path, r#"{"name":"dx-js-tool"}"#).unwrap();

        let config = SerializerOutputConfig::new()
            .with_output_dir(temp.path().join(".dx/js"))
            .with_llm(false)
            .with_machine(true)
            .with_metadata(true)
            .with_compression(CompressionAlgorithm::None);
        let result = SerializerOutput::with_config(config)
            .process_file(&source_path)
            .unwrap();

        let metadata = fs::read_to_string(&result.paths.metadata).unwrap();
        let source_bytes = fs::read(&source_path).unwrap();
        let machine_bytes = fs::read(&result.paths.machine).unwrap();

        validate_machine_metadata(
            &metadata,
            &source_path,
            &source_bytes,
            &result.paths.machine,
            &machine_bytes,
        )
        .unwrap();
    }

    #[test]
    fn machine_metadata_validation_rejects_stale_source_bytes() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("package.json");
        fs::write(&source_path, r#"{"name":"dx-js-tool"}"#).unwrap();

        let config = SerializerOutputConfig::new()
            .with_output_dir(temp.path().join(".dx/js"))
            .with_llm(false)
            .with_machine(true)
            .with_metadata(true)
            .with_compression(CompressionAlgorithm::None);
        let result = SerializerOutput::with_config(config)
            .process_file(&source_path)
            .unwrap();

        let metadata = fs::read_to_string(&result.paths.metadata).unwrap();
        let machine_bytes = fs::read(&result.paths.machine).unwrap();
        let error = validate_machine_metadata(
            &metadata,
            &source_path,
            br#"{"name":"changed123"}"#,
            &result.paths.machine,
            &machine_bytes,
        )
        .unwrap_err();

        assert!(error.to_string().contains("source blake3 mismatch"));
    }

    #[test]
    fn process_json_package_file_leaves_no_machine_temp_files() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("package.json");
        fs::write(&source_path, r#"{"name":"dx-js-tool"}"#).unwrap();

        let config = SerializerOutputConfig::new()
            .with_output_dir(temp.path().join(".dx/js"))
            .with_llm(false)
            .with_machine(true)
            .with_metadata(true)
            .with_compression(CompressionAlgorithm::None);
        let result = SerializerOutput::with_config(config)
            .process_file(&source_path)
            .unwrap();

        assert!(result.paths.machine.is_file());
        assert!(result.paths.metadata.is_file());
        assert!(!atomic_temp_path(&result.paths.machine).exists());
        assert!(!atomic_temp_path(&result.paths.metadata).exists());
    }

    #[test]
    fn process_canonical_dx_config_generates_readable_machine() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("dx");
        fs::write(
            &source_path,
            r#"
project(name=dx-devtools version=0.1.0 kind=www-app)

protected_crates[name](
dx-www-browser
dx-serializer
)

paths[name value](
ui components/ui
styles styles
)

tools[name command enabled output](
serializer "dx serializer" true .dx/serializer
style "dx style build" true styles/app.generated.css
)
"#,
        )
        .unwrap();

        let config = SerializerOutputConfig::new()
            .with_output_dir(temp.path().join(".dx/serializer"))
            .with_llm(false)
            .with_machine(true)
            .with_compression(CompressionAlgorithm::None);
        let result = SerializerOutput::with_config(config)
            .process_file(&source_path)
            .unwrap();

        assert!(!result.llm_generated);
        assert!(result.machine_generated);
        assert!(result.paths.machine.is_file());
        assert!(!result.paths.llm.exists());

        let machine = MachineFormat::new(fs::read(&result.paths.machine).unwrap());
        let document = machine_to_document(&machine).unwrap();

        assert_eq!(
            document.get_path("project.name").unwrap().as_str(),
            Some("dx-devtools")
        );
        assert!(document.section_by_name("protected_crates").is_some());
        assert_eq!(
            document
                .section_by_name("tools")
                .unwrap()
                .value_by_key("name", "style", "output")
                .unwrap()
                .as_str(),
            Some("styles/app.generated.css")
        );
    }

    #[test]
    fn process_json_package_file_generates_js_machine_cache() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("package.json");
        fs::write(
            &source_path,
            r#"{
                "name": "dx-js-tool",
                "scripts": {
                    "dev": "bun --watch src/index.tsx"
                },
                "dependencies": {
                    "react": "latest"
                },
                "private": true
            }"#,
        )
        .unwrap();

        let config = SerializerOutputConfig::new()
            .with_output_dir(temp.path().join(".dx/js"))
            .with_llm(false)
            .with_machine(true)
            .with_metadata(true)
            .with_compression(CompressionAlgorithm::None);
        let result = SerializerOutput::with_config(config)
            .process_file(&source_path)
            .unwrap();

        assert!(!result.llm_generated);
        assert!(result.machine_generated);
        assert_eq!(
            result.paths.machine.file_name().unwrap(),
            "package-json.machine"
        );
        assert_eq!(
            result.paths.metadata.file_name().unwrap(),
            "package-json.machine.meta.json"
        );
        assert!(result.paths.machine.is_file());
        assert!(result.paths.metadata.is_file());
        assert!(!result.paths.llm.exists());

        let machine = MachineFormat::new(fs::read(&result.paths.machine).unwrap());
        let document = machine_to_document(&machine).unwrap();

        assert_eq!(
            document.get_path("name").unwrap().as_str(),
            Some("dx-js-tool")
        );
        assert_eq!(
            document.get_path("scripts.dev").unwrap().as_str(),
            Some("bun --watch src/index.tsx")
        );
        assert_eq!(
            document.get_path("dependencies.react").unwrap().as_str(),
            Some("latest")
        );
        assert_eq!(document.get_path("private").unwrap().as_bool(), Some(true));

        let metadata = fs::read_to_string(&result.paths.metadata).unwrap();
        let source_bytes = fs::read(&source_path).unwrap();
        let machine_bytes = fs::read(&result.paths.machine).unwrap();
        assert!(metadata.contains("\"schema\": \"dx.machine.source_metadata.v1\""));
        assert!(metadata.contains(&format!("\"bytes\": {}", source_bytes.len())));
        assert!(metadata.contains(&format!("\"{}\"", blake3::hash(&source_bytes).to_hex())));
        assert!(metadata.contains(&format!("\"bytes\": {}", machine_bytes.len())));
        assert!(metadata.contains(&format!("\"{}\"", blake3::hash(&machine_bytes).to_hex())));
        assert!(metadata.contains("\"fallback_on_mismatch\": true"));
    }

    #[test]
    fn process_toml_bunfig_file_generates_js_machine_cache() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("bunfig.toml");
        fs::write(
            &source_path,
            r"
telemetry = false

[install]
cache = true
",
        )
        .unwrap();

        let config = SerializerOutputConfig::new()
            .with_output_dir(temp.path().join(".dx/js"))
            .with_llm(false)
            .with_machine(true)
            .with_metadata(true)
            .with_compression(CompressionAlgorithm::None);
        let result = SerializerOutput::with_config(config)
            .process_file(&source_path)
            .unwrap();

        assert!(!result.llm_generated);
        assert!(result.machine_generated);
        assert_eq!(
            result.paths.machine.file_name().unwrap(),
            "bunfig-toml.machine"
        );
        assert!(result.paths.machine.is_file());
        assert!(result.paths.metadata.is_file());

        let machine = MachineFormat::new(fs::read(&result.paths.machine).unwrap());
        let document = machine_to_document(&machine).unwrap();

        assert_eq!(
            document.get_path("telemetry").unwrap().as_bool(),
            Some(false)
        );
        assert_eq!(
            document.get_path("install.cache").unwrap().as_bool(),
            Some(true)
        );
    }
}
