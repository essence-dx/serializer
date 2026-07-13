# DX Serializer — Complete Syntax Reference

## 5 Formats

| Format | Files | 
|--------|-------|
| Human | `dx` (no ext), `.sr` |
| Loose | `.loose` |
| LLM | `.llm` |
| Compact | `.compact` |
| Machine | `.machine` |

---

## Human — `key = value` with `()` groups

Used in `.sr` and extensionless `dx` files. This is the source of truth on disk.

```
project(
  name    = dx-os
  version = 1.0.0
  license = Apache-2.0
)
```

Rules:
- Spaces around `=`
- `()` for all groups (not based on child count)
- `=` alignment within a group — pad to longest key
- Root-level flat keys also use `key = value`

## Loose — `[section]` TOML style

Generated as `dx.loose`. Expanded, subsection-numbered format.

```
[project]
name                         = dx-os
version                      = 1.0.0

[recipes:1]
name                         = build
group                        = all
doc                          = Build all workspace crates
```

Multi-row tables get numbered sub-sections: `[table:1]`, `[table:2]`, etc.

## LLM — `:` yml format

Generated as `.llm`. Token-efficient multi-line format.

```
project:
  name: dx-os
  version: 1.0.0
```

Rules:
- `:` after key name, single space before value
- Nested objects indented with 2 spaces
- No `()` wrapping

## Compact — `()` single-line minified

Generated as `dx.compact` with `--compact` flag. Most token-efficient.

```
project(name=dx-os version=1.0.0)
```

Rules:
- `()` for objects, single line
- `key=value` without spaces around `=`
- No newlines within a section
- Space-separated between sections on different lines

---

## Table Syntax

Tables are the core data structure. Example with both separator styles:

```
# Header always uses space separator for column names
recipes[name group doc script](

  # Comma separator — for rows with sentences (no quotes needed)
  build,all,Build all workspace crates,cargo build --workspace

  # Space separator — for rows with only simple values
  b,  build
  c,  check
)

aliases[name target](
  b   build
  br  build-release
)
```

### Separator Rules

| Separator | Best for | Token cost |
|-----------|----------|------------|
| **Space** | Values without spaces (names, flags, booleans) | Lowest |
| **Comma** | Values with spaces (sentences, paths, URLs) | Low |

- Auto-detected per row — mix within the same table
- With comma separator: NO `""` needed — the comma IS the field boundary
- With space separator: values with spaces must use comma separator instead

---

## Data Types

```
# String (unquoted)
name    = dx-os

# String (quoted — only when value has spaces in flat context)
name    = "DX Operating System"

# Number
count   = 42
pi      = 3.14
neg     = -10

# Boolean
active  = true
enabled = false

# Null
value   = null

# Array — [space sep] or [comma,sep], auto-detected
tags    = [rust performance serialization]
tags    = [rust, performance, serialization]
```

---

## Comments

```
# This is a comment
name = dx-os  # This is an end-of-line comment
```

---

## Nesting

```
project(
  name    = dx-os
  scripts(
    build = cargo build
    test  = cargo test
  )
)
```

---

## File Generation

When `dx-serializer human file.dx` runs:

```
file.dx  ──→  .dx/serializer/file.llm     (LLM)
             →  .dx/serializer/file.machine (Machine)
             →  file.loose                  (Loose)
             →  file.compact                (Compact)
```
