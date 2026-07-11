# DX Serializer LLM Format

Token-optimized text format for AI context windows.  
~49% token savings vs compact JSON, ~70% vs pretty-printed JSON.

---

## Three-Format Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Human Format  (.sr / .dx on disk) — visually beautiful     │
│  LLM Format    (.llm in .dx/serializer/) — token-optimized  │
│  Machine Format (.machine in .dx/serializer/) — binary RKYV  │
└─────────────────────────────────────────────────────────────┘
```

Human format is the source of truth on disk.  
LLM format is auto-generated for AI context windows.  
Machine format is auto-generated for zero-copy runtime access.

---

## File Extensions

| Extension | Type | Location |
|-----------|------|----------|
| `dx` | Human | Workspace root (extensionless, human-editable) |
| `.sr` | Human | Real disk or `.dx/serializer/` |
| `.llm` | LLM | `.dx/serializer/` |
| `.machine` | Machine (binary) | `.dx/serializer/` |

---

## Format by File Type

### `dx` extensionless file — Human format

Human-readable with spaces around `=`. Used as workspace config with optional `script()` section for task definitions.

```
project(
  name    = dx-tree
  version = 1.0.0
)

script(
  settings(
    shell    = bash
    fallback = true
  )
  recipes[name,group,doc,script](
    build,all,"Build all workspace crates","cargo build"
    test,all,"Run all workspace tests","cargo test"
  )
  aliases[name,target](
    b,build
    t,test
  )
)
```

### `.sr` file — Human or LLM

Human format for source files. LLM format is generated in `.dx/serializer/`.

```
project:
  name: dx-tree
  version: 1.0.0
```

### `.llm` file — LLM (token-optimized)

Compact single-line format for AI context windows.

```
project(name=dx-tree version=1.0.0)
```

---

## Best Practices

1. **Use Human for `dx` files** — humans edit these, readability matters
2. **Use LLM for AI context** — every token counts in context windows
3. **Use comma-separated tables** when doc strings contain spaces
4. **Use space-separated tables** for simple values (lower token count)
5. **Always quote multi-word strings** — never use underscores as space substitutes
6. **One row per line in tables** — parser expects `\n` between rows
