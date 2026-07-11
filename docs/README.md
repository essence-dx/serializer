# DX Serializer Documentation

## Contents

| File | Description |
|------|-------------|
| `SYNTAX.md` | Complete syntax reference with EBNF grammar |
| `LLM.md` | LLM format overview, 3 formats (human/llm/machine), file types |
| `MACHINE.md` | Machine binary format specification |
| `API.md` | Rust API reference |

## Quick Reference

### 3 Formats

```
Human     key = value        (visually beautiful, for dx files and .sr)
LLM       key: value         (token efficient, for .llm files)
Machine   binary RKYV        (most performance, for .machine files)
```

### Separators

Both space and comma separators are supported and auto-detected per row.

```
Space    users[id name email]         (lower token count)
Comma    users[id,name,email]         (clearer with multi-word values)
```

### Examples

```
# Human — spaces around =
project(
  name    = dx-tree
  version = 1.0.0
)

# LLM — YAML-style colon
project:
  name: dx-tree
  version: 1.0.0

# Machine — binary (auto-generated)
.dx/serializer/config.machine
```

### Tables

```
# Space-separated
users[id name email](
  1 "Alice Johnson" alice@example.com
)

# Comma-separated (recommended for complex fields)
recipes[name,group,doc,script](
  build,all,"Build all workspace crates","cargo build"
)
```
