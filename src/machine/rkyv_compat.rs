//! RKYV compatibility layer with DX-Machine ultra-performance enhancements
//!
//! This module provides RKYV-compatible APIs while enabling access to DX-Machine's
//! advanced performance features through feature flags.

// Re-export RKYV directly for zero-overhead performance
pub use rkyv::Archive;
pub use rkyv::Deserialize as RkyvDeserialize;
pub use rkyv::Serialize as RkyvSerialize;
pub use rkyv::access_unchecked;
pub use rkyv::from_bytes;
pub use rkyv::to_bytes;

use crate::machine::DxMachineError;

/// Deserialize without validation (trust mode).
///
/// # Safety
///
/// `bytes` must contain a valid archived representation of `T` from a trusted
/// source. This function intentionally skips byte validation before accessing
/// the archived value, so it must not be used on network, registry, or user
/// supplied bytes unless those bytes have already been authenticated and
/// validated by another layer.
#[inline(always)]
#[allow(unsafe_code)]
pub unsafe fn from_bytes_unchecked<T>(bytes: &[u8]) -> Result<T, DxMachineError>
where
    T: rkyv::Archive,
    T::Archived: rkyv::Deserialize<T, rkyv::rancor::Strategy<rkyv::de::Pool, rkyv::rancor::Error>>,
{
    // SAFETY: Caller guarantees data is from trusted source
    unsafe {
        let archived = rkyv::access_unchecked::<T::Archived>(bytes);
        let mut deserializer = rkyv::de::Pool::new();
        archived
            .deserialize(rkyv::rancor::Strategy::wrap(&mut deserializer))
            .map_err(|_| DxMachineError::InvalidData("Deserialization failed".into()))
    }
}

/// Zero-copy access to archived data (no deserialization!)
///
/// This is the fastest way to access data - no allocation, no copying.
/// Returns a reference to the archived representation.
#[inline(always)]
#[allow(unsafe_code)]
pub fn access_archived<T>(bytes: &[u8]) -> Result<&T::Archived, DxMachineError>
where
    T: rkyv::Archive,
{
    // SAFETY: This compatibility helper preserves the original unchecked
    // behavior. Callers should prefer validated RKYV access for untrusted bytes.
    unsafe { Ok(rkyv::access_unchecked::<T::Archived>(bytes)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

    #[derive(Debug, Clone, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
    struct TestStruct {
        id: u64,
        name: String,
        active: bool,
    }

    #[test]
    fn test_roundtrip() {
        let original = TestStruct {
            id: 42,
            name: "test".to_string(),
            active: true,
        };

        let bytes = to_bytes::<rkyv::rancor::Error>(&original).unwrap();
        let decoded: TestStruct = from_bytes::<TestStruct, rkyv::rancor::Error>(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    #[allow(unsafe_code)]
    fn test_trust_mode() {
        let original = vec![1u64, 2, 3, 4, 5];
        let bytes = to_bytes::<rkyv::rancor::Error>(&original).unwrap();

        // Safe mode
        let decoded_safe: Vec<u64> = from_bytes::<Vec<u64>, rkyv::rancor::Error>(&bytes).unwrap();
        assert_eq!(original, decoded_safe);

        // Trust mode (faster)
        let decoded_trust: Vec<u64> = unsafe { from_bytes_unchecked(&bytes).unwrap() };
        assert_eq!(original, decoded_trust);
    }

    #[test]
    fn test_zero_copy_access() {
        let original = TestStruct {
            id: 42,
            name: "test".to_string(),
            active: true,
        };

        let bytes = to_bytes::<rkyv::rancor::Error>(&original).unwrap();

        // Zero-copy access (no deserialization!)
        let archived = access_archived::<TestStruct>(&bytes).unwrap();
        assert_eq!(archived.id, 42);
        assert!(archived.active);
    }
}
