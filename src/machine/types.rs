//! Machine format error types

use thiserror::Error;

/// Result type for machine operations
pub type Result<T> = std::result::Result<T, DxMachineError>;

/// Machine format errors
#[derive(Debug, Error)]
pub enum DxMachineError {
    /// Machine-format serialization failed.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Machine-format deserialization failed.
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Compression failed before writing machine data.
    #[error("Compression error: {0}")]
    Compression(String),

    /// Decompression failed while reading machine data.
    #[error("Decompression error: {0}")]
    Decompression(String),

    /// Decompression completed unsuccessfully with an implementation-specific message.
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),

    /// Bytes did not match the expected machine format.
    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    /// Machine payload contained invalid data for the requested operation.
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Magic bytes did not match the expected machine header.
    #[error("Invalid magic bytes")]
    InvalidMagic,

    /// Provided buffer was too small for the requested read or write.
    #[error("Buffer too small: required {required}, got {actual}")]
    BufferTooSmall {
        /// Number of bytes required by the operation.
        required: usize,
        /// Number of bytes available in the provided buffer.
        actual: usize,
    },

    /// Underlying I/O operation failed.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
