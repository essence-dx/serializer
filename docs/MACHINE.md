# DX Machine Format

**Type:** Binary (RKYV + optional LZ4/Zstd compression)  
**Extension:** `.machine`  
**Location:** `.dx/serializer/` (auto-generated)  
**Purpose:** Zero-copy runtime deserialization

---

## Envelope Structure

Every `.machine` file starts with a 56-byte header:

```
┌──────────────────────────────────────────────────────────────┐
│ Magic:    b"DXM1"               (4 bytes)                    │
│ Version:  u32 LE                (4 bytes)                    │
│ Codec:    u8                    (1 byte)                     │
│           0 = None, 1 = LZ4, 2 = Zstd                       │
│ Checksum: BLAKE3 hash           (32 bytes)                   │
│ DataLen:  u64 LE                (8 bytes)                    │
│ Reserved:                       (7 bytes)                    │
└──────────────────────────────────────────────────────────────┘
```

## Codec Comparison

| Codec | Encode Speed | Decode Speed | Ratio | Use Case |
|-------|-------------|-------------|-------|----------|
| None | ~48ns | zero-copy | 1.0x | Hot path, mmap |
| LZ4 | ~500 MB/s | ~2 GB/s | 1.5-2.0x | Network transfer |
| Zstd-1 | ~300 MB/s | ~1 GB/s | 1.7-2.5x | Default, balanced |

## Performance

| Operation | Small (39 B) | Medium (69 B) | Large (1.6 KB) |
|-----------|-------------|--------------|----------------|
| Serialize | 4.2 µs | 7.1 µs | 131 µs |
| Deserialize (cold) | 1.7 µs | 2.8 µs | 43 µs |
| Deserialize (mmap hot) | 1.3 µs | 2.1 µs | 31 µs |
| Round-trip | 6.1 µs | 10.2 µs | 175 µs |

## Usage

```
# Auto-generated on save (from human format):
# Input:  config.sr or dx file
# Output: .dx/serializer/config.machine

# Rust API:
use serializer::machine::{serialize, deserialize};
let bytes = serialize(&data)?;
let archived = unsafe { deserialize::<MyType>(&bytes) };
```
