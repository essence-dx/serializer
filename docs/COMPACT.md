# DX Compact Format

**Type:** Minified text format  
**Extension:** `.compact`  
**Location:** Next to source file (auto-generated)  
**Purpose:** Most token-efficient text representation

---

## Format

Compact format puts everything in single-line `()` blocks with no spaces around `=`.

```
project(name=dx-os version=1.0.0)
build(compiler=gcc std=c++23 opt=-O2 static=true)
recipes[name group doc script](build all "Build all workspace crates" "cargo build --workspace" ...)
```

## Rules

- `()` for all objects, single line
- `key=value` without spaces around `=`
- No newlines within a section
- Space between sections
- Tables: `table[cols](row1 row2)` — rows space-separated
- Strings with spaces are quoted: `"Build all"`

## Generation

```
dx-serializer llm file.dx --compact          # generates file.compact
dx-serializer human file.dx                  # auto-generates file.compact alongside other outputs
```
