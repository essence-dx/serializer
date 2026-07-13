# DX Loose Format

**Type:** Expanded TOML-like format  
**Extension:** `.loose`  
**Location:** Next to source file (auto-generated)  
**Purpose:** Extended human-readable representation with numbered sections

---

## Format

Loose format expands parenthesized groups into TOML-like `[section]` headers.  
Multi-row tables become numbered sub-sections.

```
[project]
name                         = dx-os
version                      = 1.0.0

[recipes:1]
name                         = build
group                        = all
doc                          = Build all workspace crates
script                       = cargo build --workspace

[recipes:2]
name                         = build-release
group                        = all
doc                          = Build all crates in release mode
script                       = cargo build --workspace --release
```

## Generation

```
dx-serializer human file.dx   # auto-generates file.loose alongside other outputs
```
