# DX Human Format

**Type:** Text (readable, hand-editable)  
**Extension:** `.sr`, `dx` (extensionless)  
**Location:** On disk (source of truth)

---

## Format — `()` with aligned `=`

The default human-readable format used in hand-edited files.

```
project(
  name    = dx-os
  version = 1.0.0
)

scripts(
  settings(
    shell = bash
  )
)
```

Key characteristics:
- Parenthesized groups `name(...)` for objects
- Aligned `=` signs inside groups
- Nested groups for sub-objects

---

## Separator Rules

| Separator | When to use | Example |
|-----------|-------------|---------|
| **Space** | Values have no spaces (simple names, flags) | `aliases[name target]( b build )` |
| **Comma** | Values contain spaces (sentences, paths) | `recipes[name group doc script]( build,all,Build all,cargo build )` |

- Header columns use space separator (column names are simple identifiers)
- Rows auto-detect: comma if field count matches schema, else space
- No `""` needed with comma separator — comma is the field boundary
