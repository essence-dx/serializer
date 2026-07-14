import type { ArgsDef, CommandDef } from "citty"
import type { DecodeOptions, EncodeOptions } from "../../core/src/index.ts"
import type { InputSource } from "./types.ts"
import * as path from "node:path"
import process from "node:process"
import { defineCommand } from "citty"
import { consola } from "consola"
import pkg from "../package.json" with { type: "json" }
import { decodeToJson, encodeToDx } from "./conversion.ts"
import { formatError } from "./format-error.ts"
import { detectMode } from "./utils.ts"

const { name, version } = pkg

const args: ArgsDef = {
  input: {
    type: "positional",
    description: "Input file path (omit or use \"-\" to read from stdin)",
    required: false,
  },
  output: {
    type: "string",
    description: "Output file path",
    alias: "o",
  },
  encode: {
    type: "boolean",
    description: "Encode JSON to DX Compact (auto-detected by default)",
    alias: "e",
  },
  decode: {
    type: "boolean",
    description: "Decode DX Compact to JSON (auto-detected by default)",
    alias: "d",
  },
  indent: {
    type: "string",
    description: "JSON output indentation size",
    default: "2",
  },
  strict: {
    type: "boolean",
    description: "Strict decode validation (disable with --no-strict)",
    default: true,
  },
  expandPaths: {
    type: "string",
    description: "Enable path expansion: off, safe (default: off)",
    default: "off",
  },
  stats: {
    type: "boolean",
    description: "Show byte size comparison",
    default: false,
  },
  verbose: {
    type: "boolean",
    description: "Show full stack traces and cause chains for errors",
    default: false,
  },
} as const

export const mainCommand: CommandDef<ArgsDef> = defineCommand({
  meta: {
    name,
    description: "DX CLI – Convert between JSON and DX Compact formats",
    version,
  },
  args,
  async run({ args }) {
    const input = args.input

    const inputSource: InputSource = !input || input === "-"
      ? { type: "stdin" }
      : { type: "file", path: path.resolve(input) }
    const outputPath = args.output ? path.resolve(args.output) : undefined

    const indent = Number.parseInt(args.indent || "2", 10)
    if (Number.isNaN(indent) || indent < 0) {
      throw new Error(`Invalid indent value: ${args.indent}`)
    }

    const expandPaths = args.expandPaths || "off"
    if (expandPaths !== "off" && expandPaths !== "safe") {
      throw new Error(`Invalid expandPaths value "${expandPaths}". Valid values are: off, safe`)
    }

    const mode = detectMode(inputSource, args.encode, args.decode)

    try {
      if (mode === "encode") {
        await encodeToDx({
          input: inputSource,
          output: outputPath,
          indent,
          printStats: args.stats === true,
        })
      } else {
        await decodeToJson({
          input: inputSource,
          output: outputPath,
          indent,
          strict: args.strict !== false,
          expandPaths: expandPaths as NonNullable<DecodeOptions["expandPaths"]>,
        })
      }
    } catch (error) {
      consola.error(formatError(error, { isVerbose: args.verbose === true }))
      process.exit(1)
    }
  },
})
