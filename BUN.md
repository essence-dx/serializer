# DX Serializer — Bun Reference

## Overview

**DX Serializer** is a Bun-based TypeScript monorepo providing encoding/decoding for **DX Compact**, an ultra-compact serialization format optimized for LLM token efficiency.

This codebase was ported from the [TOON](https://github.com/toon-format/toon) reference implementation, replacing the TOON format with DX Compact syntax and switching from pnpm to Bun.

## Quick Start

```bash
# Install dependencies
cd G:\Dx\serializer
bun install

# Run tests
cd packages/core && bun test

# Use CLI directly
cd packages/cli && bun run src/cli-entry.ts --help
```

## Project Structure

```
G:\Dx\serializer/
├── package.json          # Root monorepo (Bun workspaces)
├── BUN.md                # This file
├── packages/
│   ├── core/             # @dx-serializer/core — encode/decode library
│   │   ├── src/
│   │   │   ├── index.ts           # Public API (encode, decode, etc.)
│   │   │   ├── types.ts           # TypeScript types
│   │   │   ├── constants.ts       # DX Compact structural characters
│   │   │   ├── encode/
│   │   │   │   ├── encoders.ts     # Main encoder (paren-based, table format)
│   │   │   │   ├── primitives.ts   # Value stringification & quoting
│   │   │   │   ├── normalize.ts    # JSON normalization (Date, BigInt, etc.)
│   │   │   │   └── replacer.ts     # Replacer function support
│   │   │   ├── decode/
│   │   │   │   ├── decoders.ts     # Main decoder (paren-matching)
│   │   │   │   ├── parser.ts       # Key=val, inline object, table parsing
│   │   │   │   ├── scanner.ts      # Line scanning & indentation
│   │   │   │   ├── event-builder.ts # Events → JsonValue AST builder
│   │   │   │   ├── expand.ts       # Path expansion for dotted keys
│   │   │   │   ├── validation.ts   # Parenthesis balance validation
│   │   │   │   └── errors.ts       # DxDecodeError
│   │   │   └── shared/
│   │   │       ├── validation.ts   # Quoting rules
│   │   │       ├── string-utils.ts # Escape/unescape
│   │   │       └── literal-utils.ts # Boolean/null/number detection
│   │   └── test/
│   │       ├── dx-compact.test.ts  # DX Compact round-trip tests
│   │       └── normalization.test.ts # JS type normalization tests
│   └── cli/             # @dx-serializer/cli — CLI tool
│       ├── bin/
│       │   └── dx.mjs              # CLI entry (shebang)
│       ├── src/
│       │   ├── cli-entry.ts        # Main bootstrap
│       │   ├── index.ts            # Command definition (citty)
│       │   ├── conversion.ts       # encodeToDx / decodeToJson
│       │   ├── format-error.ts     # Pretty error rendering
│       │   ├── json-stringify-stream.ts
│       │   ├── json-from-events.ts
│       │   └── utils.ts            # File I/O, stdin, mode detection
│       └── test/                   # CLI tests
```

## Package Scripts

### Root

| Command | Description |
|---------|-------------|
| `bun install` | Install all workspace dependencies |
| `bun test --filter @dx-serializer/*` | Run all tests in all packages |

### packages/core

| Command | Description |
|---------|-------------|
| `bun test` | Run core library tests (54 tests) |

### packages/cli

| Command | Description |
|---------|-------------|
| `bun test` | Run CLI tests |

## CLI Usage

```bash
# JSON → DX Compact (encode)
echo '{"name":"test","count":42}' | bun run src/cli-entry.ts
# name=test
# count=42

# DX Compact → JSON (decode)
echo 'name=test' | bun run src/cli-entry.ts --decode
# { "name": "test" }

# File encoding with output
bun run src/cli-entry.ts input.json -o output.dx

# File decoding
bun run src/cli-entry.ts input.dx --decode -o output.json

# With byte statistics
bun run src/cli-entry.ts input.json --stats
```

### CLI Options

| Flag | Description |
|------|-------------|
| `INPUT` | Input file path (omit or `-` for stdin) |
| `-o, --output` | Output file path |
| `-e, --encode` | Force encode mode |
| `-d, --decode` | Force decode mode |
| `--indent` | JSON output indentation (default: 2) |
| `--strict` | Strict validation (default: true) |
| `--expandPaths` | Path expansion: off, safe (default: off) |
| `--stats` | Show byte size comparison |
| `--verbose` | Show full error stack traces |

### Extension Auto-Detection

- `.json` → encode
- `.dx` → decode

## Library API

```typescript
import { encode, decode, encodeLines, decodeFromLines } from "@dx-serializer/core"

// Encode to DX Compact string
const dx = encode({ name: "test", count: 42 })
// "name=test\ncount=42"

// Decode DX Compact string to object
const obj = decode('name=test\ncount=42')
// { name: "test", count: 42 }

// Stream lines (encode)
for (const line of encodeLines({ a: 1, b: 2 })) {
  console.log(line)  // "a=1", "b=2"
}

// Decode from lines
const result = decodeFromLines(["name=test", "count=42"])

// With options
const dx = encode(data, { indent: 4 })
const obj = decode(dx, { strict: true, expandPaths: "safe" })
```

## DX Compact Format

### Syntax

| JSON | DX Compact | Description |
|------|-----------|-------------|
| `{"key":"val"}` | `key=val` | Simple key-value |
| `{"a":{"b":"c"}}` | `a(\n  b=c\n)` | Nested object (multi-line) |
| `{"x":{}}` | `x=()` | Empty nested object |
| `{"arr":[1,2]}` | `arr=[1, 2]` | Primitive array |
| `{"items":[{"id":1}]}` | `items[id](\n  1\n)` | Array of objects (table) |
| `{"s":"hello"}` | `s=hello` | Unquoted string (if safe) |
| `{"s":"hi there"}` | `s="hi there"` | Quoted string (spaces) |
| `true`, `false` | `true`, `false` | Boolean literals |
| `null` | `null` | Null literal |
| `{"v":42}` | `v=42` | Integer |
| `{"v":3.14}` | `v=3.14` | Float |

### Inline Objects (single-line)

Within values, objects can be inlined:

```dx
params(type=object properties(path(type=string)) required=[path])
```

This represents:
```json
{
  "type": "object",
  "properties": { "path": { "type": "string" } },
  "required": ["path"]
}
```

### Object Blocks (multi-line)

Top-level objects unwrap to multi-line blocks:

```dx
config(
  host=localhost
  port=8080
  debug=true
)
```

### Tables

Arrays of uniform objects use table format:

```dx
items[name price](
  Apple 0.99
  Banana 0.59
)
```

Table cells can contain inline objects:

```dx
tools[name description params](
  read_file "Read file contents" (type=object required=[path])
  write_file "Write file" (type=object required=[path,content])
)
```

### Empty Objects

```dx
extra=()
```

### Comments

Lines starting with `#` are ignored:

```dx
# This is a comment
name=test
```

## Round-Trip Guarantees

All DX Compact output can be decoded back to the original JSON value:

```typescript
const original = { name: "test", items: [{ id: 1 }] }
const dx = encode(original)
const decoded = decode(dx)
JSON.stringify(original) === JSON.stringify(decoded) // true
```

## Token Efficiency

DX Compact achieves significant token savings over all popular formats.
Measured with `gpt-tokenizer` o200k_base (GPT-4o tokenizer) across **20 benchmark files**
ranging from small configs to massive 100K-record datasets.

### Overall Statistics vs JSON, YAML, TOON, JSONC

| Comparison | Average | Min | Max | Samples |
|-----------|---------|-----|-----|---------|
| **DX vs JSON pretty** | **−58.9%** | −20.8% | −86.4% | 20 |
| **DX vs JSON compact** | **−37.1%** | −7.4% | −75.0% | 20 |
| **DX vs YAML** | **−38.5%** | −2.3% | −60.4% | 11 |
| **DX vs TOON** | **−33.6%** | −4.7% | −50.6% | 20 |

**DX beats TOON on every single benchmark file (20/20).**

### Record-Breaking Benchmarks

| Dataset | Tokens (o200k) | vs JSON | vs TOON | Type |
|---------|---------------:|--------:|--------:|------|
| 100K boolean records (3 cols) | 300,006 | **−86%** | −40% | `true/false` only |
| 10K boolean records (5 cols) | 50,008 | **−85%** | −29% | `true/false` only |
| 100K flat records (2 cols) | 399,004 | **−79%** | −33% | numbers + strings |
| 80-tool LLM schema | 7,686 | **−69%** | −49% | nested tool defs |
| 40-provider catalog | 667 | **−66%** | −21% | uniform objects |
| 12-tool coding assistant | 1,268 | **−51%** | −36% | tool schemas |
| 100-item github repos | 8,336 | **−45%** | −5% | mixed data |
| Small project metadata | 130 | **−38%** | −15% | 6 fields |

### By Data Type

| Data Shape | Best Case | Worst Case |
|-----------|-----------|------------|
| **Booleans** (uniform, many rows) | **−86%** | — |
| **Uniform objects** (tables) | **−79%** | −45% |
| **Tool schemas** (nested tables) | **−69%** | −51% |
| **Deep nesting** (few items) | — | **−21%** |
| **Nested tool calls** (single item) | — | **−20%** |

### Why DX Wins

- `key=value` vs `"key": "value"` — no quotes, no braces
- `table[cols](rows)` — shared column headers, no repeated keys
- Inline `()` blocks — no indentation overhead
- `[val1 val2]` — space-separated, no commas
- Adaptive quoting — only quote strings when necessary

### Cost Impact

Scenario: AI agent with 20-tool schema, 10K requests/day, GPT-4o ($2.50/1M input):

| Format | Tokens/request | Monthly cost | Annual cost | Savings vs JSON |
|--------|---------------:|------------:|------------:|----------------:|
| JSON   | 2,862 | $214.65 | $2,575.80 | — |
| TOON   | 2,062 | $154.65 | $1,855.80 | $720 |
| **DX** | **1,114** | **$83.55** | **$1,002.60** | **$1,573** |

For boolean-heavy data (100K records, 10K req/day):

| Format | Tokens/request | Monthly | Annual | Annual vs JSON |
|--------|---------------:|--------:|-------:|---------------:|
| JSON   | 2,200,009 | $165,000 | $1,980,000 | — |
| JSONC  | 1,200,005 | $90,000 | $1,080,000 | $900,000 |
| **DX** | **300,006** | **$22,500** | **$270,000** | **$1,710,000** |

## Development

```bash
# Run core tests
cd packages/core && bun test

# Test specific file
cd packages/core && bun test test/dx-compact.test.ts

# Run a single expression
cd packages/core && bun -e "
  import { encode } from './src/index.ts'
  console.log(encode({ hello: 'world' }))
"
```

## MCP (Model Context Protocol) Integration

The core library can be used to provide DX Compact formatted data to LLMs via MCP tools:

```typescript
import { encode } from "@dx-serializer/core"

const toolSchema = encode({
  tools: [
    {
      name: "read_file",
      description: "Read a file",
      parameters: {
        type: "object",
        properties: { path: { type: "string" } },
        required: ["path"],
      },
    },
  ],
})
```

The resulting DX Compact string reduces token consumption by 50-70% compared to JSON for complex schemas.

## File Extensions

| Extension | Format | Description |
|-----------|--------|-------------|
| `.dx` | DX Compact | Primary format (encode/decode) |
| `.json` | JSON | Input/output for conversion |

## Migration from TOON

This project replaces the TOON format with DX Compact:

| Aspect | TOON | DX Compact |
|--------|------|-----------|
| Key-value | `key: value` | `key=value` |
| Nesting | Indentation | Parentheses `()` |
| Tables | `key[N]{fields}:` | `key[fields](rows)` |
| Arrays | `key[N]: a,b,c` | `key=[a,b,c]` |
| Package manager | pnpm | Bun |
| Package name | `@toon-format/toon` | `@dx-serializer/core` |
| CLI binary | `toon` | `dx` |
