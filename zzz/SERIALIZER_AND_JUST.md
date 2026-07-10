# DX Serializer & Just Integration — Open Problems

This document describes all the problems blocking integration between the `dx-serializer` crate and the `just` command runner fork. Another AI should solve these.

---

## 1. Parser: `parse_parenthesized_object` — No Nested Structures

**File:** `src/llm/parser.rs:1379-1428`

**Problem:** Inside `name(...)`, only flat `key=value` pairs are accepted. Nested objects `subname(...)` and wrapped tables `subname[headers](rows)` are rejected with `UnexpectedChar`.

**What we want:**

```dx
script(
  settings(shell="pwsh.exe" fallback=true)
  recipes[name group script](
    build dev "cargo build"
    test test "cargo test"
  )
  aliases[name target](
    t test
    b build
  )
)
```

**What fails:** When parsing inside `script(...)`, `parse_parenthesized_object` sees `settings` as a key, then expects `=` but finds `(` → `UnexpectedChar { ch: '(' }`.

**Required fix:** `parse_parenthesized_object` must detect and recursively handle:
- `key=(subkey=val subkey2=val2)` — nested object
- `key[headers](rows)` — nested wrapped table
- `key=[item1 item2]` — nested array (currently only `key=[items]` where `=[` is consumed as array at line 1413, but this is inside the value branch after `=`; we need to detect `key[headers](` without preceding `=`)

---

## 2. Serializer: Nested `Obj` Outputs OLD Bracket Format

**File:** `src/llm/serializer.rs:305-311`

**Bug:** When serializing a nested `DxLlmValue::Obj` inside `serialize_value()`, the output uses OLD bracket+comma format:

```rust
DxLlmValue::Obj(fields) => {
    format!("[{}]", fields_str.join(","))  // OLD: [key=val,key2=val2]
}
```

But for top-level `Obj`, `serialize_context_entry()` correctly uses NEW paren+space format:

```rust
// serializer.rs:193-197
format!("{}({})", key, fields_str.join(self.inline_value_separator()))  // NEW: name(key=val key2=val2)
```

**Impact:** Round-trip fails for nested objects. If you serialize `{ outer: Obj({ inner: Obj({a: "b"}) }) }`, the output has `outer(inner=[a=b])`. When re-parsed, `parse_parenthesized_object` treats `=[a=b]` as an array of strings `["a=b"]`, NOT as nested object `Obj({a: "b"})`.

**Required fix:** Change line 311 from `format!("[{}]", fields_str.join(","))` to `format!("({})", fields_str.join(" "))` — use parens and space separator, consistent with top-level format. BUT this must be coordinated with the parser fix in problem #1, since the parser currently can't parse `key=(subkey=val)` inside parenthesized objects.

---

## 3. Parser: Values Inside Parenthesized Objects Stop at First Space

**File:** `src/llm/parser.rs:1419-1421`

**Bug:** Inside `parse_parenthesized_object`, values are parsed with:

```rust
let value_str = self.parse_until_delimiter(&[' ', ')', '\n'])?;
```

This stops at the FIRST space, so `task = Our favorite hikes together` only captures `Our` as the value. The remaining tokens `favorite hikes together` then cause parse errors.

**What we want:**

```dx
context(
  task = Our favorite hikes together
  location = Boulder
  season = spring_2025
)
```

Multi-word values without quotes inside parenthesized objects.

**Possible fixes:**
- Instead of stopping at space, read until `)` or newline, then split on ` = ` pattern to separate keys from values. But this breaks having space-separated fields on one line (`key=val key2=val2`).
- Use a heuristic: if a line inside `(...)` has `key=` followed by text, the value extends to end-of-line (or `)`).
- Or require quotes for multi-word values but make the error message clear.

---

## 4. Parser: Comma-Separated Arrays Without Brackets Don't Work at Root

**File:** `src/llm/parser.rs:409-427`

**Bug:** At root level with `=`, the value delimiter list includes `,`:

```rust
let value = self.parse_value_until_delimiter(&[',', '\n', '\r', ']'])?;
```

So `friends = ana,luis,sam` only captures `ana`. The `,` stops the value reader, then `luis` is left as garbage input causing an error.

**What we want:**

```dx
friends = ana,luis,sam
// or
friends=ana,luis,sam
```

Parsed as `Arr(["ana", "luis", "sam"])`.

**Note:** The `name=[ana luis sam]` format (brackets, space-separated) already works. This is about the no-bracket comma-separated variant.

**Possible fixes:**
- After reading a value at root level, peek ahead: if the next non-whitespace char is `,`, treat the entire thing as a comma-separated array.
- Or: when `=` at root level, scan the full line first. If it contains commas, parse as comma-separated array. If it contains spaces, parse as single string.

---

## 5. Parser: Wrapped Dataframe Rows Split on Spaces, Not Commas

**File:** `src/llm/parser.rs:1287-1376`

**Bug:** `parse_wrapped_dataframe_rows` → `parse_wrapped_row` splits columns on space characters (line 1345):

```rust
' ' if !in_quotes => { /* space separates values */ }
```

So comma-separated values in wrapped rows fail:

```dx
hikes[id name distanceKm elevationGain companion wasSunny](
  1,Blue Lake Trail,7.5,320,ana,true    ← parsed as ["1,Blue", "Lake", "Trail,7.5,320,ana,true"] (3 cols, needs 6)
)
```

**What we want:** Auto-detect comma vs space separator in wrapped dataframe rows. If the first row contains commas, parse as comma-separated. Otherwise, space-separated.

**Note:** There's already a separate path for `parse_table` (OLD format with `:n(schema)[rows]`) that handles comma-separated rows via `parse_inline_separated_rows` at line 1049-1053. But the NEW wrapped format `name[headers](rows)` only handles space-separated rows.

**Required fix:** In `parse_wrapped_dataframe_rows`, scan the first row to detect separator (comma vs space), then use the appropriate split logic.

---

## 6. Serializer: Table Output Should Support Both Space and Comma Modes

**File:** `src/llm/serializer.rs:200-235`

**Problem:** Currently `serialize_section_with_name` always outputs space-separated columns:

```rust
let values: Vec<String> = row.iter().map(|v| self.serialize_table_value(v)).collect();
output.push_str(&values.join(" "));
```

There's no option for comma-separated output.

**Required fix:** Add a config option — maybe `SerializerConfig { table_separator: char }` — that defaults to `' '` but can be set to `','` for comma-separated table output.

---

## 7. Parser/Serializer: Inconsistent Separator Heuristics

**Files:** `src/llm/parser.rs`, `src/llm/serializer.rs`

**Problem:** The parser auto-detects separators in multiple places (`detect_object_separator`, `detect_row_separator`, `parse_quoted_items`) but the serializer always hard-codes space separators. This leads to situations where the parser accepts a format that the serializer never produces (e.g., comma-separated inline objects `config[host=localhost,port=8080]`).

**Required fix:** Either:
- Remove all legacy comma-separated parsing paths and make the parser strictly space-separated (NEW format only), or
- Add a `SerializerConfig` option to control which separator the serializer produces, with round-trip tests for both modes.

---

## 8. Just Integration: `dx_loader` Module (New File)

**Target:** `G:\Dx\script\src\dx_loader.rs` (in the `just` fork)

**What it needs to do:**

1. Read a `dx` file (extensionless, DX Serializer LLM format)
2. Parse it via `serializer::llm_to_document()`
3. Find recipe-related entries at the document root:
   - `recipes[name script deps params group doc private]` — a wrapped dataframe table
   - `aliases[name target]` — a wrapped dataframe table
   - `settings(...)` — a parenthesized object with shell, fallback, etc.
   - `vars[name value export]` — a wrapped dataframe table for assignments
4. Convert them to `just`'s internal types (`Recipe`, `Assignment`, `Alias`, `Settings`)
5. Return a `Compilation` struct equivalent to what `just`'s own parser produces

**Signature:**

```rust
pub fn load_dx(src: &str, path: &Path) -> Result<Compilation, Error>
```

**Dependencies to add to `Cargo.toml`:**
```toml
dx-serializer = { path = "../serializer" }
```

---

## 9. Just Integration: File Search Changes

**File:** `G:\Dx\script\src\search.rs` (in the `just` fork)

**Current** (line 4):
```rust
pub(crate) const JUSTFILE_NAMES: [&str; 2] = ["justfile", ".justfile"];
```

**Required change:** Add `"dx"` as a recognized filename:
```rust
pub(crate) const JUSTFILE_NAMES: [&str; 3] = ["justfile", ".justfile", "dx"];
```

Also, when a `dx` file is found, the just fork should route to `dx_loader` instead of the standard parser.

---

## 10. Just Integration: Compiler Routing

**File:** `G:\Dx\script\src\compiler.rs` (in the `just` fork)

**Current:** `Compiler::compile()` calls `Parser::parse_source()` which parses just's own DSL.

**Required change:** Before calling `Parser::parse_source()`, check if the source file is named `dx` (or if the source starts with DX Serializer LLM format patterns). If so, route to `dx_loader::load_dx()` instead.

**Detection heuristic:** If the filename is `dx` (no extension), or if the first non-comment line matches `key=value` or `name(...)` patterns, use the DX serializer parser.

---

## 11. Just Integration: Template `dx` File for `--init`

**File:** `G:\Dx\script\src\subcommand.rs` (in the `just` fork)

**Current** (line 3-8):
```rust
pub const INIT_JUSTFILE: &str = "\
# https://just.systems

default:
    echo 'Hello, world!'
";
```

**Required change:** Add `INIT_DX` template:

```rust
pub const INIT_DX: &str = "\
settings(shell=\"bash\" fallback=true)

recipes[name group doc script](
  default misc \"Hello, world!\" \"echo 'Hello, world!'\"
)

aliases[name target]()
";
```

And route `just --init` to generate this when not in a `justfile`-style project.

---

## 12. Just Integration: CLI Flag for `--dx`

**File:** `G:\Dx\script\src\arguments.rs` or similar (in the `just` fork)

**Required change:** Add `--dx` flag as an alias for `--justfile`, and allow `just --dx` to auto-discover the `dx` file. Or make `just` recognize `dx` files by default alongside `justfile`/`.justfile`.

---

## Summary of All Problems

| # | Area | Problem | Impact | Priority |
|---|------|---------|--------|----------|
| 1 | Parser | `parse_parenthesized_object` rejects nested `()` and `[]()` | Can't have `script(recipes[...](...))` | Critical |
| 2 | Serializer | Nested `Obj` outputs `[key=val,key2=val2]` instead of `(key=val key2=val2)` | Round-trip failure for nested objects | High |
| 3 | Parser | Values inside `()` stop at first space | Multi-word unquoted values fail | High |
| 4 | Parser | Comma-separated arrays without `[]` fail | `friends = a,b,c` not supported | Medium |
| 5 | Parser | Wrapped dataframe rows don't support comma separator | `1,Blue Lake,7.5` splits wrong | High |
| 6 | Serializer | No option for comma-separated table output | Inconsistent with comma parsing support | Medium |
| 7 | Both | Inconsistent separator heuristics between parser and serializer | Parser accepts formats serializer never produces | Medium |
| 8 | Just | Need new `dx_loader.rs` module | Core integration missing | Critical |
| 9 | Just | Need to add `"dx"` to `JUSTFILE_NAMES` | File discovery doesn't find `dx` | High |
| 10 | Just | Need compiler routing to `dx_loader` | Parser selection logic missing | High |
| 11 | Just | Need `INIT_DX` template | `--init` doesn't generate dx file | Low |
| 12 | Just | Need `--dx` CLI flag | No explicit dx file selection | Low |

---

## Recommended Fix Order

1. **First:** Parser problem #1 (nested structures in parenthesized objects) — unblocks the entire `script(...)` syntax
2. **Second:** Parser problem #3 (multi-word values in parens) — needed for recipe scripts with spaces
3. **Third:** Parser problem #5 (comma-separated wrapped rows) — needed for user's preferred table format
4. **Fourth:** Serializer problem #2 (nested Obj output format) — ensures round-trip correctness
5. **Then:** Just integration #8-10 (dx_loader, search, compiler)
6. **Finally:** Lower priority items #4, #6, #7, #11, #12

---

## Test File

The existing test at `G:\Dx\serializer\tests\test_working_format.rs` demonstrates what currently works. The file `G:\Dx\serializer\tests\test_user_format.rs` demonstrates the user's desired format (which currently fails).
