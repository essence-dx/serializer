import type { FileHandle } from "node:fs/promises"
import type { DecodeOptions, DecodeStreamOptions, EncodeOptions } from "../../core/src/index.ts"
import type { InputSource } from "./types.ts"
import * as fsp from "node:fs/promises"
import * as path from "node:path"
import process from "node:process"
import { consola } from "consola"
import { decode, decodeStream, encode, encodeLines } from "../../core/src/index.ts"
import { jsonStreamFromEvents } from "./json-from-events.ts"
import { jsonStringifyLines } from "./json-stringify-stream.ts"
import { formatInputLabel, readInput, readLinesFromSource } from "./utils.ts"

export async function encodeToDx(config: {
  input: InputSource
  output?: string
  indent: NonNullable<EncodeOptions["indent"]>
  printStats: boolean
}): Promise<void> {
  const jsonContent = await readInput(config.input)

  let data: unknown
  try {
    data = JSON.parse(jsonContent)
  } catch (error) {
    throw new Error(`Failed to parse JSON: ${error instanceof Error ? error.message : String(error)}`)
  }

  const encodeOptions: EncodeOptions = {
    indent: config.indent,
  }

  if (config.printStats) {
    const dxOutput = encode(data, encodeOptions)
    const jsonBytes = Buffer.byteLength(jsonContent, "utf-8")
    const dxBytes = Buffer.byteLength(dxOutput, "utf-8")

    if (config.output) {
      await fsp.writeFile(config.output, dxOutput, "utf-8")
    } else {
      console.log(dxOutput)
    }

    const diff = jsonBytes - dxBytes
    const percent = ((diff / jsonBytes) * 100).toFixed(1)

    if (config.output) {
      const relativeInputPath = formatInputLabel(config.input)
      const relativeOutputPath = path.relative(process.cwd(), config.output)
      consola.success(`Encoded \`${relativeInputPath}\` → \`${relativeOutputPath}\``)
    }

    console.log()
    consola.info(`Byte sizes: ${jsonBytes} (JSON) → ${dxBytes} (DX Compact)`)
    consola.success(`Saved ${diff} bytes (-${percent}%)`)
  } else {
    await writeStreamingDx(encodeLines(data, encodeOptions), config.output)

    if (config.output) {
      const relativeInputPath = formatInputLabel(config.input)
      const relativeOutputPath = path.relative(process.cwd(), config.output)
      consola.success(`Encoded \`${relativeInputPath}\` → \`${relativeOutputPath}\``)
    }
  }
}

export async function decodeToJson(config: {
  input: InputSource
  output?: string
  indent: NonNullable<DecodeOptions["indent"]>
  strict: NonNullable<DecodeOptions["strict"]>
  expandPaths?: NonNullable<DecodeOptions["expandPaths"]>
}): Promise<void> {
  if (config.expandPaths === "safe") {
    const dxContent = await readInput(config.input)
    const decodeOptions: DecodeOptions = {
      strict: config.strict,
      expandPaths: config.expandPaths,
    }
    const data = decode(dxContent, decodeOptions)
    await writeStreamingJson(jsonStringifyLines(data, config.indent), config.output)
  } else {
    const lineSource = readLinesFromSource(config.input)
    const decodeStreamOptions: DecodeStreamOptions = {
      strict: config.strict,
    }
    const events = decodeStream(lineSource, decodeStreamOptions)
    const jsonChunks = jsonStreamFromEvents(events, config.indent)
    await writeStreamingJson(jsonChunks, config.output)
  }

  if (config.output) {
    const relativeInputPath = formatInputLabel(config.input)
    const relativeOutputPath = path.relative(process.cwd(), config.output)
    consola.success(`Decoded \`${relativeInputPath}\` → \`${relativeOutputPath}\``)
  }
}

async function writeStreamingJson(
  chunks: AsyncIterable<string> | Iterable<string>,
  outputPath?: string,
): Promise<void> {
  if (outputPath) {
    let fileHandle: FileHandle | undefined
    try {
      fileHandle = await fsp.open(outputPath, "w")
      for await (const chunk of chunks) {
        await fileHandle.write(chunk)
      }
    } finally {
      await fileHandle?.close()
    }
  } else {
    for await (const chunk of chunks) {
      process.stdout.write(chunk)
    }
    process.stdout.write("\n")
  }
}

async function writeStreamingDx(
  lines: Iterable<string>,
  outputPath?: string,
): Promise<void> {
  let isFirst = true
  if (outputPath) {
    let fileHandle: FileHandle | undefined
    try {
      fileHandle = await fsp.open(outputPath, "w")
      for (const line of lines) {
        if (!isFirst) await fileHandle.write("\n")
        await fileHandle.write(line)
        isFirst = false
      }
    } finally {
      await fileHandle?.close()
    }
  } else {
    for (const line of lines) {
      if (!isFirst) process.stdout.write("\n")
      process.stdout.write(line)
      isFirst = false
    }
    process.stdout.write("\n")
  }
}
