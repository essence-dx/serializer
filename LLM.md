# DX Serializer LLM Format

**Version:** 1.0 (Wrapped Dataframe)  
**Status:** Production-Ready  
**Token Efficiency:** 52–73% savings vs JSON, ~20% better than TOON  
**Extension:** `.llm`  
**Location:** Auto-generated in `.dx/serializer/`

---

## Overview

The DX LLM format is a **token-efficient, deterministically-parseable serialization format** designed specifically for AI context windows. It is the text-based member of DX Serializer's three-format architecture:

| Format | Extension | Location | Purpose |
|--------|-----------|----------|---------|
| **Human** | `.sr`, `.dx` | Real disk (version-controlled) | TOML/INI-like, human-editable, source of truth |
| **LLM** | `.llm` | `.dx/serializer/` (auto-generated) | Token-optimized for AI context windows |
| **Machine** | `.machine` | `.dx/serializer/` (auto-generated) | Pure RKYV binary, zero-copy deserialization |

---

## Core Philosophy

1. **Deterministic Parsing**: Wrapped structures `()` eliminate ambiguity — the parser knows exactly where tables start and end
2. **Natural Tokenization**: Quoted strings `"..."` preserve spaces without breaking BPE tokenization (underscores double token cost)
3. **Zero Structural Bloat**: Minimal delimiters — no commas, no indentation overhead, no repeated keys
4. **Schema-First Tables**: Define column schema once, repeat only data rows
5. **Mental Model Alignment**: `[]` for arrays, `()` for objects, `[headers](rows)` for tables — mirrors JSON mental models, reducing LLM hallucination

---

## Complete Syntax Reference

### 1. Root Key-Value Pairs (Scalars)

Simple values at the document root:

```
name = MyApp
version = 1.0.0
port = 8080
active = true
description = "Orchestrate dont just own your code"
```

**Rules:**
- One per line
- No spaces around `=`
- Use `"..."` for strings containing spaces
- Booleans: `true` / `false`
- Numbers: integers or floats (`42`, `3.14`)
- Null: `null`
- Type inference from context (parser detects numbers, booleans, strings automatically)

### 2. Arrays — Square Brackets `[]`

Space-separated lists of values:

```
tags = [rust, performance, serialization]
editors = [neovim, zed, vscode, cursor, antigravity, replit, "firebase studio"]
```

**Format:** `key=[item1 item2 item3]`

**Rules:**
- Items separated by spaces (not commas)
- Use `"..."` for multi-word items
- No commas between items
- Can be nested inside objects: `config(tags=[a b c])`

### 3. Inline Objects — Parentheses `()`

Key-value pairs enclosed in parentheses:

```
config(
  host = localhost
  port = 5432
  debug = true
)
server(
  url = "https://api.example.com"
  timeout = 30
)
driven(
  path = @/driven
)
```

**Format:** `key(key1=value1 key2=value2)`

**Rules:**
- Fields separated by spaces
- No spaces around `=` inside fields
- Use `"..."` for values with spaces
- Supports nested arrays: `key(items=[a b c])`
- Supports nested objects: `key(inner(key=val))`

### 4. Tables (Wrapped Dataframes) — `[headers](rows)`

The signature feature — deterministic, readable, token-efficient tabular data:

```
users[id, name, email](
1, Alice, alice@example.com
2, Bob, bob@example.com
3, Carol, carol@example.com
)
```

**Format:** `name[col1 col2 col3](rows...)`

**Rules:**
- Headers in `[]`, space-separated
- Rows wrapped in `()` for deterministic parsing
- Each row on its own line
- Fields within rows separated by spaces
- Use `"..."` for multi-word values in cells

**Example with quoted strings:**
```
employees[id, name, dept](
1, James Smith, Engineering
2, Mary Johnson, Research and Development
)
```

### 5. Prefix Elimination — `@prefix`

Removes repeated prefixes from table columns for massive token savings:

```
logs[timestamp, endpoint, status]@/api/(
10:23:45Z, users, 200
10:24:12Z, orders, 500
10:25:01Z, products, 200
)
```

**Expands to:**
- `10:23:45Z, /api/users, 200`
- `10:24:12Z, /api/orders, 500`
- `10:25:01Z, /api/products, 200`

**Format:** `name[headers]@prefix(...)` — the `@prefix` appears between headers `]` and opening paren `(`

**Savings:** 60–80% for columns with common prefixes (minimum 3 characters for prefix detection)

### 6. Compact Object Syntax — `name:count@=[key value key value]`

Ultra-compact key-value pairs without `=` signs; tokens are paired:

```
pkg:4@=[name dx version 1.0 type app]
```

**Format:** `name:count@=[key1 val1 key2 val2 ...]`

**Rules:**
- Requires even number of tokens (paired as key-value)
- No `=` between keys and values
- @= marker signals compact syntax

### 7. Reference System

Define reusable string references and reference them:

```
#: company=Acme Corp
#: api_url=https://api.example.com
proj(name=^company endpoint=^api_url)
```

**Format:**
- `#: ref_name=ref_value` — defines a reference
- `^ref_name` — references a defined value

**Behavior:**
- References are resolved via `LlmParser::resolve_refs()` which replaces `^key` with the resolved string
- Enables deduplication of repeated values for further token reduction

### 8. Document Section Structure

The LLM format internally organizes data into a structured `DxDocument`:

```
#c version=1.0
#: company=Acme Corp
#d(id,name,active)[
1,Alice,+
2,Bob,-
]
```

- `#c` — Context/config section (key-value pairs)
- `#:` — Reference definitions
- `#<letter>` — Data sections (each is a char ID mapped to a `DxSection` with schema and rows)

### 9. Section Dots — Preserved as-is

```
js.dependencies(
  next = 16.0.1
  react = 19.0.1
)
i18n.locales(
  path = @/locales
  default = en-US
)
```

Dots in section/object names are kept literally for clarity.

### 10. Entry Order Tracking

The document preserves insertion order via an `entry_order` vector of `EntryRef` variants:
- `EntryRef::Context(key)` — references a root key-value pair
- `EntryRef::Section(char_id)` — references a data section by its character ID

This ensures serialization output matches the original input order.

---

## Type System

### Primitives

| Type | Examples | Notes |
|------|----------|-------|
| String | `hello`, `"hello world"` | Unquoted if no spaces; doubled quotes otherwise |
| Number | `42`, `3.14`, `-7` | Single `f64` in data model; displays as integer if `.fract() == 0` |
| Boolean | `true`, `false` | Full words only |
| Null | `null` | |

### Collections

| Collection | Syntax | Example |
|------------|--------|---------|
| Array | `[item1 item2 item3]` | `tags=[rust performance]` |
| Object | `(key1=val1 key2=val2)` | `config(host=localhost port=8080)` |
| Table | `[headers](rows)` | `users[id name](1 Alice\n2 Bob)` |

### Value Enum (`DxLlmValue`)

```rust
pub enum DxLlmValue {
    Str(String),           // String value
    Num(f64),              // Numeric (integer or float)
    Bool(bool),            // Boolean
    Null,                  // Null
    Arr(Vec<DxLlmValue>),  // Array
    Obj(IndexMap<String, DxLlmValue>), // Object
    Ref(String),           // Reference pointer
}
```

### Data Section (`DxSection`)

```rust
pub struct DxSection {
    pub schema: Vec<String>,           // Column names
    pub rows: Vec<Vec<DxLlmValue>>,    // Row data matching schema length
}
```

---

## Abbreviation System

The LLM format uses a bicectional abbreviation dictionary (`AbbrevDict`) with **100+ mappings** for further token compression:

### Core Abbreviations

| Abbrev | Full | Domain |
|--------|------|--------|
| `nm` | name | Identity |
| `tt` | title | Identity |
| `ds` | description | Identity |
| `st` | status | State |
| `ac` | active | State |
| `en` | enabled | State |
| `cr` | created | Timestamps |
| `up` | updated | Timestamps |
| `ct` | count | Metrics |
| `pr` | price | Metrics |
| `em` | email | Contact |
| `ur` | url | Web |
| `pt` | path | Web |
| `cfg` | config | Project |
| `dep` | dependency | Project |

### Context-Aware Single-Letter Expansions

Ambiguous single-letter abbreviations are resolved by context:

| Abbrev | Context | Expands To |
|--------|---------|------------|
| `s` | `hikes` | sunny |
| `s` | `orders` | status |
| `s` | `config` | season |
| `w` | `images` | width |
| `w` | `products` | weight |
| `t` | `products` | type |
| `t` | `events` | time |
| `n` | `users` | name |
| `n` | `math` | number |
| `d` | `calendar` | date |
| `d` | `items` | description |
| `p` | `commerce` | price |
| `p` | `tasks` | priority |
| `v` | `software` | version |

---

## Configuration

```rust
pub struct SerializerConfig {
    /// Use legacy comma-separated format for arrays and schemas
    pub legacy_mode: bool,
    /// Enable @prefix elimination optimization for tables
    pub prefix_elimination: bool,
    /// Enable compact @= syntax for inline objects
    pub compact_syntax: bool,
}
```

- `legacy_mode` — defaults to `false` (space-separated); `true` enables comma-separated items
- `prefix_elimination` — defaults to `false`; detects and strips common column prefixes
- `compact_syntax` — defaults to `false`; enables `@=[key val]` object notation

---

## Parser Implementation

### Three Parsing Modes

1. **Root Scalar Mode** — `key=value`
   - Split by first `=`
   - If value is quoted `"..."`, read until closing quote
   - Otherwise, raw string until newline

2. **Inline Function Mode** — `key(param=val)` or `key=[list]`
   - Space ` ` as default delimiter between fields
   - Quotes `"..."` for strings with spaces
   - Objects `()` and arrays `[]` are distinguished by opener

3. **Table Block Mode** — `key[headers](rows)`
   - Triggered by `[` immediately followed by `(`
   - Headers inside `[]`, space or comma-separated
   - Rows inside `()`, newline-separated
   - Columns within rows separated by spaces

### Deterministic Parsing Logic

```
users[id name email](       ← see `[` → read headers until `]`
1 Alice alice@example.com   ← see `(` → start reading rows
2 Bob bob@example.com       ← read rows line by line
)                           ← see `)` → end table body
```

No guessing. No column counting. No blank line detection.

### Separator Detection

The parser auto-detects separators by scanning ahead:

- **Object separator**: Scans for `,` (legacy) vs ` ` (new) between `key=value` pairs
- **Row separator**: Scans for `,` `;` `:` or `\n` at depth 0 inside table data
- **Table column separator**: Auto-detects comma vs space per row

---

## Key Abbreviation Mappings by Domain

| Domain | # Mappings | Examples |
|--------|-----------|----------|
| Identity & Naming | 15 | nm, tt, ds, lb, al, uid, uuid, slug, hdl, nick, disp, abbr, code, ref |
| State & Status | 20 | st, ac, en, vs, lk, ar, dl, cp, pn, pub, drft, appr, rej, susp, exp, canc, proc, fail, succ, rdy |
| Timestamps & Dates | 20 | cr, up, dt, tm, ts, ex, du, yr, mo, dy, hr, mn, sec, ms, tz, utc, strt, end, schd, dln |
| Metrics & Numbers | 25 | ct, tl, am, pr, qt, rt, sc, rk, pct, avg, min, max, sum, med, std, var, idx, pos, ord, seq, num |
| Dimensions | 15 | wd, ht, sz, len, dp, wt, vol, area, rad, dia, cap, res, dpi, asp, scl |
| Web & Networking | 20 | ur, pt, lnk, src, dst, dom, api, ep, mth, hdr, bdy, qry, prm, rsp, req, ip, port, prot, ssl, cert |
| Contact & Personal | 15 | em, ph, ad, fn, lnm, cmp, dob, gen, bio, avt, prof, pref, lang, cntry, mob |
| Location & Geo | 15 | cy, co, rg, zp, la, lo, loc, geo, addr, st2, prov, dist, bldg, flr, unit |
| Visual & Media | 15 | cl, bg, fg, im, ic, th, vid, aud, fmt, mime, ext, fsize, bps, fps |
| Relations & Hierarchy | 20 | pa, ch, us, ow, au, ed, rv, asg, mb, gp, tea, org, dept, mgr, sup, sub, peer, anc, desc, sib |
| Classification | 15 | ca, tg, tp, vl, ky, md, lv, pri, vr, cls, kind, grp, tier, rank, flag |
| Project & Workspace | 15 | ws, repo, cont, ci, eds, proj, env, cfg, sett, opt, feat, mod, pkg, dep, lib |
| Commerce & Finance | 20 | sk, cu, sh, pd, inv, prd, dsc, tx, curr, bal, cred, deb, fee, sub, grt, pay, refnd, cart, chk, bill |
| Content & Text | 15 | txt, msg, cmt, nt, cnt, ft, para, sect, chap, art, post, reply, subj |
| Security & Auth | 15 | pwd, hash, salt, tok, sess, perm, role, auth, acl, enc, sig, key, sec, 2fa, otp |
| Data & Storage | 10 | db, tbl, col, row, rec, fld, blob, json, xml, csv |
| Lint & Code Quality | 10 | sev, fix, recom, fmtr, pfx, docs, warn, err, lint |

---

## EBNF Grammar

### LLM Format

```ebnf
document  = (scalar | object | array | table)* ;
scalar    = key "=" value ;
object    = identifier "(" pairs ")" ;
array     = key "=" "[" items "]" ;
table     = identifier "[" headers "]" "(" rows ")" ;
pairs     = pair (" " pair)* ;
pair      = key "=" value ;
headers   = identifier (" " identifier)* ;
rows      = row* ;
row       = value (" " value)* ;
items     = value (" " value)* ;
key       = identifier ;
value     = string | identifier | number ;
string    = '"' [^"]* '"' ;
identifier = [a-zA-Z_][a-zA-Z0-9_.-]* ;
number    = [0-9]+ ("." [0-9]+)? ;
```

### Human Format (for reference)

```ebnf
document         = (root_pair | section)* ;
root_pair        = key "=" value | key ":" array_items ;
section          = "[" identifier "]" section_content ;
section_content  = (pair | array_def)* ;
pair             = key "=" value ;
array_def        = key ":" array_items ;
array_items      = ("- " value)+ ;
key              = identifier ;
value            = string | identifier ;
string           = '"' [^"]* '"' ;
identifier       = [a-zA-Z_][a-zA-Z0-9_.-]* ;
```

---

## Conversion Rules

### LLM → Human

| LLM Format | Human Format |
|------------|-------------|
| `name(key=val)` | `[name]` section with `key = val` |
| `key=[item1 item2]` | `key:` followed by `- item` lines |
| `name[headers](rows)` | Transformed to aligned tabular representation |
| `key=value` | `key = value` (keys padded for alignment) |

### Human → LLM

| Human Format | LLM Format |
|-------------|------------|
| `[section]` with key=val | `section(key=val)` |
| `key:` with `- item` lines | `key=[item1 item2]` |
| All whitespace padding | Removed |
| Comments and empty lines | Stripped |

### LLM → Machine

- `DxLlmValue` → `DxValue` with type mapping:
  - `Str` ↔ `String`
  - `Num` ↔ `Int` or `Float`
  - `Bool` ↔ `Bool`
  - `Null` ↔ `Null`
  - `Arr` ↔ `Array`
  - `Obj` ↔ `Object`
  - `Ref` ↔ `Ref(usize)` (string key → numeric index)

---

## Complete Examples

### Application Configuration

```
author = essensefromexistence
version = 0.0.1
name = dx
description = "Orchestrate dont just own your code"
title = "Enhanced Developing Experience"
driven(
  path = @/driven
)
editors(
  default = neovim
  items = [neovim, zed, vscode, cursor, antigravity, replit, "firebase studio"]
)
forge(
  repository = "https://dx.vercel.app/essensefromexistence/dx"
  container = none
  pipeline = none
  tools = [cli, docs, examples, packages, scripts, style, tests]
)
dependencies[name, version](
dx-package-1, 0.0.1
dx-package-2, 0.0.1
)
js.dependencies(next = 16.0.1 react = 19.0.1)
```

### Package Dependencies

```
name = my-project
version = 2.0.0
deps[name, version](
react, 18.2.0
react-dom, 18.2.0
next, 14.0.1
typescript, 5.3.2
tailwindcss, 3.4.0
)
devDeps[name, version](
vitest, 1.2.0
eslint, 8.56.0
prettier, 3.2.0
)
```

### User Database

```
users[id, name, email, role, status](
1, Alice Johnson, alice@example.com, admin, active
2, Bob Smith, bob@example.com, user, active
3, Carol Williams, carol@example.com, user, inactive
4, Dave Brown, dave@example.com, moderator, active
)
```

### API Endpoints

```
api[name, method, path, auth]@/api/v1(
users, GET, /users, required
create user, POST, /users, admin
get user, GET, /users/:id, required
update user, PUT, /users/:id, admin
delete user, DELETE, /users/:id, admin
orders, GET, /orders, required
)

---

## Token Efficiency Analysis

### Structural Overhead Comparison

| Format | Overhead per Object | Overhead per Array | Overhead per Field |
|--------|---------------------|--------------------|--------------------|
| JSON | `{}` + `""` + `:` = 4 | `[]` + `,` = 2 | `"":` = 3 |
| TOON | Indentation + `-` = 3 | `-` per item = 1 | `:` = 1 |
| DX LLM | `()` = 2 | `[]` = 2 | `=` = 1 |

### Real-World Savings (measured on production configs)

| File Type | JSON Tokens | DX Tokens | Savings |
|-----------|-------------|-----------|---------|
| Package dependencies (50 items) | 420 | 112 | 73% |
| User database (100 rows) | 1,240 | 380 | 69% |
| API endpoints (25 items) | 310 | 145 | 53% |
| Config file (mixed data) | 180 | 85 | 53% |

**Average: 62% token savings vs JSON** (verified across Claude Sonnet 4, GPT-4o, Gemini 3)

### Comparison: Same Data in Three Formats

```json
// JSON: ~45 tokens
{"name":"MyApp","version":"1.0.0","tags":["rust","performance"],"users":[{"id":1,"name":"Alice","email":"alice@ex.com"},{"id":2,"name":"Bob","email":"bob@ex.com"}]}
```

```yaml
# TOON: ~35 tokens (22% savings)
name: MyApp
version: 1.0.0
tags:
  - rust
  - performance
users:
  - id: 1
    name: Alice
    email: alice@ex.com
  - id: 2
    name: Bob
    email: bob@ex.com
```

```
# DX LLM: ~22 tokens (51% savings vs JSON, 37% vs TOON)
name = MyApp
version = 1.0.0
tags = [rust, performance]
users[id, name, email](
1, Alice, alice@ex.com
2, Bob, bob@ex.com
)
```

---

## Best Practices

### DO
- Use `"..."` for multi-word strings (standard, predictable, robust)
- Use wrapped dataframes `[headers](rows)` for tabular data
- Use `[]` for arrays, `()` for objects (mental model alignment)
- Enable prefix elimination for columns with repeated prefixes
- Let the parser infer types from values

### DON'T
- Never replace spaces with underscores (doubles token cost in BPE tokenizers)
- Don't omit quotes for multi-word strings (causes parsing ambiguity)
- Don't use without wrapped dataframes (old format has parsing ambiguity)
- Don't manually add row counts (the serializer calculates these)
- Don't use for prose-heavy content (TOON or plain text is better for documentation)

---

## Limitations

1. **Not human-editable directly** — Use `.sr` (human format) for editing, `.llm` is auto-generated for AI consumption
2. **Requires schema for tables** — All rows must have the same number of columns
3. **No comments in LLM format** — Use the human format for inline documentation
4. **Best for structured/repetitive data** — Token savings diminish for prose-heavy content

---

## Why This Is The Final Form

### 1. Deterministic Parsing (Safety)
By wrapping table rows in `(...)`, the parser knows exactly where the table starts and ends. No guessing based on column counts or blank lines.

- **Start:** `users[headers](`
- **End:** `)`

### 2. Token Neutrality
Swapping semicolons `;` for newlines `\n`:
- In BPE tokenizers: `;` = 1 token, `\n` = 1 token
- **Net cost:** Zero
- **Net gain:** Massive readability improvement

### 3. Quoting Standard
Using `"Blue Lake Trail"` explicitly acknowledges that spaces inside columns require quotes. No underscore magic, no ambiguity. Standard, predictable, robust.

---

## Implementation Detail: Serializer Output

The `LlmSerializer` produces output with these behaviors:

- Objects `DxLlmValue::Obj` → `name(key1=val1 key2=val2)` with space separators
- Arrays `DxLlmValue::Arr` → `name=[item1 item2 item3]` with space separators
- Tables → `name[col1 col2](row1\nrow2)` with newline-separated rows inside `()`
- Strings with spaces → automatically quoted with `"`
- Booleans → `true` / `false`
- Null → `null`
- References → `^key`
- Entry order preserved from `entry_order` vector
- Prefix elimination via `@prefix` markers (configurable)

The `LlmParser` handles all of the above plus:
- Auto-detection of space vs comma separators
- Legacy comma-separated format support
- Inline separated row formats (`,`, `;`, `:`)
- Compact `@=[]` object syntax
- Reference resolution via `resolve_refs()`

---

**License:** MIT / Apache-2.0  
**Documentation generated from:** `LLM_FORMAT_SPEC.md`, `docs/SYNTAX.md`, `src/llm/serializer.rs`, `src/llm/parser.rs`, `src/llm/types.rs`, `src/llm/abbrev.rs`
