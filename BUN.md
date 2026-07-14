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

DX Compact achieves significant savings over JSON:

| Dataset | JSON | DX Compact | Savings |
|---------|------|-----------|---------|
| 80-tool LLM schema | 110,259 B | 31,886 B | **71.1%** |
| 12-tool coding assistant | 10,189 B | 6,253 B | 38.6% |
| 100-item github repos | 42,298 B | 21,586 B | 49.0% |

The format achieves these savings through:
- `key=value` syntax vs JSON's `"key": "value"` (no quotes, no braces)
- Single-line `()` blocks for objects (no indentation overhead)
- `table[cols](rows)` for arrays of objects (shared column headers)
- `[val1, val2]` for primitive arrays (no per-element wrapping)
- Adaptive quoting — only quote strings when necessary

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
