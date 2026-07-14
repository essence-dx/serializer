import type { JsonPrimitive, JsonValue } from "../types.ts"
import {
  EQUALS, OPEN_PAREN, CLOSE_PAREN, OPEN_BRACKET, CLOSE_BRACKET,
  SPACE, COMMA, DOUBLE_QUOTE,
} from "../constants.ts"
import { unescapeString } from "../shared/string-utils.ts"
import { isBooleanOrNullLiteral, isNumericLiteral } from "../shared/literal-utils.ts"

export interface ParsedLine {
  content: string
  raw: string
  lineNumber: number
  depth: number
}

export interface TableHeader {
  key: string
  columns: string[]
}

export class DxDecodeError extends SyntaxError {
  readonly line?: number
  readonly source?: string

  constructor(message: string, context?: { line?: number; source?: string; cause?: unknown }) {
    const prefix = context?.line !== undefined ? `Line ${context.line}: ` : ""
    super(prefix + message, context?.cause !== undefined ? { cause: context.cause } : undefined)
    this.name = "DxDecodeError"
    this.line = context?.line
    this.source = context?.source
  }
}

export function withLine<T>(line: ParsedLine, fn: () => T): T {
  try {
    return fn()
  } catch (error) {
    if (error instanceof DxDecodeError) throw error
    if (error instanceof Error) {
      throw new DxDecodeError(error.message, {
        line: line.lineNumber,
        source: line.raw,
        cause: error,
      })
    }
    throw error
  }
}

// Try to parse a key=value from content. Returns null if no `=` found.
export function tryParseKeyValue(content: string): { key: string; rawValue: string } | null {
  const eqIndex = findUnquotedChar(content, EQUALS)
  if (eqIndex < 0) return null
  const key = content.slice(0, eqIndex).trim()
  if (!key) return null
  const rawValue = content.slice(eqIndex + 1).trim()
  return { key, rawValue }
}

// Try to parse a table header: key[col1 col2]( or [col1 col2]( (root array)
export function tryParseTableHeader(content: string): TableHeader | null {
  const bracketIndex = findUnquotedChar(content, OPEN_BRACKET)
  if (bracketIndex < 0) return null
  const key = content.slice(0, bracketIndex).trim()
  // key is optional — root arrays have no key
  const rest = content.slice(bracketIndex + 1)
  const closeBracketIndex = rest.indexOf(CLOSE_BRACKET)
  if (closeBracketIndex < 0) return null
  const colsRaw = rest.slice(0, closeBracketIndex).trim()
  const columns = parseColumns(colsRaw)
  if (columns.length === 0) return null
  const afterBracket = rest.slice(closeBracketIndex + 1).trim()
  if (!afterBracket.startsWith(OPEN_PAREN)) return null
  return { key, columns }
}

// Check if a line is a table header
export function isTableHeader(content: string): boolean {
  return tryParseTableHeader(content) !== null
}

// Check if line starts an object block: key(  (non-table)
export function startsObjectBlock(content: string): boolean {
  const bracketIndex = findUnquotedChar(content, OPEN_BRACKET)
  if (bracketIndex >= 0) {
    const beforeBracket = content.slice(0, bracketIndex).trim()
    // Only reject if [ is followed by columns pattern (table header), not array values
    // If [ is at depth 0 AND the content after looks like a table...
    if (beforeBracket.length > 0 || bracketIndex === 0) {
      const rest = content.slice(bracketIndex + 1)
      const closeBracket = rest.indexOf(CLOSE_BRACKET)
      if (closeBracket >= 0 && rest.slice(closeBracket + 1).trimStart().startsWith(OPEN_PAREN)) {
        return false // This is a table header, not an object block
      }
    }
  }
  const parenIndex = findUnquotedChar(content, OPEN_PAREN)
  if (parenIndex < 0) return false
  const before = content.slice(0, parenIndex).trim()
  if (before.length === 0) return false
  if (before.includes(EQUALS)) return false
  return true
}

// Parse columns from header: "col1 col2 col3" → ["col1", "col2", "col3"]
function parseColumns(raw: string): string[] {
  const cols: string[] = []
  let current = ""
  let inQuote = false
  for (const ch of raw) {
    if (ch === DOUBLE_QUOTE) { inQuote = !inQuote; current += ch; continue }
    if (ch === SPACE && !inQuote) {
      if (current) { cols.push(current.trim()); current = "" }
      continue
    }
    current += ch
  }
  if (current) cols.push(current.trim())
  return cols
}

// Split inline table content (rows on one line) into per-row value arrays
export function splitInlineTableRows(content: string, columnCount: number): string[][] {
  const rows: string[][] = []
  let pos = 0
  while (pos < content.length) {
    while (pos < content.length && content[pos] === SPACE) pos++
    if (pos >= content.length) break
    const slice = content.slice(pos)
    const values = parseTableRowValues(slice, columnCount)
    if (values.length < columnCount) {
      if (values.length > 0) rows.push(values)
      break
    }
    rows.push(values)
    let advanced = 0
    let remaining = columnCount
    let j = 0
    while (j < slice.length && remaining > 0) {
      while (j < slice.length && slice[j] === SPACE) j++
      if (j >= slice.length) break
      if (slice[j] === DOUBLE_QUOTE) {
        const end = findClosingQuote(slice, j)
        j = end > 0 ? end + 1 : slice.length
      } else if (slice[j] === OPEN_PAREN) {
        const end = findMatchingParen(slice, j)
        j = end > 0 ? end + 1 : slice.length
      } else if (slice[j] === OPEN_BRACKET) {
        const end = findMatchingBracket(slice, j)
        j = end > 0 ? end + 1 : slice.length
      } else {
        while (j < slice.length && slice[j] !== SPACE && slice[j] !== OPEN_PAREN && slice[j] !== OPEN_BRACKET) j++
      }
      remaining--
    }
    pos += j
  }
  return rows
}

// Parse a table row into primitive values, given column count.
// Respects quoted strings and paren blocks as atomic values.
export function parseTableRowValues(content: string, columnCount: number): string[] {
  const values: string[] = []
  let i = 0
  while (i < content.length && values.length < columnCount) {
    // Skip leading whitespace between values
    while (i < content.length && content[i] === SPACE) i++
    if (i >= content.length) break
    const ch = content[i]
    if (ch === DOUBLE_QUOTE) {
      const end = findClosingQuote(content, i)
      if (end < 0) { values.push(content.slice(i)); break }
      values.push(content.slice(i, end + 1))
      i = end + 1
    } else if (ch === OPEN_PAREN) {
      const end = findMatchingParen(content, i)
      if (end < 0) { values.push(content.slice(i)); break }
      values.push(content.slice(i, end + 1))
      i = end + 1
    } else if (ch === OPEN_BRACKET) {
      const end = findMatchingBracket(content, i)
      if (end < 0) { values.push(content.slice(i)); break }
      values.push(content.slice(i, end + 1))
      i = end + 1
    } else {
      let start = i
      while (i < content.length && content[i] !== SPACE) {
        if ((content[i] === OPEN_PAREN || content[i] === OPEN_BRACKET) && values.length < columnCount - 1) {
          break
        }
        i++
      }
      values.push(content.slice(start, i))
    }
  }
  return values
}

// Parse an inline object content: "field1=val1 field2=val2 nested(key=val)"
// Returns array of [key, rawValue] pairs
export function parseInlineObject(content: string): Array<{ key: string; rawValue: string }> {
  const fields: Array<{ key: string; rawValue: string }> = []
  let i = 0
  while (i < content.length) {
    while (i < content.length && content[i] === SPACE) i++
    if (i >= content.length) break
    const eqIndex = findEqAtDepth0(content, i)
    const parenIndex = findParenAtDepth0(content, i)
    if (eqIndex >= 0 && (parenIndex < 0 || eqIndex < parenIndex)) {
      // key=value pattern
      const key = content.slice(i, eqIndex).trim()
      if (!key) break
      i = eqIndex + 1
      const valEnd = findNextFieldStart(content, i)
      const rawValue = content.slice(i, valEnd).trim()
      fields.push({ key, rawValue })
      i = valEnd
    } else if (parenIndex >= 0 && (eqIndex < 0 || parenIndex < eqIndex)) {
      // key(...) pattern (nested object, no = between key and parens)
      const key = content.slice(i, parenIndex).trim()
      if (!key) break
      let pDepth = 1
      let j = parenIndex + 1
      let inQ = false
      while (j < content.length && pDepth > 0) {
        if (content[j] === '"') inQ = !inQ
        else if (!inQ) { if (content[j] === "(") pDepth++; else if (content[j] === ")") pDepth-- }
        j++
      }
      const rawValue = content.slice(parenIndex, j).trim()
      fields.push({ key, rawValue })
      i = findNextFieldStart(content, j)
    } else {
      break
    }
  }
  return fields
}

function findEqAtDepth0(content: string, start: number): number {
  let depth = 0
  let bDepth = 0
  let inQ = false
  for (let i = start; i < content.length; i++) {
    const ch = content[i]
    if (ch === '"') { inQ = !inQ; continue }
    if (inQ) continue
    if (ch === "(") { depth++; continue }
    if (ch === ")") { depth--; continue }
    if (ch === "[") { bDepth++; continue }
    if (ch === "]") { bDepth--; continue }
    if (depth === 0 && bDepth === 0 && ch === "=") return i
  }
  return -1
}

function findParenAtDepth0(content: string, start: number): number {
  let depth = 0
  let bDepth = 0
  let inQ = false
  for (let i = start; i < content.length; i++) {
    const ch = content[i]
    if (ch === '"') { inQ = !inQ; continue }
    if (inQ) continue
    if (depth === 0 && bDepth === 0 && ch === "(") return i
    if (ch === "(") { depth++; continue }
    if (ch === ")") { depth--; continue }
    if (ch === "[") { bDepth++; continue }
    if (ch === "]") { bDepth--; continue }
  }
  return -1
}

// Find the start of the next field in an inline object
function findNextFieldStart(content: string, start: number): number {
  let depth = 0
  let bDepth = 0
  let inQ = false
  for (let i = start; i < content.length; i++) {
    const ch = content[i]
    if (ch === '"') { inQ = !inQ; continue }
    if (inQ) continue
    if (ch === "(") { depth++; continue }
    if (ch === ")") { depth--; continue }
    if (ch === "[") { bDepth++; continue }
    if (ch === "]") { bDepth--; continue }
    if (depth === 0 && bDepth === 0 && ch === " ") {
      if (isNextKeyStart(content, i)) return i
    }
  }
  return content.length
}

function isNextKeyStart(content: string, spacePos: number): boolean {
  let j = spacePos + 1
  while (j < content.length && content[j] === " ") j++
  if (j >= content.length) return false

  // Check key=val pattern
  const eqPos = findEqAtDepth0(content, j)
  if (eqPos > j) {
    const keyPart = content.slice(j, eqPos).trim()
    if (keyPart && !keyPart.includes(" ") && !keyPart.includes("(")) return true
  }

  // Check key(...) pattern
  const parenPos = findParenAtDepth0(content, j)
  if (parenPos > j) {
    const keyPart = content.slice(j, parenPos).trim()
    if (keyPart && !keyPart.includes(" ") && !keyPart.includes("(")) return true
  }

  return false
}

// Parse a raw value string into a JsonValue
export function parseRawValue(raw: string): JsonValue {
  // Quoted string — check before trimming to preserve trailing spaces
  const trimmed = raw.trim()
  if (!trimmed) return raw

  if (trimmed.startsWith(DOUBLE_QUOTE)) {
    const end = findClosingQuote(trimmed, 0)
    if (end >= 0) {
      return unescapeString(trimmed.slice(1, end))
    }
    return unescapeString(trimmed.slice(1))
  }

  // Inline object: (...)
  if (trimmed.startsWith(OPEN_PAREN)) {
    const inner = trimmed.slice(1, -1).trim()
    const fields = parseInlineObject(inner)
    const obj: Record<string, JsonValue> = {}
    for (const { key, rawValue } of fields) {
      obj[key] = parseRawValue(rawValue)
    }
    return obj
  }

  // Inline array: [...]
  if (trimmed.startsWith(OPEN_BRACKET)) {
    const inner = trimmed.slice(1, -1).trim()
    if (!inner) return []
    const items = splitArrayValues(inner)
    return items.map(v => parseRawValue(v.trim()))
  }

  // Boolean/null
  if (isBooleanOrNullLiteral(trimmed)) {
    if (trimmed === "true") return true
    if (trimmed === "false") return false
    return null
  }

  // Number
  if (isNumericLiteral(trimmed)) {
    const num = Number(trimmed)
    if (!Number.isNaN(num)) return num
  }

  // String — preserve original (non-trimmed) value
  return raw
}

// Split array content by comma (respecting nesting)
function splitArrayValues(content: string): string[] {
  const items: string[] = []
  let current = ""
  let depth = 0
  let inQuote = false
  for (const ch of content) {
    if (ch === DOUBLE_QUOTE) { inQuote = !inQuote; current += ch; continue }
    if (inQuote) { current += ch; continue }
    if (ch === OPEN_PAREN || ch === OPEN_BRACKET) { depth++; current += ch; continue }
    if (ch === CLOSE_PAREN || ch === CLOSE_BRACKET) { depth--; current += ch; continue }
    if (depth === 0 && (ch === COMMA || ch === SPACE)) {
      if (current.trim()) {
        items.push(current.trim())
        current = ""
      }
      continue
    }
    current += ch
  }
  if (current.trim()) items.push(current.trim())
  return items
}

export function findClosingQuote(content: string, start: number): number {
  let i = start + 1
  while (i < content.length) {
    if (content[i] === "\\" && i + 1 < content.length) { i += 2; continue }
    if (content[i] === '"') return i
    i++
  }
  return -1
}

export function findMatchingParen(content: string, start: number): number {
  let depth = 1
  let i = start + 1
  let inQuote = false
  while (i < content.length) {
    if (content[i] === '"') { inQuote = !inQuote; i++; continue }
    if (!inQuote) {
      if (content[i] === OPEN_PAREN) depth++
      else if (content[i] === CLOSE_PAREN) { depth--; if (depth === 0) return i }
    }
    i++
  }
  return -1
}

export function findMatchingBracket(content: string, start: number): number {
  let depth = 1
  let i = start + 1
  let inQuote = false
  while (i < content.length) {
    if (content[i] === '"') { inQuote = !inQuote; i++; continue }
    if (!inQuote) {
      if (content[i] === OPEN_BRACKET) depth++
      else if (content[i] === CLOSE_BRACKET) { depth--; if (depth === 0) return i }
    }
    i++
  }
  return -1
}

export function findUnquotedChar(content: string, char: string): number {
  return findUnquotedCharAt(content, char, 0)
}

export function findUnquotedCharAt(content: string, char: string, start: number): number {
  let inQuote = false
  let parenDepth = 0
  let bracketDepth = 0
  let i = start
  while (i < content.length) {
    const ch = content[i]
    if (ch === '"') { inQuote = !inQuote; i++; continue }
    if (inQuote) { i++; continue }
    if (parenDepth === 0 && bracketDepth === 0 && ch === char) return i
    if (ch === OPEN_PAREN) { parenDepth++; i++; continue }
    if (ch === CLOSE_PAREN) { parenDepth--; i++; continue }
    if (ch === OPEN_BRACKET) { bracketDepth++; i++; continue }
    if (ch === CLOSE_BRACKET) { bracketDepth--; i++; continue }
    i++
  }
  return -1
}

// Check if content is a closing paren for a block
export function isCloseParen(content: string): boolean {
  return content.trim() === CLOSE_PAREN
}
