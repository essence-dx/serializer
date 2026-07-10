# DX Serializer

Token-optimized serialization format for AI context windows with ~49% token savings vs compact JSON (~70% vs pretty-printed JSON) and pure RKYV binary format.

## Direct JSON vs `.machine` Proof

DX Serializer's generated `.machine` files are not a cosmetic cache. When used directly, the typed RKYV + mmap path is dramatically faster than parsing JSON again.

This proof compares three JSON fixture sizes against pre-generated DX `.machine` files. Generation time is excluded because the DX ecosystem creates `.machine` files ahead of runtime. Each path reads the same payload shape and checksum-verifies the same data.

| case | JSON size | `.machine` size | JSON parse | JSON read + parse | `.machine` validated | `.machine` hot mmap |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | 85,969 B | 41,824 B | 305.693us | 416.907us | 28.001us | 1.334us |
| medium | 2,751,387 B | 1,336,240 B | 8.667ms | 10.461ms | 675.906us | 55.974us |
| large | 22,024,633 B | 10,690,480 B | 98.201ms | 129.378ms | 5.538ms | 499.677us |

| case | validated `.machine` vs JSON parse | hot mmap `.machine` vs JSON parse | hot mmap `.machine` vs JSON read + parse |
| --- | ---: | ---: | ---: |
| small | 10.92x faster | 229.11x faster | 312.47x faster |
| medium | 12.82x faster | 154.85x faster | 186.90x faster |
| large | 17.73x faster | 196.53x faster | 258.92x faster |

Brutal truth: JSON is excellent as an interchange format, but it is the wrong runtime read model for already-known data. DX `.machine` turns JSON-derived data into a compact binary read model that can be mmaped and accessed without reparsing. That is the unlock.

### Machine Format Performance

**DX-Machine uses pure RKYV** — identical performance:
- Single serialize: ~48-51ns (RKYV: ~48ns, within 6% variance)
- Batch 100: ~7.5µs (RKYV: ~7.9µs, actually 5% faster)
- Zero-copy deserialization (identical to RKYV)
- Production-ready and battle-tested
**Implementation**: Zero-overhead wrapper with `#[inline(always)]` that compiles to identical machine code as RKYV.

## Three Formats

DX Serializer uses a revolutionary 3-format system:

### Human Format (.sr / .dx files on disk)

Beautiful, readable format that developers edit directly:
- TOML/INI-like syntax with aligned `=` at column 28
- **Lives on real disk** where you work (e.g., `dx`, `package.sr`)
- Easy to read, write, and version control
- This is the **source of truth** — you edit these files

### LLM Format (.sr / .llm in .dx/serializer/)

Token-optimized format for AI context windows with **three optimization levels**:
- **Low**: Compact single-line, maximum token efficiency
- **Medium** (default): Balanced auto-select — YAML-style for small objects, parens for large ones
- **High**: Human-readable YAML-style with `=`, for `.dx` config files
- ~49% token savings vs compact JSON (~70% vs pretty-printed)
- Beats TOON by 11%, Tauq by 14%, TONL by 13%
- **Auto-generated** in `.dx/serializer/*.sr` folder
- Never edit manually — regenerated from human format

### Machine Format (.machine in .dx/serializer/)

Pure RKYV binary format for maximum performance:
- Zero-copy deserialization
- ~48-51ns serialize time
- **Auto-generated** in `.dx/serializer/*.machine` folder
- Identical to RKYV wire format

**Architecture**: Human format files live on disk. When you save a `dx` file (or any file with DX serializer syntax), the extension automatically generates the `.sr` and `.machine` versions in the `.dx/serializer/` folder. The `.dx/` folder is gitignored as it contains generated files.

**Note**: DX-Machine IS RKYV. We use RKYV's wire format directly with no modifications.

## CLI Usage

### Standalone `dx-serializer` binary

```
DX Serializer — token-efficient LLM serialization

Usage:
  dx-serializer <file> [options]                 Process file (Medium level)
  dx-serializer low <file> [options]              Low optimization (compact single-line)
  dx-serializer medium <file> [options]           Medium optimization (balanced, auto-select)
  dx-serializer high <file> [options]             High optimization (human-readable)
  dx-serializer convert json <file> [options]     Convert JSON to DX LLM format
  dx-serializer convert yml <file> [options]      Convert YAML to DX LLM format
  dx-serializer convert toon <file> [options]     Convert TOON to DX LLM format
  dx-serializer convert jsonc <file> [options]    Convert JSONC to DX LLM format

Options:
  --output-dir <dir>       Output directory (default: .dx/serializer)
  --js-cache               Default output directory to .dx/js
  --machine-only           Generate only .machine output
  --metadata               Generate .machine.meta.json validation sidecar
  --llm-only               Generate only .llm output
  --lz4 | --speed          Use LZ4 compression (fastest, default)
  --zstd | --size          Use Zstd compression (better ratio)
  --no-compression         Disable compression
  --beautify               Human-readable output
  --format                 Formatted LLM output
  --stdout                 Print LLM output to stdout instead of writing files
```

### Via `dx serializer` CLI

All commands also work through the unified DX CLI:

```bash
dx serializer                           # Show workspace status
dx serializer low file.json --stdout    # Low optimization to stdout
dx serializer medium file.sr            # Medium optimization (default)
dx serializer high config.dx            # Human-readable YAML-style
dx serializer convert json pkg.json     # Convert JSON to DX LLM
dx serializer convert yml config.yml    # Convert YAML to DX LLM
dx serializer convert toon data.toon    # Convert TOON to DX LLM
```

## Optimization Levels

The serializer auto-selects the format that balances token efficiency and readability:

| Level | Objects (< 8 children) | Objects (8+ children) | Tables | Best For |
|-------|----------------------|----------------------|--------|----------|
| **Low** | `key(field=val)` (compact) | `big(k0=0 k1=1...)` (compact) | `name[cols](row1,row2)` (single-line) | Max token savings |
| **Medium** | YAML: `key:\n  field: val` | Parens: `key(\n  field = val\n)` | Unindented rows, space-sep | Balanced |
| **High** | YAML: `key:\n  field = val` | YAML: `key:\n  field = val` | Unindented rows | `.dx` config files |

### Examples

**Medium** (YAML auto-selected for < 8 children):
```
project:
  name: DX Serializer
  version: 1.0.0
  features:
    converters: true
```

**Low** (compact single-line):
```
project(name=DX Serializer version=1.0.0 features(converters=true compression=true))
```

**High** (human-readable YAML with `=`):
```
project:
  name = DX Serializer
  version = 1.0.0
  features:
    converters = true
```

## LLM Format Examples

### Scalars
```
name = MyApp
version = 1.0.0
port = 8080
active = true
description = "Multi word string"
```

### Arrays (smart separator)
```
tags = rust performance serialization          # space-sep for simple tokens
editors = neovim, zed, "firebase studio"       # comma-sep when values have spaces
```

### Objects (multi-line)
```
config(
  host = localhost
  port = 5432
  debug = true
)
```

### Tables (wrapped dataframes)
```
users[id name email](
1 "Alice Johnson" alice@example.com
2 "Bob Smith" bob@example.com
)
```

### Nested Objects
```
project(
  name = "DX Serializer"
  version = 1.0.0
  features(
    converters = true
    compression = true
    wasm = true
  )
)
```

## Human Format Example
```
author = essensefromexistence
version = 0.0.1
name = dx
description = Orchestrate dont just own your code

[driven]
path = @/driven

[editors]
default = neovim
items:
- neovim
- zed
- vscode
```

## Format Locations

**Architecture Overview**:

- **Human format** — Lives on **real disk**, you edit these files directly
  - Examples: `dx`, `package.sr`
  - Source of truth, version controlled in git
  - TOML/INI-like syntax with aligned `=` at column 28

- **LLM format** (.sr / .llm) — **Auto-generated** in `.dx/serializer/` folder
  - Low / Medium / High optimization levels
  - Never edit manually
  - ~49% token savings vs compact JSON

- **Machine format** (.machine) — **Auto-generated** in `.dx/serializer/` folder
  - Binary format (pure RKYV)
  - Zero-copy deserialization
  - ~48-51ns serialize time

The `.dx/` folder is gitignored as it contains generated files. Only commit human format files.

## Machine Format (RKYV)

**DX-Machine IS RKYV** — we use RKYV directly:
- Pure RKYV wire format (no modifications)
- Zero-overhead wrapper with `#[inline(always)]`
- Identical performance: ~48-51ns single, ~7.5µs batch 100
- Zero-copy deserialization
- Production-ready

```rust
use serializer::machine::{serialize, deserialize};
let bytes = serialize(&data)?;
let archived = unsafe { deserialize::<MyType>(&bytes) };
```

## Machine Format Compression

DX Machine format supports optional compression using LZ4 and ZSTD.

| Algorithm | Compression Speed | Decompression Speed | Ratio | Use Case |
|-----------|-------------------|---------------------|-------|----------|
| LZ4 | ~500 MB/s | ~2000 MB/s | 50-70% | Network, real-time |
| ZSTD-1 | ~300 MB/s | ~1000 MB/s | 60-80% | Fast compression |
| ZSTD-3 | ~100 MB/s | ~800 MB/s | 70-85% | Balanced |
| ZSTD-19 | ~10 MB/s | ~500 MB/s | 75-90% | Maximum compression |

```rust
use serializer::machine::compress::{DxCompressed, CompressionLevel};
let compressed = DxCompressed::compress(b"your binary data here");
```

## Token Efficiency vs Competitors

DX Serializer was benchmarked against TOON, TONL, and Tauq across 7 datasets using `tiktoken` `cl100k_base` (GPT-4 tokenizer).

| Format | Total tokens | vs JSON | vs DX Serializer |
|--------|-------------|---------|-----------------|
| **DX Serializer** | **15,026** | **-48.8%** | — |
| TOON | 16,693 | -43.1% | +11.1% |
| TONL | 16,915 | -42.3% | +12.6% |
| Tauq | 17,148 | -41.5% | +14.1% |
| JSON | 29,325 | baseline | +95.2% |

## Quick Start

```rust
use serializer::{json_to_dx, dx_to_json};
let json = r#"{"name": "app", "version": "1.0"}"#;
let dx = json_to_dx(json)?;
```

## Features

```toml
[dependencies]
dx-serializer = { version = "0.1", features = ["tiktoken"] }
```

| Feature | Description |
|---------|-------------|
| `converters` | JSON/YAML/TOML support |
| `compression` | LZ4 + ZSTD compression |
| `watch` | File watching daemon |
| `tiktoken` | Token counting |

## Documentation

- [Syntax Reference](docs/SYNTAX.md) — Complete LLM and Human format syntax
- [LLM Format Spec](LLM_FORMAT_SPEC.md) — Detailed format specification
- [API Reference](docs/API.md) — Rust API documentation
- [Benchmarks](docs/BENCHMARKS.md) — Performance comparisons

## License

MIT / Apache-2.0
