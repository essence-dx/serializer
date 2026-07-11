# DX Serializer — Design Decisions (Final)

All decisions finalized. This is the source of truth for the DX Serializer.

---

## 3 Formats

| Format | Flavors | File Extensions | Purpose |
|--------|---------|-----------------|---------|
| **Human** | Normal, Loose | `.sr`, `dx` (none), `.loose` | Source of truth on disk, hand-editable |
| **LLM** | Normal, Compact | `.llm`, `.compact` | Token-optimized for AI context windows |
| **Machine** | — | `.machine` | Binary RKYV, zero-copy runtime access |

## CLI

| Command | Action |
|---------|--------|
| `dx-serializer human <file>` | Parse as Human, generate .llm/.machine/.loose/.compact |
| `dx-serializer llm <file>` | Generate only .llm output |
| `dx-serializer machine <file>` | Generate only .machine output |
| `--compact` | Output compact LLM flavor (single-line `()`) |
| `--stdout` | Print to stdout |

## Human Format (Normal)

```
project(
  name    = dx-os
  version = 1.0.0
)
```

- `()` for groups with 3+ children
- `=` with spaces around it, aligned to longest key
- Groups with 1-2 children also use `()` (consistent style for dx files)

## Human Format (Loose)

```
[project]
name                         = dx-os
version                      = 1.0.0
```

- TOML-like `[section]` headers
- Auto-generated as `dx.loose`

## LLM Format (Normal)

```
project:
  name: dx-os
  version: 1.0.0
```

- `:` yml-style, multi-line
- Auto-generated as `.llm`

## LLM Format (Compact)

```
project(name=dx-os version=1.0.0)
```

- `()` single-line, no newlines within a section
- `key=value` without spaces around `=`
- Auto-generated as `dx.compact` when `--compact` flag is used

## Tables

```
# Header: space separator (simple column names)
recipes[name group doc script](

  # Rows with sentences → comma separator, no quotes needed
  build,all,Build all workspace crates,cargo build --workspace
  check,all,Run cargo check,cargo check --workspace
)

# Spave separator (simple values)
aliases[name target](
  b  build
  c  check
)
```

- Header columns use space separator
- Rows auto-detect: comma if values contain spaces, space for simple values
- When using comma separator, values with spaces do NOT need `""` — the comma is the delimiter

## Data Types

- **String**: unquoted if no spaces, `"quoted"` if spaces
- **Number**: `42`, `3.14`, `-10`, `0`
- **Boolean**: `true`, `false`
- **Null**: `null`
- **Array**: `[item1 item2]` space-separated or `[item1,item2]` comma-separated, auto-detected
- **Object (context)**: `key = value` flat or `parent(child = value)` grouped

## Comments

```
# Line comment
key = value  # end-of-line comment
```

## Conversion Pipeline

```
dx / .sr (Human Normal)
  ├──→ .llm (LLM Normal)
  ├──→ .machine (Machine binary)
  ├──→ .loose (Human Loose)
  └──→ .compact (LLM Compact)
```
