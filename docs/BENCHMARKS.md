# DX Serializer Token Efficiency Benchmarks

## Overview

DX Serializer was benchmarked against TOON, TONL, and Tauq across 7 datasets of varying shapes and sizes. All measurements use actual CLI tools and `tiktoken` `cl100k_base` (GPT-4/GPT-3.5 tokenizer).

## Results

### Overall (all 7 datasets combined)

| Format | Bytes | cl100k tokens | vs JSON |
|--------|-------|---------------|---------|
| **DX Serializer** | **54,016** | **15,026** | **-48.8%** |
| TOON | 54,357 | 16,693 | -43.1% |
| TONL | 57,177 | 16,915 | -42.3% |
| Tauq | 56,304 | 17,148 | -41.5% |
| JSON | 105,260 | 29,325 | baseline |

### DX Serializer vs Competitors

| Competitor | DX Serializer tokens | Competitor tokens | DX advantage |
|-----------|-------------------|-------------------|-------------|
| JSON | 15,026 | 29,325 | **48.8% fewer** |
| TOON | 15,026 | 16,693 | **10.0% fewer** |
| TONL | 15,026 | 16,915 | **11.2% fewer** |
| Tauq | 15,026 | 17,148 | **12.4% fewer** |

### By Dataset Type

| Dataset | DX Serializer | TOON | DX better by |
|---------|--------------|------|-------------|
| Config (flat KV) | 46 | 54 | **14.8%** |
| Users (10 rows, 6 cols) | 146 | 172 | **15.1%** |
| Logs (6 rows, 6 cols) | 168 | 190 | **11.6%** |
| Project (nested) | 65 | 94 | **30.9%** |
| E-commerce (objects) | 163 | 180 | **9.4%** |
| CI Pipeline (mixed) | 112 | 216 | **48.1%** |
| Users 1000 rows | 14,326 | 15,787 | **9.3%** |

### Output Modes

| Mode | Tokens | vs JSON | Notes |
|------|--------|---------|-------|
| **Default** | 15,026 | -48.8% | Most token-efficient, recommended for LLM |
| **--format** | 17,079 | -41.8% | +7% tokens for readability |
| **--compact** | 15,022 | -48.8% | Identical to default |

### Small vs Large Data

| Category | DX Serializer | Best competitor | DX advantage |
|----------|--------------|----------------|-------------|
| Small (6 mixed datasets) | 700 | TOON: 906 | **22.7% better** |
| Large (1000-row table) | 14,326 | TOON: 15,787 | **9.3% better** |

## Running Yourself

```bash
# Build DX Serializer CLI
cargo build --release --features converters

# Run the benchmark
python benchmark_formats.py
```

### Prerequisites
- Python 3.10+ with `tiktoken` installed
- Competitor CLIs built (see `G:\Dx\inspirations\`)

### Datasets
The benchmark uses 7 JSON datasets:
- `config`: Flat key-value config (8 keys)
- `users`: 10-row user table (6 columns)
- `logs`: 6-row log table (6 columns)
- `project`: Nested project config
- `ecommerce`: Order with nested objects + arrays
- `ci_pipeline`: CI pipeline with mixed data
- `users_1000`: 1000-row user table (scale test)

## Methodology

1. Write each dataset as a JSON file
2. Convert to each format using actual CLI tools:
   - DX Serializer: `dx-serialize --llm-only --output-dir <dir>`
   - TOON: `toon -e`
   - TONL: `node encodeTONL()`
   - Tauq: `tauq format`
3. Tokenize with `tiktoken` `cl100k_base` (GPT-4) and `o200k_base` (GPT-4o)
4. Compare token counts

Results on this page are for `cl100k_base`. `o200k_base` produces nearly identical rankings.
