# DX Serializer Documentation

## Contents

| File | Description |
|------|-------------|
| `SYNTAX.md` | Complete syntax reference |
| `HUMAN.md` | Human format — parenthesized groups with aligned `=` |
| `LOOSE.md` | Loose format — expanded `[section]` TOML-like style |
| `LLM.md` | LLM format — `:` YAML-style multi-line |
| `COMPACT.md` | Compact format — single-line parenthesized, minified |
| `MACHINE.md` | Machine binary format (RKYV) |
| `SERIALIZER.md` | Design decisions and options |

## Quick Reference

### 5 Formats

```
Human (source of truth, hand-editable)
  └── scripts(...)    parenthesized groups, aligned =

Loose (expanded TOML-like)
  └── [scripts]       section headers, key = value

LLM (token-optimized)
  └── scripts:        YAML-style multi-line

Compact (minified)
  └── scripts(...)    single-line parenthesized

Machine (binary)
  └── RKYV binary     zero-copy runtime
```

### CLI Commands

| Command | Action |
|---------|--------|
| `dx-serializer human <file>` | Process as Human format, generate .llm + .machine + .loose + .compact |
| `dx-serializer llm <file>` | Generate only .llm output |
| `dx-serializer machine <file>` | Generate only .machine output |
| `--compact` | Output compact flavor |
| `--stdout` | Print to stdout instead of writing files |
