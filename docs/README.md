# DX Serializer Documentation

## Contents

| File | Description |
|------|-------------|
| `SYNTAX.md` | Complete syntax reference |
| `HUMAN.md` | Human format (Normal flavor) |
| `LOOSE.md` | Human format (Loose flavor — expanded `[section]` style) |
| `LLM.md` | LLM format (Normal flavor — `:` yml style) |
| `COMPACT.md` | LLM format (Compact flavor — single-line minified) |
| `MACHINE.md` | Machine binary format (RKYV) |
| `SERIALIZER.md` | Design decisions and options |

## Quick Reference

### 3 Formats, 2 Flavors Each

```
Human (source of truth)
  ├── Normal   () aligned =     — hand-editable dx / .sr files
  └── Loose    [section] TOML   — expanded, generated as dx.loose

LLM (token-optimized)
  ├── Normal   : yml multi-line — generated as .llm
  └── Compact  () single-line   — minified, generated as dx.compact

Machine (binary performance)
  └── RKYV binary               — generated as .machine
```

### CLI Commands

| Command | Action |
|---------|--------|
| `dx-serializer human <file>` | Process as Human format, generate .llm + .machine + .loose + .compact |
| `dx-serializer llm <file>` | Generate only .llm output |
| `dx-serializer machine <file>` | Generate only .machine output |
| `--compact` | Output compact LLM flavor |
| `--stdout` | Print to stdout instead of writing files |
