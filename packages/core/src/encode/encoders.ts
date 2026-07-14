import type { Depth, JsonArray, JsonObject, JsonPrimitive, JsonValue, ResolvedEncodeOptions } from "../types.ts"
import { SPACE, OPEN_BRACKET, CLOSE_BRACKET, COMMA, OPEN_PAREN, CLOSE_PAREN, EQUALS } from "../constants.ts"
import { isJsonArray, isJsonObject, isJsonPrimitive, isArrayOfPrimitives, isArrayOfObjects, isEmptyObject } from "./normalize.ts"
import { encodeKey, encodePrimitive } from "./primitives.ts"

export function* encodeJsonValue(value: JsonValue, options: ResolvedEncodeOptions, depth: Depth): Generator<string> {
  if (isJsonPrimitive(value)) {
    yield encodePrimitive(value)
    return
  }
  if (isJsonArray(value)) {
    yield* encodeArrayLines(undefined, value, depth, options)
    return
  }
  yield* encodeObjectLines(value, depth, options)
}

function* encodeObjectLines(value: JsonObject, depth: Depth, options: ResolvedEncodeOptions): Generator<string> {
  for (const [key, val] of Object.entries(value)) {
    yield* encodeKeyValueLine(key, val, depth, options)
  }
}

function* encodeKeyValueLine(key: string, value: JsonValue, depth: Depth, options: ResolvedEncodeOptions): Generator<string> {
  const encodedKey = encodeKey(key)

  if (isJsonPrimitive(value)) {
    yield indent(depth) + encodedKey + EQUALS + encodePrimitive(value)
    return
  }

  if (isJsonArray(value)) {
    yield* encodeArrayLines(encodedKey, value, depth, options)
    return
  }

  if (isEmptyObject(value)) {
    yield indent(depth) + encodedKey + EQUALS + OPEN_PAREN + CLOSE_PAREN
    return
  }

  yield indent(depth) + encodedKey + OPEN_PAREN
  yield* encodeObjectLines(value, depth + 1, options)
  yield indent(depth) + CLOSE_PAREN
}

function* encodeArrayLines(key: string | undefined, value: JsonArray, depth: Depth, options: ResolvedEncodeOptions): Generator<string> {
  if (value.length === 0) {
    if (key === undefined) {
      yield "[]"
    } else {
      yield indent(depth) + key + "=" + OPEN_BRACKET + CLOSE_BRACKET
    }
    return
  }

  if (isArrayOfPrimitives(value)) {
    const encoded = (value as JsonPrimitive[]).map(v => encodePrimitive(v)).join(COMMA + SPACE)
    if (key === undefined) {
      yield OPEN_BRACKET + encoded + CLOSE_BRACKET
    } else {
      yield indent(depth) + key + EQUALS + OPEN_BRACKET + encoded + CLOSE_BRACKET
    }
    return
  }

  if (isArrayOfObjects(value)) {
    yield* encodeTabularArrayLines(key, value as JsonObject[], depth, options)
    return
  }

  yield* encodeMixedArrayLines(key, value, depth, options)
}

function* encodeTabularArrayLines(key: string | undefined, rows: JsonObject[], depth: Depth, options: ResolvedEncodeOptions): Generator<string> {
  if (rows.length === 0) {
    if (key === undefined) {
      yield "[]"
    } else {
      yield indent(depth) + key + EQUALS + OPEN_BRACKET + CLOSE_BRACKET
    }
    return
  }

  const columns = Object.keys(rows[0])
  for (const row of rows) {
    const rowKeys = Object.keys(row)
    if (rowKeys.length !== columns.length || !rowKeys.every(k => columns.includes(k))) {
      yield* encodeMixedArrayLines(key, rows, depth, options)
      return
    }
  }

  const encodedKey = key === undefined ? "" : key
  const schema = columns.map(c => encodeKey(c)).join(SPACE)
  yield indent(depth) + encodedKey + OPEN_BRACKET + schema + CLOSE_BRACKET + OPEN_PAREN

  for (const row of rows) {
    const vals = columns.map(col => encodeTableValue(row[col]))
    yield indent(depth + 1) + vals.join(SPACE)
  }

  yield indent(depth) + CLOSE_PAREN
}

function encodeInlineValue(value: JsonValue): string {
  if (isJsonPrimitive(value)) return encodePrimitive(value)
  if (isJsonArray(value)) {
    const items = (value as JsonValue[]).map(v => encodeInlineValue(v))
    return OPEN_BRACKET + items.join(COMMA) + CLOSE_BRACKET
  }
  if (isEmptyObject(value as JsonObject)) return OPEN_PAREN + CLOSE_PAREN
  const fields = Object.entries(value as JsonObject).map(([k, v]) => encodeInlineField(k, v))
  return OPEN_PAREN + fields.join(SPACE) + CLOSE_PAREN
}

function encodeInlineField(key: string, value: JsonValue): string {
  const ek = encodeKey(key)
  if (isJsonPrimitive(value) || isJsonArray(value)) {
    return ek + EQUALS + encodeInlineValue(value)
  }
  if (isEmptyObject(value as JsonObject)) {
    return ek + EQUALS + ""
  }
  // Nested object: no `=` between key and parens
  return ek + OPEN_PAREN + objectFieldsInline(value as JsonObject) + CLOSE_PAREN
}

function objectFieldsInline(obj: JsonObject): string {
  return Object.entries(obj).map(([k, v]) => encodeInlineField(k, v)).join(SPACE)
}

// Re-export for compatibility
const encodeTableValue = encodeInlineValue
const encodeMixedValue = encodeInlineValue

function* encodeMixedArrayLines(key: string | undefined, items: JsonValue[], depth: Depth, options: ResolvedEncodeOptions): Generator<string> {
  const encodedKey = key === undefined ? "" : key
  const prefix = key === undefined ? "" : encodedKey + EQUALS
  const encoded = items.map(v => encodeInlineValue(v))
  yield indent(depth) + prefix + OPEN_BRACKET + encoded.join(COMMA + SPACE) + CLOSE_BRACKET
}

function indent(depth: Depth): string {
  return "  ".repeat(depth)
}
