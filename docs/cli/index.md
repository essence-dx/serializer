---
description: Convert JSON to DX Serializer and back from the command line, with token statistics, streaming, and delimiter options.
---

# Command Line Interface

The `@dx-serializer/cli` package converts JSON to DX Serializer and DX Serializer to JSON. Use it to measure token savings before integrating DX Serializer into your application, or to pipe JSON through DX Serializer in shell workflows alongside tools like `curl` and `jq`. The CLI supports stdin/stdout, token statistics, streaming for large datasets, and every encoding option in the library.

The CLI is built on the `@dx-serializer/core` TypeScript implementation and follows the [latest specification](/reference/spec).

## Usage

### Without Installation

Use `npx` to run the CLI without installing:

::: code-group

```bash [Encode]
npx @dx-serializer/cli input.json -o output.dx
```

```bash [Decode]
npx @dx-serializer/cli data.dx -o output.json
```

```bash [Stdin]
echo '{"name": "Ada"}' | npx @dx-serializer/cli
```

:::

### Global Installation

Or install globally for repeated use:

::: code-group

```bash [npm]
npm install -g @dx-serializer/cli
```

```bash [pnpm]
pnpm add -g @dx-serializer/cli
```

```bash [yarn]
yarn global add @dx-serializer/cli
```

:::

After global installation, use the `dx` command:

```bash
dx input.json -o output.dx
```

## Basic Usage

### Auto-Detection

The CLI automatically detects the operation based on file extension:
- `.json` files → encode (JSON to DX Serializer)
- `.dx` files → decode (DX Serializer to JSON)

When reading from stdin, use `--encode` or `--decode` flags to specify the operation (defaults to encode).

::: code-group

```bash [Encode JSON to DX Serializer]
dx input.json -o output.dx
```

```bash [Decode DX Serializer to JSON]
dx data.dx -o output.json
```

```bash [Output to stdout]
dx input.json
```

```bash [Pipe from stdin]
cat data.json | dx
echo '{"name": "Ada"}' | dx
```

```bash [Decode from stdin]
cat data.dx | dx --decode
```

:::

By convention, DX Serializer files use the `.dx` extension and the provisional media type `text/dx` (see [spec §17](https://github.com/dx-www/spec/blob/main/SPEC.md#17-iana-considerations)).

### Standard Input

Omit the input argument or use `-` to read from stdin. This enables piping data directly from other commands:

```bash
# No argument needed
cat data.json | dx

# Explicit stdin with hyphen (equivalent)
cat data.json | dx -

# Decode from stdin
cat data.dx | dx --decode
```

## Performance

### Streaming Output

Both encoding and decoding operations use streaming output, writing incrementally without building the full output string in memory. This makes the CLI efficient for large datasets without requiring additional configuration.

**JSON → DX Serializer (Encode)**:

- Streams DX Serializer lines to output.
- No full DX Serializer string in memory.

**DX Serializer → JSON (Decode)**:

- Uses the same event-based streaming decoder as the `decodeStream` API in `@dx-serializer/core`.
- Streams JSON tokens to output.
- No full JSON string in memory.
- When `--expandPaths safe` is enabled, falls back to non-streaming decode internally to apply deep-merge expansion before writing JSON.

Process large files with minimal memory usage:

```bash
# Encode large JSON file
dx huge-dataset.json -o output.dx

# Decode large DX Serializer file
dx huge-dataset.dx -o output.json

# Process millions of records efficiently via stdin
cat million-records.json | dx > output.dx
cat million-records.dx | dx --decode > output.json
```

Peak memory usage scales with data depth, not total size. This allows processing arbitrarily large files as long as individual nested structures fit in memory.

::: tip Token Statistics
When using the `--stats` flag with encode, the CLI builds the full DX Serializer string once to compute accurate token counts. For maximum memory efficiency on very large files, omit `--stats`.
:::

## Options

| Option | Description |
| ------ | ----------- |
| `-o, --output <file>` | Output file path (prints to stdout if omitted) |
| `-e, --encode` | Force encode mode (overrides auto-detection) |
| `-d, --decode` | Force decode mode (overrides auto-detection) |
| `--delimiter <char>` | Array delimiter: `,` (comma), tab character, `\|` (pipe). Pass tab as `$'\t'` in bash/zsh |
| `--indent <number>` | Indentation size (default: `2`) |
| `--stats` | Show token count estimates and savings (encode only) |
| `--no-strict` | Skip decode validation (array counts, indentation, header delimiter); last-write-wins on duplicate keys |
| `--keyFolding <mode>` | Key folding mode: `off`, `safe` (default: `off`) |
| `--flattenDepth <number>` | Maximum segments to fold (default: `Infinity`) – requires `--keyFolding safe` |
| `--expandPaths <mode>` | Path expansion mode: `off`, `safe` (default: `off`) |
| `--verbose` | Show full stack traces and cause chains for errors (default: `false`) |

## Advanced Examples

### Token Statistics

Show token savings when encoding:

```bash
dx data.json --stats -o output.dx
```

This helps you estimate token cost savings before sending data to LLMs.

Example output:

```
✔ Encoded data.json → output.dx

ℹ Token estimates: ~15,145 (JSON) → ~8,745 (DX Serializer)
✔ Saved ~6,400 tokens (-42.3%)
```

### Alternative Delimiters

DX Serializer supports three delimiters: comma (default), tab, and pipe. Alternative delimiters can save additional tokens depending on the data.

::: code-group

```bash [Tab-separated (bash/zsh)]
dx data.json --delimiter $'\t' -o output.dx
```

```bash [Pipe-separated]
dx data.json --delimiter "|" -o output.dx
```

:::

The `--delimiter` value must be the actual delimiter character. In bash/zsh, use `$'\t'` to pass a real tab; literal `"\t"` is rejected as an invalid delimiter.

**Tab delimiter example:**

::: code-group

```yaml [Tab]
items[2	]{id	name	qty	price}:
  A1	Widget	2	9.99
  B2	Gadget	1	14.5
```

```yaml [Comma (default)]
items[2]{id,name,qty,price}:
  A1,Widget,2,9.99
  B2,Gadget,1,14.5
```

:::

::: tip
Tab delimiters often tokenize more efficiently than commas and reduce the need for quote-escaping. Use `--delimiter $'\t'` (bash/zsh) for maximum token savings on large tabular data. See [Delimiter Strategies](/reference/api#delimiter-strategies) for full guidance.
:::

### Lenient Decoding

Skip validation for faster, more forgiving decoding:

```bash
dx data.dx --no-strict -o output.json
```

With `--no-strict`, the decoder stops enforcing array count matches, indentation multiples, and header delimiter mismatches. Duplicate sibling keys no longer throw – the last value wins. Malformed array headers fall back to plain `key: value` lines instead of erroring.

### Decode Error Output

When a DX Serializer document fails to parse, the CLI renders the offending line with a caret pointing at the first non-whitespace character. Tabs are shown as `→` so the caret column reflects what the decoder actually saw.

For an input file that uses a tab to indent the second line (rendered here with `→`):

```
a:
→b: 1
```

The CLI prints:

```
 ERROR  Failed to decode DX Serializer at line 2: Tabs are not allowed in indentation in strict mode

  2 | →b: 1
      ^
```

The exit code is `1` on any error. Stack traces are suppressed by default. Pass `--verbose` to include the full stack and the underlying cause chain – useful when filing a bug report or diagnosing an unexpected error path:

```bash
cat broken.dx | dx --decode --verbose
```

::: tip Programmatic Access
Decode errors are thrown as `ToonDecodeError` instances by the library. The CLI's caret rendering is built on the structured `line` and `source` fields exposed on that class. See the [Error Handling](/reference/api#error-handling) section of the API reference if you want the same diagnostic detail in your own code.
:::

### Stdin Workflows

The CLI integrates seamlessly with Unix pipes and other command-line tools:

```bash
# Convert API response to DX Serializer
curl https://api.example.com/data | dx --stats

# Process large dataset
cat large-dataset.json | dx --delimiter $'\t' > output.dx

# Chain with jq
jq '.results' data.json | dx > filtered.dx
```

### Key Folding

Collapse nested wrapper chains to reduce tokens (since spec v1.5):

::: code-group

```bash [Basic key folding]
dx input.json --keyFolding safe -o output.dx
```

```bash [Limit folding depth]
dx input.json --keyFolding safe --flattenDepth 2 -o output.dx
```

:::

**Example:**

For data like:

```json
{
  "data": {
    "metadata": {
      "items": ["a", "b"]
    }
  }
}
```

With `--keyFolding safe`, output becomes:

```yaml
data.metadata.items[2]: a,b
```

Instead of:

```yaml
data:
  metadata:
    items[2]: a,b
```

### Path Expansion

Reconstruct nested structure from folded keys when decoding:

```bash
dx data.dx --expandPaths safe -o output.json
```

This pairs with `--keyFolding safe` for lossless round-trips.

### Round-Trip Workflow

```bash
# Encode with folding
dx input.json --keyFolding safe -o compressed.dx

# Decode with expansion (restores original structure)
dx compressed.dx --expandPaths safe -o output.json

# Verify round-trip
diff input.json output.json
```

### Combined Options

Combine multiple options for maximum efficiency:

```bash
# Key folding + tab delimiter + stats
dx data.json --keyFolding safe --delimiter $'\t' --stats -o output.dx
```
