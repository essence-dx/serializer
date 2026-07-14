import type { DecodeOptions, DecodeStreamOptions, EncodeOptions, JsonStreamEvent, JsonValue, ResolvedEncodeOptions } from "./types.ts"
import { decodeStreamSync as decodeStreamSyncCore, decodeFromLines } from "./decode/decoders.ts"
import { buildValueFromEvents } from "./decode/event-builder.ts"
import { expandPathsSafe } from "./decode/expand.ts"
import { encodeJsonValue } from "./encode/encoders.ts"
import { normalizeValue } from "./encode/normalize.ts"
import { applyReplacer } from "./encode/replacer.ts"

export { DxDecodeError } from "./decode/parser.ts"
export type {
  DecodeOptions,
  DecodeStreamOptions,
  EncodeOptions,
  EncodeReplacer,
  JsonArray,
  JsonObject,
  JsonPrimitive,
  JsonStreamEvent,
  JsonValue,
} from "./types.ts"

export function encode(input: unknown, options?: EncodeOptions): string {
  return Array.from(encodeLines(input, options)).join("\n")
}

export function encodeLines(input: unknown, options?: EncodeOptions): Iterable<string> {
  const normalizedValue = normalizeValue(input)
  const resolvedOptions = resolveOptions(options)
  const maybeReplacedValue = resolvedOptions.replacer
    ? applyReplacer(normalizedValue, resolvedOptions.replacer)
    : normalizedValue
  return encodeJsonValue(maybeReplacedValue, resolvedOptions, 0)
}

export function decode(input: string, options?: DecodeOptions): JsonValue {
  const lines = input.split("\n")
  return decodeFromLines(lines, options)
}

export function decodeFromLines(lines: Iterable<string>, options?: DecodeOptions): JsonValue {
  const resolvedOptions = resolveDecodeOptions(options)
  const streamOptions: DecodeStreamOptions = {
    strict: resolvedOptions.strict,
  }
  const events = decodeStreamSyncCore(lines, streamOptions)
  const decodedValue = buildValueFromEvents(events)
  if (resolvedOptions.expandPaths === "safe") {
    return expandPathsSafe(decodedValue, resolvedOptions.strict)
  }
  return decodedValue
}

export function decodeStreamSync(lines: Iterable<string>, options?: DecodeStreamOptions): Iterable<JsonStreamEvent> {
  return decodeStreamSyncCore(lines, options)
}

export function decodeStream(
  source: AsyncIterable<string> | Iterable<string>,
  options?: DecodeStreamOptions,
): AsyncIterable<JsonStreamEvent> {
  if (Symbol.asyncIterator in source) {
    return decodeStreamAsync(source as AsyncIterable<string>, options)
  }
  const events = decodeStreamSyncCore(source as Iterable<string>, options)
  return {
    [Symbol.asyncIterator](): AsyncIterator<JsonStreamEvent> {
      const iterator = events[Symbol.iterator]()
      return {
        async next(): Promise<IteratorResult<JsonStreamEvent>> {
          const result = iterator.next()
          if (result.done) return { done: true, value: undefined }
          return { done: false, value: result.value }
        },
      }
    },
  }
}

async function* decodeStreamAsync(
  source: AsyncIterable<string>,
  _options?: DecodeStreamOptions,
): AsyncGenerator<JsonStreamEvent> {
  const lines: string[] = []
  for await (const chunk of source) {
    lines.push(chunk)
  }
  const events = decodeStreamSyncCore(lines, _options)
  for (const event of events) {
    yield event
  }
}

function resolveOptions(options?: EncodeOptions): ResolvedEncodeOptions {
  return {
    indent: options?.indent ?? 2,
    replacer: options?.replacer,
  }
}

function resolveDecodeOptions(options?: DecodeOptions): { strict: boolean; expandPaths: "off" | "safe" } {
  return {
    strict: options?.strict ?? true,
    expandPaths: options?.expandPaths ?? "off",
  }
}
