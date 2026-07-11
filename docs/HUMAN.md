# DX Human Format

**Type:** Text (readable, hand-editable)  
**Extension:** `.sr`, `dx` (extensionless)  
**Location:** On disk (source of truth)  
**Purpose:** Human-friendly editing and version control

---

## Overview

Human format is the **source of truth** on disk. It's designed to be:
- Beautiful and readable
- Easy to hand-edit
- Friendly to version control diffs
- Converted to LLM and Machine formats automatically

---

## Syntax

### Parenthesized Groups (3+ children)

```
project(
  name    = dx-os
  version = 1.0.0
)
```

- Aligned `=` for visual clarity
- Each child on its own line
- Used when a section has 3+ children

### Inline Style (< 3 children)

```
settings: shell = bash  fallback = true
```

- Colon after the section name
- Children on the same line
- Used when a section has 1-2 children

### Tables

#### Comma-separated (values with spaces — no quotes needed)

Commas act as the delimiter, so values with spaces don't need `""`:

```
recipes[name group doc script](
  build,all,Build all workspace crates,cargo build --workspace
  check,all,Run cargo check,cargo check --workspace
)
```

- Header uses space separator (simple column names)
- Rows use comma separator (doc strings contain spaces)
- No quotes needed — the comma is the delimiter

#### Space-separated (simple values — lower token count)

```
aliases[name target](
  b  build
  c  check
  t  test
)

vars[name value export](
  DX_ROOT              false
  CARGO_PROFILE debug  false
  RUST_LOG      info   false
)
```

- Values have no spaces or commas — space is the natural delimiter
- Lower token count (no comma BPE tokens)

### Flat Key-Value (root level)

```
name    = dx-os
version = 1.0.0
```

---

## Separator Rules

| Separator | When to use | Token cost |
|-----------|-------------|------------|
| **Space** | Values have no spaces (simple names, flags, booleans) | Lowest |
| **Comma** | Values contain spaces (sentences, paths, URLs) | Slightly higher |

- Auto-detected per row — mix within the same table
- No `""` needed with comma separators — the comma is the field boundary
- Header columns always use space separator (column names are simple identifiers)

---

## Design Principles

| Principle | Why |
|-----------|-----|
| **Spaces around `=`** | Readability — visually separates keys from values |
| **Aligned indentation** | Scannable — eyes follow a vertical line |
| **Auto-detected separators** | Flexibility — space for simple, comma for complex |
| **No unnecessary quotes** | Commas delimit fields — no need for `""` |
| **`()` for 3+, inline for 1-2** | Token efficiency without sacrificing readability |
| **Source of truth** | All other formats (LLM, Machine) are derived from Human |

---

## Conversion

```
Human (.sr / dx)  ──→  LLM (.llm)       (token-optimized for AI)
                   ──→  Machine (.machine) (binary, zero-copy)

dx-serializer human file.dx    # validate/process as human format
dx-serializer llm   file.dx    # generate LLM output
dx-serializer machine file.dx  # generate Machine output
```
