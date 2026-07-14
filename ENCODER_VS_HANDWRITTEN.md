# Encoder vs Hand-Written DX: All Differences

> Every difference between what `@dx-serializer/core` encoder produces and what the
> hand-written `.dx` benchmark files use. The hand-written format is more compact
> and is what `dx-token optimize` prefers.

---

## Summary of All Differences

| # | Difference | Hand-Written (compact) | Encoder (current) | Impact | Source File |
|---|-----------|----------------------|-------------------|--------|-------------|
| 1 | **Object block style** | Inline on one line: `key(k=v k=v)` | Multi-line with indent: `key(\n  k=v\n)` | **Major** — biggest token saver | `encoders.ts:42-44` |
| 2 | **Table row indentation** | No indent inside `()`: rows flush left | 2-space indent on every row | **Major** — removes whitespace tokens | `encoders.ts:98-103` |
| 3 | **Table inline object syntax** | `name(args)` no space before `(` | `name (args)` space before `(` | **Medium** — extra token per row | `encoders.ts:96` |
| 4 | **Key quoting (hyphens)** | `react-dom=^19.0.0` unquoted | `"react-dom"=^19.0.0` quoted | **Medium** — adds quote tokens | `validation.ts:6` |
| 5 | **Enum in inline objects** | `enum=celsius,fahrenheit` | `enum=[celsius,fahrenheit]` | **Small** — brackets add tokens | `encoders.ts:109` |
| 6 | **Required in inline objects** | `required=path,content` | `required=[path,content]` | **Small** — brackets add tokens | `encoders.ts:109` |
| 7 | **additionalProperties** | `additionalProperties=string` | `additionalProperties(type=string)` | **Small** — `type=` wrapper adds tokens | `encoders.ts:113-127` |
| 8 | **Array separator** | Space: `=[a b c]` | Comma: `=[a, b, c]` | **Tiny** — comma + space = extra chars | `encoders.ts:58` |
| 9 | **Table cell quoting** | Only quotes when truly needed | Quotes `"1.0"` but decoder strips it | **Bug** — round-trip broken | `primitives.ts:13-15` |
| 10 | **Top-level object unwrap** | All fields on one line | One field per line | **Major** — huge line count difference | `encoders.ts:18-22` |

---

## Detailed Breakdown with Examples

### 1. Object Block Style (inline vs multi-line)

This is the **single biggest difference**. Hand-written collapses small objects onto one line.

**Hand-written:**
```dx
scripts(dev=vite build="vite build" preview="vite preview" lint="eslint src/" test=vitest)
engines(node=>=22.0.0)
ci(runner=ubuntu-latest timeout=30 parallel=true)
```

**Encoder:**
```dx
scripts(
  dev=vite
  build="vite build"
  preview="vite preview"
  lint="eslint src/"
  test=vitest
)
engines(
  node=">=22.0.0"
)
ci(
  runner=ubuntu-latest
  timeout=30
  parallel=true
)
```

**Token impact:** `small.json` — hand-written: 128 tokens vs encoder: 155 tokens (+21%)

| File | Hand lines | Encoder lines | Extra lines from multi-line |
|------|-----------|--------------|---------------------------|
| small.dx | 5 | 26 | +21 lines |
| medium.dx | 26 | 66 | +40 lines |
| large.dx | 44 | 46 | +2 lines (tables mostly OK) |

---

### 2. Table Row Indentation

Hand-written has NO indent inside table `()`. Encoder adds 2-space indent.

**Hand-written:**
```dx
providers[id name type models hasFree website](
openai OpenAI api-key 40 false https://openai.com
anthropic Anthropic api-key 15 false https://anthropic.com
)
```

**Encoder:**
```dx
providers[id name type models hasFree website](
  openai OpenAI api-key 40 false https://openai.com
  anthropic Anthropic api-key 15 false https://anthropic.com
)
```

**Source:** `encoders.ts:98` — `yield indent(depth + 1) + vals.join(SPACE)`

---

### 3. Table Inline Object Space Before Paren

Hand-written has NO space between function name and `(`. Encoder adds a space.

**Hand-written:**
```dx
tool_calls[name arguments](
read_file(path=src/index.ts encoding=utf-8)
grep_code(query="export default" path=src/)
)
```

**Encoder:**
```dx
tool_calls[name arguments](
  read_file (path=src/index.ts encoding=utf-8)
  grep_code (query="export default" path=src/)
)
```

**Source:** `encoders.ts:134` — `encodeTableValue = encodeInlineValue` which produces `(fields)` with no space, but the table row joining at line 100 does `vals.join(SPACE)` — so the name and `(fields)` are joined by space.

---

### 4. Key Quoting (Hyphens and Dots)

Hand-written does NOT quote keys containing hyphens. Encoder quotes them.

**Hand-written:**
```dx
dependencies(react=^19.0.0 react-dom=^19.0.0 typescript=^5.7.0)
```

**Encoder:**
```dx
dependencies(
  react=^19.0.0
  "react-dom"=^19.0.0
  typescript=^5.7.0
)
```

**Source:** `validation.ts:6` — `isValidUnquotedKey` uses `/^[A-Z_][\w.]*$/i` which rejects hyphens (`-` is not in `\w`).

---

### 5. Enum/Required in Inline Objects (brackets vs bare)

Hand-written uses bare comma-separated values. Encoder wraps in `[]`.

**Hand-written:**
```dx
properties(
  language(type=string enum=javascript,python,rust)
  ...
) required=language,code
```

**Encoder:**
```dx
properties(
  language(type=string enum=[javascript,python,rust])
  ...
) required=[language,code]
```

**Source:** `encoders.ts:109` — `encodeInlineValue` always wraps arrays with `[...]`.

---

### 6. additionalProperties Format

Hand-written uses simple `=string`. Encoder uses `(type=string)`.

**Hand-written:**
```dx
env(type=object additionalProperties=string)
```

**Encoder:**
```dx
env(type=object additionalProperties(type=string))
```

**Source:** `encoders.ts:113-127` — nested objects in inline mode get `key(fields)` syntax, not `key=fields`.

---

### 7. Array Separator (space vs comma)

Hand-written uses space-separated arrays. Encoder uses comma+space.

**Hand-written:**
```dx
categories = [api-key local gateway oauth free]
default=[serde tokio clap tracing]
tags = serialization performance zero-copy llm
```

**Encoder:**
```dx
categories=[api-key, local, gateway, oauth, free]
default=[serde, tokio, clap, tracing]
tags=[serialization, performance, zero-copy, llm]
```

**Source:** `encoders.ts:58` — `join(COMMA + SPACE)` where `COMMA = ","`.

---

### 8. Top-Level Object Unwrapping

Hand-written puts multiple simple key=value pairs on one line. Encoder always one-per-line.

**Hand-written:**
```dx
total_providers = 40
total_models = 956
```
(these are separate lines but could be inline)

**Encoder:**
```dx
total_providers=40
total_models=956
```

Actually these are the same. The difference is in **nested objects** — see #1.

---

### 9. Table Cell Value Quoting (BUG)

Encoder quotes strings that look like numbers in table cells, but decoder strips quotes.

**Encoder produces:**
```dx
dependencies[name version optional](
  serde "1.0" false
  tokio "1.35" false
)
```

**Decoder returns:** `[{"name":"serde","version":1,"optional":false}]`

**Hand-written avoids this by NOT quoting numeric-looking strings:**
```dx
dependencies[name version optional](serde 1.0 false tokio 1.35 false)
```

This is actually a **design choice** — hand-written trusts the table schema to infer types. The encoder tries to be safe by quoting, but the decoder doesn't respect the quotes.

---

## Ranked by Token Impact

| Rank | Difference | Token Savings if Fixed | Difficulty to Fix |
|------|-----------|----------------------|-------------------|
| 1 | Inline object blocks (#1) | ~15-25% | Medium — need depth/width heuristic |
| 2 | No table row indent (#2) | ~3-5% | Easy — remove `indent(depth+1)` in table rows |
| 3 | No space before table paren (#3) | ~1-2% | Easy — remove SPACE in table value join |
| 4 | Unquote hyphen keys (#4) | ~2-5% | Easy — update `isValidUnquotedKey` regex |
| 5 | Bare enum/required (#5) | ~1-2% | Medium — change inline array encoding |
| 6 | additionalProperties format (#6) | ~0.5% | Medium — special-case in inline encoder |
| 7 | Space array separator (#7) | ~0.5% | Easy — change `join` separator |
| 8 | Fix table type coercion bug (#9) | 0 (correctness) | Hard — decoder parser fix needed |

---

## What `dx-token optimize` Prefers

The optimizer tested 8 format variants. Winners:

| Rank | Format | Score (sum of 4 BPE) | Description |
|------|--------|---------------------|-------------|
| 1 | `yaml space noindent` | 3,015 | Colon/equals + space + no indent |
| 2 | `yaml comma noindent` | 3,015 | Same, comma arrays |
| 5 | `= parens space indent` | 3,182 | **Current encoder format** |

**The encoder's default format ranks 5th out of 8.** The hand-written format is closer to the winning `yaml space noindent` style.
