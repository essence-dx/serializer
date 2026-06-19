//! Typed `.machine` cache helpers for JSON/config/receipt/index read models.
//!
//! These helpers are intentionally separate from the `DxDocument` machine
//! conversion path. They archive caller-owned Rust types directly so hot paths
//! can parse source JSON once, then validate and read an immutable machine cache.

use std::fs;
use std::io::Write;
#[cfg(feature = "mmap")]
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rkyv::Serialize as RkyvSerialize;
use rkyv::rancor::Error as RkyvError;
use thiserror::Error;

use super::api::serialize;

/// Number of bytes in the typed machine cache header.
pub const MACHINE_CACHE_HEADER_LEN: usize = 256;

type MachineSerializer<'a> = rkyv::rancor::Strategy<
    rkyv::ser::Serializer<
        rkyv::util::AlignedVec,
        rkyv::ser::allocator::ArenaHandle<'a>,
        rkyv::ser::sharing::Share,
    >,
    RkyvError,
>;
type MachineValidator<'a> = rkyv::api::high::HighValidator<'a, RkyvError>;

const MACHINE_CACHE_MAGIC: [u8; 8] = *b"DXMCACH1";
const MACHINE_CACHE_VERSION: u32 = 1;
const MACHINE_CACHE_FLAG_NONE: u32 = 0;

/// Broad kind of typed cache stored in a `.machine` sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineCacheKind {
    /// Source was a JSON document or JSON-derived read model.
    Json,
    /// Source was configuration such as TOML, YAML, or normalized config JSON.
    Config,
    /// Source was a generated receipt or status report.
    Receipt,
    /// Source was an index/catalog/search read model.
    Index,
    /// Project-specific cache kind.
    Custom(u32),
}

impl MachineCacheKind {
    const fn as_u32(self) -> u32 {
        match self {
            Self::Json => 1,
            Self::Config => 2,
            Self::Receipt => 3,
            Self::Index => 4,
            Self::Custom(value) => value,
        }
    }
}

/// Compression codec used by a typed `.machine` cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineCacheCodec {
    /// Store the RKYV archive uncompressed for true mmap-friendly reads.
    None,
    /// LZ4 payload compression for warm caches.
    Lz4,
    /// Zstandard payload compression for cold or distribution caches.
    Zstd,
}

impl MachineCacheCodec {
    const fn as_u32(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Lz4 => 1,
            Self::Zstd => 2,
        }
    }
}

/// Schema identity expected by a typed `.machine` cache reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineCacheSchema {
    /// Stable human-readable schema name, for example `dx.www.forge_status`.
    pub name: &'static str,
    /// Monotonic schema version for this cache payload.
    pub version: u32,
    /// Broad cache kind.
    pub kind: MachineCacheKind,
}

/// Fingerprint for the authoritative source file behind a machine cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCacheSource {
    /// Source file path.
    pub path: PathBuf,
    /// Source file length in bytes.
    pub bytes: u64,
    /// Source modified time as Unix milliseconds, when the platform provides it.
    pub modified_unix_ms: Option<u64>,
    /// BLAKE3 hash of the source file bytes.
    pub blake3: [u8; 32],
}

/// Resolved paths for a source file and its generated machine sidecars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCachePaths {
    /// Authoritative source path.
    pub source: PathBuf,
    /// Generated `.machine` cache path.
    pub machine: PathBuf,
    /// Generated metadata JSON path for diagnostics and receipts.
    pub metadata: PathBuf,
}

/// Write options for a typed `.machine` cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineCacheWriteOptions {
    /// Payload codec. Hot mmap caches should use [`MachineCacheCodec::None`].
    pub codec: MachineCacheCodec,
}

impl Default for MachineCacheWriteOptions {
    fn default() -> Self {
        Self {
            codec: MachineCacheCodec::None,
        }
    }
}

/// Summary of a typed machine cache write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCacheReceipt {
    /// Machine cache path written.
    pub machine: PathBuf,
    /// Metadata path written when metadata support is enabled.
    pub metadata: PathBuf,
    /// Number of archived payload bytes before the typed cache header.
    pub archive_bytes: u64,
    /// Number of bytes written to the `.machine` file.
    pub machine_bytes: u64,
    /// BLAKE3 hash of the RKYV archive bytes.
    pub archive_blake3: [u8; 32],
}

/// Parsed typed machine cache envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCacheEnvelopeV1 {
    /// Cache magic bytes. Current value is `DXMCACH1`.
    pub magic: [u8; 8],
    /// Envelope format version.
    pub version: u32,
    /// Header byte length.
    pub header_bytes: u32,
    /// Broad cache kind ID.
    pub kind_id: u32,
    /// Payload schema version.
    pub schema_version: u32,
    /// Compression codec ID.
    pub codec: u32,
    /// Reserved flags for future use.
    pub flags: u32,
    /// Stored payload length in bytes.
    pub payload_bytes: u64,
    /// Uncompressed archive length in bytes.
    pub archive_bytes: u64,
    /// Authoritative source length in bytes.
    pub source_bytes: u64,
    /// BLAKE3 hash of the authoritative source.
    pub source_blake3: [u8; 32],
    /// BLAKE3 hash of the stored payload bytes.
    pub payload_blake3: [u8; 32],
    /// BLAKE3 hash of the uncompressed archive bytes.
    pub archive_blake3: [u8; 32],
    /// BLAKE3 hash of schema name, version, and kind.
    pub schema_blake3: [u8; 32],
    /// Reserved zero bytes.
    pub reserved: [u8; 64],
}

/// Error returned by typed machine cache helpers.
#[derive(Debug, Error)]
pub enum MachineCacheError {
    /// Underlying I/O operation failed.
    #[error("machine cache IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Source path or cache path is not valid for a project cache.
    #[error("invalid machine cache path: {0}")]
    InvalidCachePath(String),

    /// Project name or cache name contains unsupported characters.
    #[error("invalid machine cache name: {0}")]
    InvalidCacheName(String),

    /// Requested codec is not supported by the typed cache helper yet.
    #[error("unsupported machine cache codec: {0:?}")]
    UnsupportedCodec(MachineCacheCodec),

    /// Cache bytes specified an unknown codec ID.
    #[error("unsupported machine cache codec id: {0}")]
    UnsupportedCodecId(u32),

    /// RKYV serialization failed.
    #[error("machine cache serialization failed: {0}")]
    Serialization(String),

    /// Header magic did not match typed machine cache bytes.
    #[error("invalid machine cache magic")]
    InvalidMagic,

    /// Header version is not supported.
    #[error("unsupported machine cache version: {0}")]
    UnsupportedVersion(u32),

    /// Header length was not the expected fixed size.
    #[error("invalid machine cache header length: {0}")]
    InvalidHeaderLength(u32),

    /// Header reserved bytes or flags were non-zero.
    #[error("machine cache reserved bytes or flags were set")]
    ReservedBytesSet,

    /// Payload length in the header did not match file bytes.
    #[error("machine cache length mismatch")]
    LengthMismatch,

    /// Source fingerprint did not match the authoritative source file.
    #[error("machine cache source fingerprint mismatch")]
    SourceMismatch,

    /// Schema identity did not match the expected typed payload.
    #[error("machine cache schema mismatch")]
    SchemaMismatch,

    /// Payload checksum did not match.
    #[error("machine cache payload checksum mismatch")]
    PayloadChecksumMismatch,

    /// Archive checksum did not match.
    #[error("machine cache archive checksum mismatch")]
    ArchiveChecksumMismatch,

    /// RKYV byte validation failed.
    #[error("machine cache bytecheck failed: {0}")]
    BytecheckFailed(String),

    /// JSON source parsing failed.
    #[cfg(feature = "converters")]
    #[error("machine cache JSON parse failed: {0}")]
    Json(String),

    /// TOML source parsing failed.
    #[cfg(feature = "converters")]
    #[error("machine cache TOML parse failed: {0}")]
    Toml(String),
}

/// Reason a typed cache could not be opened as a valid fast-path hit.
#[derive(Debug, Error)]
pub enum MachineCacheMiss {
    /// Machine cache file does not exist.
    #[error("machine cache missing: {0}")]
    Missing(PathBuf),

    /// Machine cache existed but was invalid, stale, or unreadable.
    #[error(transparent)]
    Invalid(#[from] MachineCacheError),
}

/// Memory-mapped typed machine cache.
#[cfg(feature = "mmap")]
#[derive(Debug)]
pub struct MappedMachineCache<T>
where
    T: rkyv::Archive,
{
    mmap: memmap2::Mmap,
    payload_offset: usize,
    payload_len: usize,
    _marker: PhantomData<T>,
}

#[cfg(feature = "mmap")]
impl<T> MappedMachineCache<T>
where
    T: rkyv::Archive,
{
    /// Returns the validated archived payload.
    #[allow(unsafe_code)]
    pub fn archived(&self) -> &T::Archived {
        let payload = &self.mmap[self.payload_offset..self.payload_offset + self.payload_len];
        // SAFETY: `open_typed_machine_cache` validates this byte range with
        // rkyv::access before constructing the mapped cache.
        unsafe { rkyv::access_unchecked::<T::Archived>(payload) }
    }
}

/// Compute the authoritative source fingerprint for a cache input file.
pub fn source_fingerprint(path: &Path) -> Result<MachineCacheSource, MachineCacheError> {
    let data = fs::read(path)?;
    let metadata = fs::metadata(path)?;
    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| u64::try_from(duration.as_millis()).ok());

    Ok(MachineCacheSource {
        path: path.to_path_buf(),
        bytes: metadata.len(),
        modified_unix_ms,
        blake3: *blake3::hash(&data).as_bytes(),
    })
}

/// Resolve deterministic `.machine` and metadata paths for a project cache.
pub fn paths_for_project_cache(
    project_root: &Path,
    project_name: &str,
    cache_name: &str,
    source_path: &Path,
) -> Result<MachineCachePaths, MachineCacheError> {
    validate_cache_name(project_name)?;
    validate_cache_name(cache_name)?;

    let cache_dir = project_root.join(".dx").join(project_name);
    if !path_stays_under(project_root, &cache_dir) {
        return Err(MachineCacheError::InvalidCachePath(
            cache_dir.display().to_string(),
        ));
    }

    Ok(MachineCachePaths {
        source: source_path.to_path_buf(),
        machine: cache_dir.join(format!("{cache_name}.machine")),
        metadata: cache_dir.join(format!("{cache_name}.machine.meta.json")),
    })
}

/// Write a typed RKYV payload to a validated machine cache sidecar.
pub fn write_typed_machine_cache<T>(
    payload: &T,
    source: &MachineCacheSource,
    paths: &MachineCachePaths,
    schema: MachineCacheSchema,
    options: MachineCacheWriteOptions,
) -> Result<MachineCacheReceipt, MachineCacheError>
where
    T: for<'a> RkyvSerialize<MachineSerializer<'a>>,
{
    if options.codec != MachineCacheCodec::None {
        return Err(MachineCacheError::UnsupportedCodec(options.codec));
    }

    let archive = serialize(payload).map_err(|error| {
        MachineCacheError::Serialization(format!("RKYV serialization failed: {error}"))
    })?;
    let archive_bytes = archive.as_ref();
    let archive_blake3 = *blake3::hash(archive_bytes).as_bytes();
    let schema_blake3 = schema_hash(schema);
    let header = MachineCacheEnvelopeV1 {
        magic: MACHINE_CACHE_MAGIC,
        version: MACHINE_CACHE_VERSION,
        header_bytes: MACHINE_CACHE_HEADER_LEN as u32,
        kind_id: schema.kind.as_u32(),
        schema_version: schema.version,
        codec: options.codec.as_u32(),
        flags: MACHINE_CACHE_FLAG_NONE,
        payload_bytes: archive_bytes.len() as u64,
        archive_bytes: archive_bytes.len() as u64,
        source_bytes: source.bytes,
        source_blake3: source.blake3,
        payload_blake3: archive_blake3,
        archive_blake3,
        schema_blake3,
        reserved: [0; 64],
    };

    if let Some(parent) = paths.machine.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut machine_bytes = Vec::with_capacity(MACHINE_CACHE_HEADER_LEN + archive_bytes.len());
    machine_bytes.extend_from_slice(&encode_header(&header));
    machine_bytes.extend_from_slice(archive_bytes);
    write_atomic(&paths.machine, &machine_bytes)?;

    write_metadata_if_enabled(source, paths, schema, &header)?;

    Ok(MachineCacheReceipt {
        machine: paths.machine.clone(),
        metadata: paths.metadata.clone(),
        archive_bytes: archive_bytes.len() as u64,
        machine_bytes: machine_bytes.len() as u64,
        archive_blake3,
    })
}

/// Validate typed cache bytes and return the archived payload.
pub fn access_typed_machine_cache<'a, T>(
    data: &'a [u8],
    current_source: &MachineCacheSource,
    expected_schema: MachineCacheSchema,
) -> Result<&'a T::Archived, MachineCacheError>
where
    T: rkyv::Archive,
    T::Archived: for<'check> rkyv::bytecheck::CheckBytes<MachineValidator<'check>>,
{
    let payload = validate_machine_cache_bytes(data, current_source, expected_schema)?;
    rkyv::access::<T::Archived, RkyvError>(payload)
        .map_err(|error| MachineCacheError::BytecheckFailed(error.to_string()))
}

/// Open a typed cache with mmap after validating source, envelope, checksums, and bytes.
#[cfg(feature = "mmap")]
#[allow(unsafe_code)]
pub fn open_typed_machine_cache<T>(
    paths: &MachineCachePaths,
    current_source: &MachineCacheSource,
    expected_schema: MachineCacheSchema,
) -> Result<MappedMachineCache<T>, MachineCacheMiss>
where
    T: rkyv::Archive,
    T::Archived: for<'check> rkyv::bytecheck::CheckBytes<MachineValidator<'check>>,
{
    if !paths.machine.exists() {
        return Err(MachineCacheMiss::Missing(paths.machine.clone()));
    }

    let file = fs::File::open(&paths.machine).map_err(MachineCacheError::Io)?;
    // SAFETY: The file is opened read-only and the returned mapping is kept
    // alive inside `MappedMachineCache` for the full lifetime of archived refs.
    let mmap = unsafe { memmap2::MmapOptions::new().map(&file) }.map_err(MachineCacheError::Io)?;
    let payload = validate_machine_cache_bytes(&mmap, current_source, expected_schema)?;
    rkyv::access::<T::Archived, RkyvError>(payload)
        .map_err(|error| MachineCacheError::BytecheckFailed(error.to_string()))?;

    let payload_offset = MACHINE_CACHE_HEADER_LEN;
    let payload_len = payload.len();
    Ok(MappedMachineCache {
        mmap,
        payload_offset,
        payload_len,
        _marker: PhantomData,
    })
}

/// Parse a JSON source file into `T` and write it as a typed machine cache.
#[cfg(feature = "converters")]
pub fn json_source_to_typed_machine_cache<T>(
    source_path: &Path,
    paths: &MachineCachePaths,
    schema: MachineCacheSchema,
    options: MachineCacheWriteOptions,
) -> Result<MachineCacheReceipt, MachineCacheError>
where
    T: serde::de::DeserializeOwned + for<'a> RkyvSerialize<MachineSerializer<'a>>,
{
    let source = source_fingerprint(source_path)?;
    let text = fs::read_to_string(source_path)?;
    let payload = serde_json::from_str::<T>(&text)
        .map_err(|error| MachineCacheError::Json(error.to_string()))?;
    write_typed_machine_cache(&payload, &source, paths, schema, options)
}

/// Parse a TOML source file into `T` and write it as a typed machine cache.
#[cfg(feature = "converters")]
pub fn toml_source_to_typed_machine_cache<T>(
    source_path: &Path,
    paths: &MachineCachePaths,
    schema: MachineCacheSchema,
    options: MachineCacheWriteOptions,
) -> Result<MachineCacheReceipt, MachineCacheError>
where
    T: serde::de::DeserializeOwned + for<'a> RkyvSerialize<MachineSerializer<'a>>,
{
    let source = source_fingerprint(source_path)?;
    let text = fs::read_to_string(source_path)?;
    let payload =
        toml::from_str::<T>(&text).map_err(|error| MachineCacheError::Toml(error.to_string()))?;
    write_typed_machine_cache(&payload, &source, paths, schema, options)
}

fn validate_machine_cache_bytes<'a>(
    data: &'a [u8],
    current_source: &MachineCacheSource,
    expected_schema: MachineCacheSchema,
) -> Result<&'a [u8], MachineCacheError> {
    if data.len() < MACHINE_CACHE_HEADER_LEN {
        return Err(MachineCacheError::LengthMismatch);
    }

    let header = decode_header(&data[..MACHINE_CACHE_HEADER_LEN])?;
    if header.source_bytes != current_source.bytes || header.source_blake3 != current_source.blake3
    {
        return Err(MachineCacheError::SourceMismatch);
    }
    if header.kind_id != expected_schema.kind.as_u32()
        || header.schema_version != expected_schema.version
        || header.schema_blake3 != schema_hash(expected_schema)
    {
        return Err(MachineCacheError::SchemaMismatch);
    }
    if header.codec != MachineCacheCodec::None.as_u32() {
        return match header.codec {
            1 => Err(MachineCacheError::UnsupportedCodec(MachineCacheCodec::Lz4)),
            2 => Err(MachineCacheError::UnsupportedCodec(MachineCacheCodec::Zstd)),
            other => Err(MachineCacheError::UnsupportedCodecId(other)),
        };
    }

    let payload_len =
        usize::try_from(header.payload_bytes).map_err(|_| MachineCacheError::LengthMismatch)?;
    let archive_len =
        usize::try_from(header.archive_bytes).map_err(|_| MachineCacheError::LengthMismatch)?;
    if payload_len != archive_len || data.len() != MACHINE_CACHE_HEADER_LEN + payload_len {
        return Err(MachineCacheError::LengthMismatch);
    }

    let payload = &data[MACHINE_CACHE_HEADER_LEN..];
    let payload_blake3 = *blake3::hash(payload).as_bytes();
    if payload_blake3 != header.payload_blake3 {
        return Err(MachineCacheError::PayloadChecksumMismatch);
    }
    if payload_blake3 != header.archive_blake3 {
        return Err(MachineCacheError::ArchiveChecksumMismatch);
    }

    Ok(payload)
}

fn encode_header(header: &MachineCacheEnvelopeV1) -> [u8; MACHINE_CACHE_HEADER_LEN] {
    let mut bytes = [0u8; MACHINE_CACHE_HEADER_LEN];
    bytes[0..8].copy_from_slice(&header.magic);
    write_u32(&mut bytes, 8, header.version);
    write_u32(&mut bytes, 12, header.header_bytes);
    write_u32(&mut bytes, 16, header.kind_id);
    write_u32(&mut bytes, 20, header.schema_version);
    write_u32(&mut bytes, 24, header.codec);
    write_u32(&mut bytes, 28, header.flags);
    write_u64(&mut bytes, 32, header.payload_bytes);
    write_u64(&mut bytes, 40, header.archive_bytes);
    write_u64(&mut bytes, 48, header.source_bytes);
    bytes[56..88].copy_from_slice(&header.source_blake3);
    bytes[88..120].copy_from_slice(&header.payload_blake3);
    bytes[120..152].copy_from_slice(&header.archive_blake3);
    bytes[152..184].copy_from_slice(&header.schema_blake3);
    bytes[184..248].copy_from_slice(&header.reserved);
    bytes
}

fn decode_header(bytes: &[u8]) -> Result<MachineCacheEnvelopeV1, MachineCacheError> {
    if bytes.len() != MACHINE_CACHE_HEADER_LEN {
        return Err(MachineCacheError::InvalidHeaderLength(bytes.len() as u32));
    }

    let mut magic = [0; 8];
    magic.copy_from_slice(&bytes[0..8]);
    if magic != MACHINE_CACHE_MAGIC {
        return Err(MachineCacheError::InvalidMagic);
    }

    let version = read_u32(bytes, 8);
    if version != MACHINE_CACHE_VERSION {
        return Err(MachineCacheError::UnsupportedVersion(version));
    }

    let header_bytes = read_u32(bytes, 12);
    if header_bytes != MACHINE_CACHE_HEADER_LEN as u32 {
        return Err(MachineCacheError::InvalidHeaderLength(header_bytes));
    }

    let flags = read_u32(bytes, 28);
    let mut reserved = [0; 64];
    reserved.copy_from_slice(&bytes[184..248]);
    if flags != MACHINE_CACHE_FLAG_NONE || reserved.iter().any(|byte| *byte != 0) {
        return Err(MachineCacheError::ReservedBytesSet);
    }
    if bytes[248..MACHINE_CACHE_HEADER_LEN]
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(MachineCacheError::ReservedBytesSet);
    }

    let mut source_blake3 = [0; 32];
    source_blake3.copy_from_slice(&bytes[56..88]);
    let mut payload_blake3 = [0; 32];
    payload_blake3.copy_from_slice(&bytes[88..120]);
    let mut archive_blake3 = [0; 32];
    archive_blake3.copy_from_slice(&bytes[120..152]);
    let mut schema_blake3 = [0; 32];
    schema_blake3.copy_from_slice(&bytes[152..184]);

    Ok(MachineCacheEnvelopeV1 {
        magic,
        version,
        header_bytes,
        kind_id: read_u32(bytes, 16),
        schema_version: read_u32(bytes, 20),
        codec: read_u32(bytes, 24),
        flags,
        payload_bytes: read_u64(bytes, 32),
        archive_bytes: read_u64(bytes, 40),
        source_bytes: read_u64(bytes, 48),
        source_blake3,
        payload_blake3,
        archive_blake3,
        schema_blake3,
        reserved,
    })
}

fn schema_hash(schema: MachineCacheSchema) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(schema.name.as_bytes());
    hasher.update(&schema.version.to_le_bytes());
    hasher.update(&schema.kind.as_u32().to_le_bytes());
    *hasher.finalize().as_bytes()
}

fn validate_cache_name(name: &str) -> Result<(), MachineCacheError> {
    let valid = !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains("..")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if valid {
        Ok(())
    } else {
        Err(MachineCacheError::InvalidCacheName(name.to_string()))
    }
}

fn path_stays_under(project_root: &Path, path: &Path) -> bool {
    path.starts_with(project_root.join(".dx"))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), MachineCacheError> {
    let tmp = path.with_extension(format!(
        "{}.tmp.{}",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("machine"),
        std::process::id()
    ));

    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }

    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(feature = "converters")]
fn write_metadata_if_enabled(
    source: &MachineCacheSource,
    paths: &MachineCachePaths,
    schema: MachineCacheSchema,
    header: &MachineCacheEnvelopeV1,
) -> Result<(), MachineCacheError> {
    use serde::Serialize;

    #[derive(Serialize)]
    struct Metadata<'a> {
        cache_schema: &'a str,
        cache_version: u32,
        cache_kind: u32,
        source_path: String,
        source_len: u64,
        source_modified_unix_ms: Option<u64>,
        source_blake3: String,
        machine_path: String,
        machine_payload_len: u64,
        machine_archive_len: u64,
        machine_payload_blake3: String,
    }

    let metadata = Metadata {
        cache_schema: schema.name,
        cache_version: schema.version,
        cache_kind: schema.kind.as_u32(),
        source_path: source.path.display().to_string(),
        source_len: source.bytes,
        source_modified_unix_ms: source.modified_unix_ms,
        source_blake3: hex32(source.blake3),
        machine_path: paths.machine.display().to_string(),
        machine_payload_len: header.payload_bytes,
        machine_archive_len: header.archive_bytes,
        machine_payload_blake3: hex32(header.payload_blake3),
    };

    let bytes = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| MachineCacheError::Json(error.to_string()))?;
    write_atomic(&paths.metadata, &bytes)
}

#[cfg(not(feature = "converters"))]
fn write_metadata_if_enabled(
    _source: &MachineCacheSource,
    _paths: &MachineCachePaths,
    _schema: MachineCacheSchema,
    _header: &MachineCacheEnvelopeV1,
) -> Result<(), MachineCacheError> {
    Ok(())
}

#[cfg(feature = "converters")]
fn hex32(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap_or([0; 4]))
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap_or([0; 8]))
}

#[cfg(all(test, feature = "converters"))]
mod tests {
    use std::fs;

    use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
    use serde::Deserialize;
    use tempfile::tempdir;

    use super::{
        MachineCacheError, MachineCacheKind, MachineCacheSchema, MachineCacheWriteOptions,
        access_typed_machine_cache, json_source_to_typed_machine_cache, paths_for_project_cache,
        source_fingerprint, write_typed_machine_cache,
    };

    #[derive(Archive, RkyvSerialize, RkyvDeserialize, Deserialize, Debug, PartialEq)]
    #[rkyv(compare(PartialEq), derive(Debug))]
    struct TestPayload {
        id: u64,
        flags: u32,
        active: bool,
    }

    const fn schema() -> MachineCacheSchema {
        MachineCacheSchema {
            name: "dx.test.payload",
            version: 1,
            kind: MachineCacheKind::Json,
        }
    }

    #[test]
    fn source_fingerprint_changes_for_same_length_content_change() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("source.json");

        fs::write(&source_path, b"{\"id\":1}").unwrap();
        let first = source_fingerprint(&source_path).unwrap();

        fs::write(&source_path, b"{\"id\":2}").unwrap();
        let second = source_fingerprint(&source_path).unwrap();

        assert_eq!(first.bytes, second.bytes);
        assert_ne!(first.blake3, second.blake3);
    }

    #[test]
    fn paths_for_project_cache_uses_project_cache_root() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("nested").join("source.json");

        let paths =
            paths_for_project_cache(temp.path(), "check", "check-report-latest", &source_path)
                .unwrap();

        assert_eq!(paths.source, source_path);
        assert_eq!(
            paths.machine,
            temp.path()
                .join(".dx")
                .join("check")
                .join("check-report-latest.machine")
        );
        assert_eq!(
            paths.metadata,
            temp.path()
                .join(".dx")
                .join("check")
                .join("check-report-latest.machine.meta.json")
        );
    }

    #[test]
    fn paths_for_project_cache_rejects_parent_traversal_names() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("source.json");

        let error = paths_for_project_cache(temp.path(), "..", "cache", &source_path).unwrap_err();
        assert!(matches!(error, MachineCacheError::InvalidCacheName(_)));

        let error =
            paths_for_project_cache(temp.path(), "check", "../cache", &source_path).unwrap_err();
        assert!(matches!(error, MachineCacheError::InvalidCacheName(_)));
    }

    #[test]
    fn write_typed_machine_cache_writes_readable_machine_and_metadata() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, b"{\"id\":1,\"flags\":7,\"active\":true}").unwrap();

        let source = source_fingerprint(&source_path).unwrap();
        let paths = paths_for_project_cache(temp.path(), "check", "payload", &source_path).unwrap();
        let payload = TestPayload {
            id: 1,
            flags: 7,
            active: true,
        };

        let receipt = write_typed_machine_cache(
            &payload,
            &source,
            &paths,
            schema(),
            MachineCacheWriteOptions::default(),
        )
        .unwrap();

        assert_eq!(receipt.machine, paths.machine);
        assert!(paths.machine.exists());
        assert!(paths.metadata.exists());
        assert_eq!(
            fs::read_dir(paths.machine.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
                .count(),
            0
        );

        let bytes = fs::read(&paths.machine).unwrap();
        let archived =
            access_typed_machine_cache::<TestPayload>(&bytes, &source, schema()).unwrap();
        assert_eq!(archived.id, 1);
        assert_eq!(archived.flags, 7);
        assert!(archived.active);
    }

    #[test]
    fn json_source_to_typed_machine_cache_parses_source_once_into_machine() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, b"{\"id\":9,\"flags\":3,\"active\":false}").unwrap();

        let paths =
            paths_for_project_cache(temp.path(), "www", "forge-status", &source_path).unwrap();
        json_source_to_typed_machine_cache::<TestPayload>(
            &source_path,
            &paths,
            schema(),
            MachineCacheWriteOptions::default(),
        )
        .unwrap();

        let source = source_fingerprint(&source_path).unwrap();
        let bytes = fs::read(&paths.machine).unwrap();
        let archived =
            access_typed_machine_cache::<TestPayload>(&bytes, &source, schema()).unwrap();
        assert_eq!(archived.id, 9);
        assert_eq!(archived.flags, 3);
        assert!(!archived.active);
    }

    #[test]
    fn corrupt_machine_header_is_rejected_as_cache_miss_material() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, b"{\"id\":1,\"flags\":7,\"active\":true}").unwrap();

        let source = source_fingerprint(&source_path).unwrap();
        let paths = paths_for_project_cache(temp.path(), "check", "payload", &source_path).unwrap();
        let payload = TestPayload {
            id: 1,
            flags: 7,
            active: true,
        };
        write_typed_machine_cache(
            &payload,
            &source,
            &paths,
            schema(),
            MachineCacheWriteOptions::default(),
        )
        .unwrap();

        let mut bytes = fs::read(&paths.machine).unwrap();
        bytes[0] = b'X';
        let error =
            access_typed_machine_cache::<TestPayload>(&bytes, &source, schema()).unwrap_err();
        assert!(matches!(error, MachineCacheError::InvalidMagic));
    }

    #[test]
    fn source_fingerprint_mismatch_is_rejected() {
        let temp = tempdir().unwrap();
        let source_path = temp.path().join("source.json");
        fs::write(&source_path, b"{\"id\":1,\"flags\":7,\"active\":true}").unwrap();

        let source = source_fingerprint(&source_path).unwrap();
        let paths = paths_for_project_cache(temp.path(), "check", "payload", &source_path).unwrap();
        let payload = TestPayload {
            id: 1,
            flags: 7,
            active: true,
        };
        write_typed_machine_cache(
            &payload,
            &source,
            &paths,
            schema(),
            MachineCacheWriteOptions::default(),
        )
        .unwrap();

        fs::write(&source_path, b"{\"id\":2,\"flags\":7,\"active\":true}").unwrap();
        let changed_source = source_fingerprint(&source_path).unwrap();
        let bytes = fs::read(&paths.machine).unwrap();
        let error = access_typed_machine_cache::<TestPayload>(&bytes, &changed_source, schema())
            .unwrap_err();
        assert!(matches!(error, MachineCacheError::SourceMismatch));
    }
}
