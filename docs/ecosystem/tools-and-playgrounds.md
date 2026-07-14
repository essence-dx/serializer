---
description: DX Serializer playgrounds, CLI, editor support, and ecosystem tools.
---

# Tools and Playgrounds

Experiment with DX Serializer format interactively using these tools for token comparison, format conversion, and validation.

## Playgrounds

### Official Playground

The [DX Serializer Playground](/playground) lets you convert JSON or YAML to DX Serializer in real time, compare token counts, and share your experiments via URL.

### Community Playgrounds

- [Format Tokenization Playground](https://www.curiouslychase.com/playground/format-tokenization-exploration)
- [DX Serializer Tools](https://toontools.vercel.app/)

## CLI Tool

The official DX Serializer CLI provides command-line conversion, token statistics, and all encoding/decoding features. See the [CLI reference](/cli/) for full documentation.

```bash
npx @dx-serializer/cli input.json --stats -o output.dx
```

## Editor Support

### VS Code

[DX Serializer Language Support](https://marketplace.visualstudio.com/items?itemName=vishalraut.vscode-dx) – Syntax highlighting, validation, conversion, and token analysis.

Install from the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=vishalraut.vscode-dx) or via command line:

```bash
code --install-extension vishalraut.vscode-dx
```

### Tree-sitter Grammar

[tree-sitter-dx](https://github.com/3swordman/tree-sitter-dx) – Grammar for Tree-sitter-compatible editors (Neovim, Helix, Emacs, Zed).

### Neovim

[toon.nvim](https://github.com/thalesgelinger/toon.nvim) – Lua-based plugin for Neovim.

### Other Editors

Use YAML syntax highlighting as a close approximation. Most editors allow associating `.dx` files with YAML language mode.

## Databases

### ToonStore

[ToonStore](https://github.com/Kalama-Tech/toonstoredb) – Redis-compatible embedded database (Rust) that stores data in DX Serializer format.

## ORMs

### TORM

[TORM](https://github.com/Kalama-Tech/torm) – ORM that works with the ToonStore database, with SDKs for Node.js, Python, Go, and PHP.

## Web APIs

If you're building web applications that work with DX Serializer, you can use the TypeScript library in the browser:

```ts
import { decode, encode } from '@dx-serializer/core'

// Works in browsers, Node.js, Deno, and Bun
const dx = encode(data)
const data = decode(dx)
```

See the [API Reference](/reference/api) for details.

## MCP

### Tooner

[Tooner](https://github.com/chaindead/tooner) – MCP proxy that converts JSON tool responses to TOON.
