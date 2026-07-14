import { isBooleanOrNullLiteral } from "./literal-utils.ts"

const NUMERIC_LIKE_PATTERN = /^-?\d+(?:\.\d+)?(?:e[+-]?\d+)?$/i

export function isValidUnquotedKey(key: string): boolean {
  return /^[A-Z_][\w.]*$/i.test(key)
}

export function isIdentifierSegment(key: string): boolean {
  return /^[A-Z_]\w*$/i.test(key)
}

export function isSafeUnquoted(value: string): boolean {
  if (!value) return false
  if (value !== value.trim()) return false
  if (isBooleanOrNullLiteral(value) || isNumericLike(value)) return false
  if (value.includes(" ")) return false
  if (value.includes("=")) return false
  if (value.includes("(") || value.includes(")")) return false
  if (value.includes("[") || value.includes("]")) return false
  if (value.includes('"') || value.includes("\\")) return false
  if (/[\u0000-\u001F]/.test(value)) return false
  return true
}

function isNumericLike(value: string): boolean {
  return NUMERIC_LIKE_PATTERN.test(value)
}
