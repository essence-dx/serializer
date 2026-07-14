import type { JsonPrimitive } from "../types.ts"
import { DOUBLE_QUOTE, NULL_LITERAL } from "../constants.ts"
import { escapeString } from "../shared/string-utils.ts"
import { isSafeUnquoted, isValidUnquotedKey } from "../shared/validation.ts"

export function encodePrimitive(value: JsonPrimitive): string {
  if (value === null) return NULL_LITERAL
  if (typeof value === "boolean") return String(value)
  if (typeof value === "number") return String(value)
  return encodeStringLiteral(value)
}

export function encodeStringLiteral(value: string): string {
  if (isSafeUnquoted(value)) return value
  return `${DOUBLE_QUOTE}${escapeString(value)}${DOUBLE_QUOTE}`
}

export function encodeKey(key: string): string {
  if (isValidUnquotedKey(key)) return key
  return `${DOUBLE_QUOTE}${escapeString(key)}${DOUBLE_QUOTE}`
}
