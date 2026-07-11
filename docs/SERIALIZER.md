# DX Serializer — Design Decisions

A reference for every design question about the DX Serializer.
Each row lists the scenario and candidate options. Remove wrong options to finalize.

---

## 1. Three-Format Architecture

| # | Scenario | Candidates |
|---|----------|------------|
| 1.1 | What are the 3 formats called? | A) High / Medium / Low  B) Human / LLM / Machine  C) Readable / Compact / Binary  D) Source / Token / Runtime |
| 1.2 | Which format is the source of truth on disk? | A) Human  B) LLM  C) Machine |
| 1.3 | Which format is stored in `.dx/serializer/*.llm`? | A) Human  B) LLM  C) Machine |
| 1.4 | Which format is stored in `.dx/serializer/*.machine`? | A) Human  B) LLM  C) Machine |
| 1.5 | Does the `dx` extensionless file use... | A) Human format  B) LLM format  C) Machine format |
| 1.6 | Does the `.sr` file use... | A) Human format  B) LLM format  C) Machine format |

## 2. CLI Commands

| # | Scenario | Candidates |
|---|----------|------------|
| 2.1 | What are the format subcommands? | A) `high medium low`  B) `human llm machine`  C) `readable compact binary` |
| 2.2 | Default command (`dx-serializer <file>`) should... | A) Auto-detect format  B) Default to human  C) Default to llm |
| 2.3 | `dx-serializer human <file>` should... | A) Validate & output human format  B) Convert to LLM  C) Generate both .llm and .machine |
| 2.4 | `dx-serializer llm <file>` should... | A) Generate only .llm output  B) Generate both .llm and .machine  C) Parse human and output LLM text |
| 2.5 | `dx-serializer machine <file>` should... | A) Generate only .machine output  B) Generate both .llm and .machine  C) Parse human and output machine binary |
| 2.6 | `--stdout` flag prints... | A) LLM format  B) Human format  C) Raw parsed data |

## 3. Table Separators

| # | Scenario | Candidates |
|---|----------|------------|
| 3.1 | Table header columns (`schema[name,group,...]`) should use... | A) Always comma separator  B) Always space separator  C) Auto-detect same as rows  D) Comma for complex, space for simple |
| 3.2 | Table rows with simple values (no spaces) should use... | A) Space separator (`b build`)  B) Comma separator (`b,build`)  C) Either, auto-detected |
| 3.3 | Table rows with sentences (values contain spaces) should use... | A) Comma separator with quotes (`build,all,"Build all","cargo build"`)  B) Comma separator no quotes (`build,all,Build all,cargo build`)  C) Space separator (values must not contain spaces) |
| 3.4 | Can different rows in the same table use different separators? | A) Yes, auto-detected per row  B) No, first row sets the style  C) No, header decides the style |
| 3.5 | If a value itself contains a comma (e.g. `"hello, world"`), how is it handled? | A) Must use space separator for that row  B) Must be quoted `"hello, world"`  C) Not allowed, restructure data |
| 3.6 | Empty/missing cell in a table row should be... | A) `null` keyword for space-separated, empty for comma-separated  B) Always `null` keyword  C) Just leave blank (consecutive delimiters)  D) Not allowed — all cells required |

## 4. Quoting Rules

| # | Scenario | Candidates |
|---|----------|------------|
| 4.1 | When using comma separator, values with spaces need... | A) No quotes — comma is the delimiter  B) Double quotes `"like this"`  C) Single quotes `'like this'` |
| 4.2 | When using space separator, values with spaces need... | A) Not allowed — must use comma separator instead  B) Double quotes `"like this"`  C) Single quotes `'like this'` |
| 4.3 | What character to use for string quotes? | A) Double quotes `"` only  B) Single quotes `'` only  C) Both, auto-detected  D) Backticks `` ` `` |
| 4.4 | Can quotes appear inside an unquoted comma-separated value? | A) Yes, quote is just a character  B) No, quote starts a quoted string  C) Escaped with backslash `\"` |
| 4.5 | Empty string value `""` in space-separated context should be... | A) `""` (explicit empty quotes)  B) `null` keyword  C) `~` tilde  D) `-` dash  E) Just empty (consecutive spaces) |

## 5. Group / Object Syntax

| # | Scenario | Candidates |
|---|----------|------------|
| 5.1 | A group with 3+ children uses... | A) Parenthesized `section(key = value ...)`  B) YAML-style `section: key = value` on multiple lines  C) Brackets `section { key = value }` |
| 5.2 | A group with 1-2 children uses... | A) Inline colon `section: key = value`  B) Same parenthesized syntax as 3+  C) Single-line parentheses `section(key = value)` |
| 5.3 | Nested groups should be... | A) Fully parenthesized `outer( inner( key = value ) )`  B) Dot-path `outer.inner.key = value`  C) YAML-indented `outer: inner: key: value` |
| 5.4 | Empty group (no children) should be... | A) `section()` with nothing inside  B) `section: null`  C) Omitted entirely |
| 5.5 | Alignment of `=` inside groups — spaces before `=` should... | A) Align to longest key in that group  B) Always 2 spaces  C) Always 1 space  D) No alignment, just 1 space before & after |

## 6. Key-Value Format

| # | Scenario | Candidates |
|---|----------|------------|
| 6.1 | Root-level key-value pairs use... | A) `key = value` with spaces around `=`  B) `key=value` no spaces  C) `key: value` with colon |
| 6.2 | Inside parenthesized groups, key-value pairs use... | A) `key = value` with spaces around `=`  B) `key=value` no spaces  C) `key: value` with colon |
| 6.3 | Inside inline colon groups, key-value pairs use... | A) `key = value` with spaces  B) `key=value` no spaces  C) `key: value` with colon |
| 6.4 | Multi-word unquoted values — allowed? | A) No, must be quoted  B) Yes, until end of line  C) Yes, until next key starts |

## 7. Data Types

| # | Scenario | Candidates |
|---|----------|------------|
| 7.1 | Booleans use... | A) `true` / `false` only  B) `true` / `false` / `yes` / `no`  C) `true` / `false` / `+` / `-` |
| 7.2 | Null values use... | A) `null` only  B) `null` / `none`  C) `null` / `~` / `-`  D) `null` / `none` / `~` / `-` |
| 7.3 | Numbers — integers vs floats? | A) Auto-detected: `42` is int, `3.14` is float  B) All stored as f64 internally  C) Integer if no decimal point |
| 7.4 | Number format — leading zeros? | A) `42` not `042`  B) Both allowed  C) `042` is octal |
| 7.5 | Negative numbers? | A) `-42` supported  B) Only positive  C) `-42` supported with space before `-` |
| 7.6 | Hex/octal/binary literals? | A) Not supported  B) `0xFF`, `0o77`, `0b11` supported  C) Only hex `0xFF` |

## 8. Arrays

| # | Scenario | Candidates |
|---|----------|------------|
| 8.1 | Inline arrays use... | A) Brackets `[item1 item2 item3]` space-separated  B) Brackets `[item1, item2, item3]` comma-separated  C) Either, auto-detected |
| 8.2 | Multi-line arrays use... | A) `key[n]:` line followed by `- item` lines  B) Brackets spanning multiple lines  C) Parenthesized with `items(...)` |
| 8.3 | Empty array? | A) `[]`  B) `key[0]:` with no items  C) Both |

## 9. Comments

| # | Scenario | Candidates |
|---|----------|------------|
| 9.1 | Single-line comments use... | A) `#` prefix  B) `//` prefix  C) `#` or `//`  D) `;` prefix |
| 9.2 | Multi-line comments? | A) Not supported  B) `#` on each line  C) `/* ... */` block comments |
| 9.3 | End-of-line comments? | A) Supported: `key = value  # comment`  B) Not supported  C) Only on their own line |

## 10. File Extensions & Locations

| # | Scenario | Candidates |
|---|----------|------------|
| 10.1 | Extensionless config file at workspace root is named... | A) `dx` only  B) `.dx` (hidden)  C) Both `dx` and `.dx` |
| 10.2 | Generated `.llm` files go to... | A) `.dx/serializer/`  B) Same directory as source  C) `.dx/llm/` |
| 10.3 | Generated `.machine` files go to... | A) `.dx/serializer/`  B) Same directory as source  C) `.dx/machine/` |
| 10.4 | What extension for human-readable source files? | A) `.sr` only  B) `.dx` (extensionless) only  C) Both `.sr` and extensionless `dx` |

## 11. Conversion Flow

| # | Scenario | Candidates |
|---|----------|------------|
| 11.1 | `dx serializer dx` reads `dx` file as... | A) Human format, generates .llm + .machine  B) LLM format, generates .machine  C) Autodetect and convert |
| 11.2 | When a `.sr` file is processed, the output is... | A) `.llm` and/or `.machine` in `.dx/serializer/`  B) Overwrite the `.sr` file  C) Print to stdout |
| 11.3 | Lossy or lossless conversion between formats? | A) Lossless — human ↔ llm ↔ machine roundtrip  B) Lossy — machine→human may lose some data  C) Mostly lossless, metadata may differ |

## 12. Edge Cases

| # | Scenario | Candidates |
|---|----------|------------|
| 12.1 | Single-row table — should use table syntax or flat key-value? | A) Table syntax always  B) Inline object if only 1 row  C) Both valid, user preference |
| 12.2 | Table with 0 rows? | A) `tname[cols]()` with nothing inside  B) Omitted entirely  C) `tname[cols]()` or omitted |
| 12.3 | Value containing both comma and spaces (e.g. `"hello, world"`)? | A) Use comma separator, no quotes — but comma inside value breaks it  B) Must use space separator with quotes  C) Escape with backslash `hello\, world` |
| 12.4 | Unicode/emoji in values? | A) Supported as-is (UTF-8)  B) Only ASCII  C) Supported with escaping `\uXXXX` |
| 12.5 | Maximum nesting depth? | A) No limit  B) 10 levels max  C) Configurable limit |

## 13. Table Header Syntax

| # | Scenario | Candidates |
|---|----------|------------|
| 13.1 | Table header separator — should `name group doc` or `name,group,doc`? | A) Space for simple col names, comma if col names contain spaces  B) Always space  C) Always comma  D) Auto-detect same as row separator |
| 13.2 | Table header location — before `()`? | A) `table[col1 col2](rows...)`  B) `table(col1 col2)[rows...]`  C) `table: col1 col2` then rows indented |
| 13.3 | Can a table have no header (no column names)? | A) No, headers always required  B) Yes, use empty brackets `table[](rows)`  C) Use numbers as default col names |
