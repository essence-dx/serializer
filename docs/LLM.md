# DX Serializer LLM Format

Token-optimized text format for AI context windows.

---

## Three-Format Architecture

```
Human  (.sr / .dx on disk)  ─── source of truth, `()` parenthesized groups
Loose  (.loose)             ─── expanded `[section]` TOML-like
LLM    (.llm)               ─── token-optimized `:` YAML-style
Compact (.compact)          ─── single-line `()` minified
Machine (.machine)          ─── binary RKYV, zero-copy runtime access
```

---

## LLM Format

`: yml` multi-line style, auto-generated as `.llm`.

```
project:
  name: dx-os
  version: 1.0.0
```

Key characteristics:
- `:` separator after keys
- Indented sub-values
- Space-separated tokens
- Strings with spaces are quoted: `"Build all"`
