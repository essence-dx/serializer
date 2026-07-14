# Format Examples

Source: `sample.json` — 30-line project config with nested objects and arrays.

## Files

| File | Format | Generator | Bytes |
|------|--------|-----------|------:|
| `sample.json` | JSON | Source file | 1,067 |
| `sample.jsonc` | JSONC | JSON + comments (manual) | 1,205 |
| `sample.dx` | DX LLM | `json_to_document` → `document_to_llm` | 802 |
| `sample.dx-compact` | DX Compact | `json_to_document` → `LlmSerializer` | 753 |
| `sample.toon` | TOON | `npx @toon-format/cli sample.json -o sample.toon` | 827 |

## Commands

```bash
# DX LLM — via dx-serializer library (CLI's convert json produces broken format)
cargo run --example gen_dx

# TOON — official CLI via npx
npx @toon-format/cli sample.json -o sample.toon

# JSONC — manual (dx-serializer has no JSON→JSONC converter)

# Token comparison
dx-token compare sample.json sample.jsonc sample.toon sample.dx sample.dx-compact
```

## Token Counts (dx-token compare, real BPE tokenizers)

```
+-------------------+-------------+-----------+-----------+------------+
| Format            | cl100k_base | p50k_base | r50k_base | o200k_base |
+-------------------+-------------+-----------+-----------+------------+
| DX Compact        | 213         | 255       | 255       | 213        |
| DX LLM            | 238         | 259       | 259       | 239        |
| TOON              | 273         | 317       | 317       | 274        |
| JSON              | 350         | 420       | 452       | 349        |
| JSONC             | 383         | 453       | 485       | 382        |
+-------------------+-------------+-----------+-----------+------------+
```

**Ranked by GPT-4o (o200k_base) tokens:**

| # | Format | Tokens | vs JSON |
|---|--------|-------:|--------:|
| 1 | DX Compact | 213 | -39% |
| 2 | DX LLM | 239 | -32% |
| 3 | TOON | 274 | -21% |
| 4 | JSON | 349 | baseline |
| 5 | JSONC | 382 | +9% |

**Key observations:**
- DX Compact saves **39%** vs JSON — nested objects collapse into `section(key=value)` inline form
- DX LLM saves **32%** vs JSON — uses `key = value` without braces/quotes/commas
- TOON saves **21%** vs JSON — drops quotes and commas, adds array length markers
- JSONC costs **9%** more than JSON — comments add tokens

## Note on DX CLI

The `dx-serializer convert json` command produces a broken format (`c.key:value^key:value` instead of proper `key=value`). This is a known bug in `src/converters/json.rs:303` where `convert_object()` hardcodes `if true` to always use the old inline format. The proper DX output above was generated through the Rust library API (`json_to_document` → `document_to_llm`).
