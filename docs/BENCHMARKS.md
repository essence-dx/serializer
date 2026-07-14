# DX Serializer — Benchmark Results

> **Tokenizer**: o200k_base (GPT-4o / GPT-4o-mini) via `gpt-tokenizer`  
> **Encoder**: `@dx-serializer/core` Bun encoder (no abbreviation engine)  
> **Date**: 2026-07-14  
> **Files**: 20 benchmark files across 6 categories  

---

## Overall Statistics

| Comparison | Average | Min | Max | Samples |
|-----------|---------|-----|-----|---------|
| **DX vs JSON pretty** | **−58.9%** | −20.8% | −86.4% | 20 |
| **DX vs JSON compact** | **−37.1%** | −7.4% | −75.0% | 20 |
| **DX vs YAML** | **−38.5%** | −2.3% | −60.4% | 11 |
| **DX vs TOON** | **−33.6%** | −4.7% | −50.6% | 20 |

**DX beats TOON on every benchmark file (20/20).**

---

## Per-Category Results

### Main Benchmarks (3 files)

| Format | Tokens | vs JSON | vs TOON |
|--------|-------:|--------:|--------:|
| JSON pretty | 3,303 | — | — |
| TOON | 1,618 | −51% | — |
| **DX** | **1,299** | **−61%** | **−20%** |

### Tool Schemas (2 files)

| Format | Tokens | vs JSON | vs TOON |
|--------|-------:|--------:|--------:|
| JSON pretty | 3,147 | — | — |
| TOON | 2,398 | −24% | — |
| **DX** | **1,502** | **−52%** | **−37%** |

### Tool Calls (4 files)

| Format | Tokens | vs JSON | vs TOON |
|--------|-------:|--------:|--------:|
| JSON pretty | 675 | — | — |
| TOON | 533 | −21% | — |
| **DX** | **353** | **−48%** | **−34%** |

### Showcase (2 files)

| Format | Tokens | vs JSON | vs TOON |
|--------|-------:|--------:|--------:|
| JSON pretty | 3,578 | — | — |
| TOON | 2,668 | −25% | — |
| **DX** | **1,436** | **−60%** | **−46%** |

### Extreme (4 files)

| Format | Tokens | vs JSON | vs TOON |
|--------|-------:|--------:|--------:|
| JSON pretty | 86,137 | — | — |
| TOON | 68,467 | −21% | — |
| **DX** | **36,015** | **−58%** | **−47%** |

### Record Benchmarks (4 files)

| Format | Tokens | vs JSON | vs TOON |
|--------|-------:|--------:|--------:|
| JSON pretty | 3,409,027 | — | — |
| TOON | 1,018,036 | −70% | — |
| **DX** | **678,022** | **−80%** | **−33%** |

---

## Record-Breaking Results

| Scenario | DX Tokens | vs JSON | vs JSONC | vs TOON |
|----------|----------:|--------:|---------:|--------:|
| **100K boolean records, 3 cols** | **300,006** | **−86%** | **−75%** | −40% |
| **10K boolean records, 5 cols** | **50,008** | **−85%** | **−75%** | −29% |
| 100K flat records, 2 cols | 399,004 | −79% | −56% | −33% |
| 50K flat records, 2 cols | 199,004 | −79% | −56% | −33% |
| 10K bools, 3 cols | 30,006 | −86% | −75% | −40% |
| 80-tool schema | 7,686 | −69% | −40% | −49% |
| 20-tool agent schema | 1,114 | −61% | −34% | −46% |
| 40-provider catalog | 667 | −66% | −52% | −21% |
| 200-tool schema | 15,005 | −42% | −42% | −51% |
| 12-tool coding assistant | 1,268 | −51% | −24% | −36% |
| 100-item github repos | 8,336 | −45% | −27% | −5% |
| 4-tool schema | 234 | −57% | −29% | −42% |
| 6-tool batch call | 166 | −54% | −31% | −43% |

---

## Methodology

1. **All DX files generated** via `@dx-serializer/core` Bun encoder (`encode()`)
2. **All TOON files generated** via official `@toon-format/cli` v2.3.0 (`toon -e`)
3. **JSON**: Pretty-printed with 2-space indent
4. **JSONC**: Minified JSON (no whitespace)
5. **YAML**: Generated via `yaml` npm package
6. **Token counting**: `gpt-tokenizer` npm package with o200k_base encoding
7. **Round-trip verified**: All DX files decode back to original JSON
