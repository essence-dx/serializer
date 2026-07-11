# DX Serializer — Design Decisions

My answer letter in last column. Tell me which #s to change.

---

## 1. Three-Format Architecture

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 1.1 | What are the 3 formats called? | A) High/Medium/Low  B) Human/LLM/Machine  C) Readable/Compact/Binary | **B** |
| 1.2 | Which format is source of truth on disk? | A) Human  B) LLM  C) Machine | **A** |
| 1.3 | Which format goes in `.dx/serializer/*.llm`? | A) Human  B) LLM  C) Machine | **B** |
| 1.4 | Which format goes in `.dx/serializer/*.machine`? | A) Human  B) LLM  C) Machine | **C** |
| 1.5 | What format does the `dx` extensionless file use by default? | A) Human  B) LLM  C) Machine | **A** |
| 1.6 | What format does the `.sr` file use by default? | A) Human  B) LLM  C) Machine | **A** |
| 1.7 | Can `.sr` and `dx` files contain LLM format too? | A) Yes auto-detect  B) No strict per extension  C) Only human | **A** |
| 1.8 | What determines human vs llm parsing? | A) Auto-detect by content  B) File extension  C) CLI flag | **A** |
| 1.9 | Does LLM format have two flavors (normal + compact)? | A) Yes  B) No only one  C) Three flavors | **A** |
| 1.10 | Normal LLM uses what syntax? | A) `:` yml-style multi-line  B) `()` multi-line  C) `key=value` flat | **A** |
| 1.11 | Compact LLM uses what syntax? | A) `()` single-line no newlines  B) `:` single-line  C) `key=value` flat | **A** |
| 1.12 | Compact LLM is like what in JSON world? | A) JSONC (minified, no whitespace)  B) Pretty-printed JSON  C) YAML | **A** |

## 2. CLI Commands

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 2.1 | What format subcommands should the CLI have? | A) high/medium/low  B) human/llm/machine  C) readable/compact/binary | **B** |
| 2.2 | Default `dx-serializer <file>` should do what? | A) Auto-detect  B) Parse as human, generate .llm+.machine  C) Default to llm | **B** |
| 2.3 | `dx-serializer human <file>` should do what? | A) Validate & output human  B) Convert to LLM  C) Generate both .llm+.machine | **C** |
| 2.4 | `dx-serializer llm <file>` should do what? | A) Generate .llm (normal flavor)  B) Generate both .llm+.machine  C) Parse human output LLM text | **A** |
| 2.5 | `dx-serializer machine <file>` should do what? | A) Generate only .machine  B) Generate both .llm+.machine  C) Parse human output machine binary | **A** |
| 2.6 | `--stdout` should print what? | A) LLM format (normal flavor)  B) Human format  C) Raw parsed data | **A** |
| 2.7 | Should there be a `--compact` flag for LLM output? | A) Yes outputs compact flavor  B) No always normal  C) Separate subcommand | **A** |
| 2.8 | `dx-serializer llm --compact <file>` outputs? | A) Compact LLM (single-line `()`)  B) Normal LLM  C) Machine | **A** |

## 3. Table Separators

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 3.1 | Header columns in `table[header here]` should use what separator? | A) Always comma  B) Always space  C) Auto-detect  D) Space if simple comma if spaces | **D** |
| 3.2 | Rows with no spaces in any value should use? | A) Space (`b  build`)  B) Comma (`b,build`)  C) Either auto-detected | **A** |
| 3.3 | Rows with sentences/values containing spaces should use? | A) Comma with quotes (`"Build all"`)  B) Comma no quotes (`Build all`)  C) Space not allowed | **B** |
| 3.4 | Can different rows in same table use different separators? | A) Yes auto-detected per row  B) No first row decides  C) No header decides | **A** |
| 3.5 | If a value itself contains a comma, how to handle? | A) Use space separator for that row  B) Must be quoted  C) Not allowed | **A** |
| 3.6 | How to represent an empty/missing cell in a row? | A) `null` for space empty for comma  B) Always `null` keyword  C) Consecutive delimiters  D) All cells required | **B** |

## 4. Quoting Rules

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 4.1 | With comma separator do values with spaces need quotes? | A) No — comma is the delimiter  B) Yes double quotes  C) Yes single quotes | **A** |
| 4.2 | With space separator how to handle values with spaces? | A) Not allowed — switch to comma separator  B) Use double quotes  C) Use single quotes | **A** |
| 4.3 | What quote character to use? | A) Double `"` only  B) Single `'` only  C) Both  D) Backtick `` ` `` | **A** |
| 4.4 | Inside an unquoted comma-separated value is `"` just a character? | A) Yes quote is just a char  B) No starts a quoted string  C) Escaped with `\"` | **A** |
| 4.5 | How to represent an empty string in space-separated context? | A) `""`  B) `null`  C) `~`  D) `-`  E) Consecutive spaces | **B** |

## 5. Group / Object Syntax

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 5.1 | In `dx` extensionless files groups use what syntax? | A) `()` parens style  B) `:` yml style  C) Both | **A** |
| 5.2 | In `.sr` files groups use what syntax by default? | A) `()` parens style  B) `:` yml style  C) Both | **B** |
| 5.3 | In LLM normal flavor groups use what syntax? | A) `()` parens multi-line  B) `:` yml style  C) Flat `key=value` | **B** |
| 5.4 | In LLM compact flavor groups use what syntax? | A) `()` single-line no newlines  B) `:` single-line  C) Flat `key=value` | **A** |
| 5.5 | Nested groups should use? | A) Nested parens `outer( inner( ) )`  B) Dot-path `outer.inner.key`  C) YAML-indented | **A** |
| 5.6 | Empty group (no children) should be? | A) `section()` / `section:`  B) `section: null`  C) Omitted entirely | **A** |
| 5.7 | `=` alignment inside groups? | A) Align to longest key  B) Always 2 spaces before `=`  C) Always 1 space  D) No alignment | **A** |

## 6. Key-Value Format

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 6.1 | Root-level key-value pairs use? | A) `key = value` spaces  B) `key=value` no spaces  C) `key: value` colon | **A** |
| 6.2 | Inside parenthesized groups key-value pairs use? | A) `key = value` spaces  B) `key=value` no spaces  C) `key: value` colon | **A** |
| 6.3 | Inside yml-style groups key-value pairs use? | A) `key = value` spaces  B) `key=value` no spaces  C) `key: value` colon | **C** |
| 6.4 | Inside LLM compact `()` groups key-value pairs use? | A) `key = value` spaces  B) `key=value` no spaces  C) `key: value` colon | **B** |
| 6.5 | Multi-word unquoted values — allowed? | A) No must be quoted  B) Yes until end of line  C) Yes until next key | **A** |

## 7. Data Types

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 7.1 | What boolean keywords are supported? | A) `true`/`false` only  B) +`yes`/`no`  C) +`+`/`-` | **A** |
| 7.2 | What null keywords are supported? | A) `null` only  B) +`none`  C) +`~`/`-` | **A** |
| 7.3 | How are numbers handled? | A) Auto int vs float  B) All f64 internally  C) Int if no decimal point | **C** |
| 7.4 | Are leading zeros allowed? | A) No `42` not `042`  B) Both  C) `042` is octal | **A** |
| 7.5 | Are negative numbers supported? | A) Yes `-42`  B) Only positive  C) With space before `-` | **A** |
| 7.6 | Are hex/octal/binary literals supported? | A) No  B) Yes `0xFF` etc  C) Only hex | **A** |

## 8. Arrays

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 8.1 | Inline arrays use what syntax? | A) `[a b c]` space-separated  B) `[a,b,c]` comma-separated  C) Either auto-detected | **C** |
| 8.2 | Multi-line arrays use what syntax? | A) `key[n]:` then `- item` lines  B) Brackets spanning lines  C) `items(...)` parens | **A** |
| 8.3 | Empty array syntax? | A) `[]`  B) `key[0]:` no items  C) Both | **A** |

## 9. Comments

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 9.1 | What character for single-line comments? | A) `#`  B) `//`  C) `#` or `//`  D) `;` | **A** |
| 9.2 | Are multi-line comments supported? | A) No use `#` each line  B) `/* */` blocks  C) `#` on each line is fine | **C** |
| 9.3 | Are end-of-line comments supported? | A) Yes `key = val  # comment`  B) No  C) Own line only | **A** |

## 10. File Extensions & Locations

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 10.1 | What name for extensionless config at root? | A) `dx` only  B) `.dx` hidden  C) Both | **A** |
| 10.2 | Where do generated `.llm` files go? | A) `.dx/serializer/`  B) Same dir as source  C) `.dx/llm/` | **A** |
| 10.3 | Where do generated `.machine` files go? | A) `.dx/serializer/`  B) Same dir as source  C) `.dx/machine/` | **A** |
| 10.4 | What extensions use human format by default? | A) `.sr` only  B) `dx` only  C) Both `.sr` and `dx` | **C** |
| 10.5 | What extension uses LLM format by default? | A) `.llm`  B) `.sr`  C) `dx` | **A** |

## 11. Conversion Flow

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 11.1 | `dx serializer dx` reads dx file as what and outputs what? | A) Human → .llm (normal) + .machine  B) LLM → .machine  C) Autodetect | **A** |
| 11.2 | Processing a `.sr` file outputs to where? | A) `.llm`/`.machine` in `.dx/serializer/`  B) Overwrite `.sr`  C) Stdout | **A** |
| 11.3 | Is conversion between formats lossless? | A) Yes full roundtrip  B) No machine→human loses data  C) Mostly | **A** |
| 11.4 | Does compact LLM also go in `.dx/serializer/`? | A) Yes alongside normal .llm  B) Separate file  C) Only one flavor per file | **A** |

## 12. Edge Cases

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 12.1 | Single-row table — table syntax or flat? | A) Table syntax always  B) Inline object  C) Both | **A** |
| 12.2 | Table with 0 rows? | A) `tname[cols]()`  B) Omitted  C) Either | **C** |
| 12.3 | Value with both comma AND spaces (e.g. `hello, world`)? | A) Use space-sep with quotes `"hello, world"`  B) Escape `\,`  C) Not allowed | **A** |
| 12.4 | Unicode/emoji in values? | A) UTF-8 as-is  B) ASCII only  C) `\uXXXX` escape | **A** |
| 12.5 | Max nesting depth? | A) No limit  B) 10 levels  C) Configurable | **A** |

## 13. Table Header Syntax

| # | Question | Options | Ans |
|---|----------|---------|-----|
| 13.1 | Header separator: `[name group]` or `[name,group]`? | A) Space if simple comma if spaces  B) Always space  C) Always comma  D) Auto-detect | **A** |
| 13.2 | Header position relative to `()` rows? | A) `table[cols](rows)`  B) `table(cols)[rows]`  C) `table: cols` then rows | **A** |
| 13.3 | Can a table have no column names? | A) No always required  B) Yes `table[](...)`  C) Default numbered cols | **A** |
