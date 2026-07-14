import type { Depth, JsonPrimitive, JsonStreamEvent, DecodeStreamOptions } from "../types.ts"
import type { ParsedLine, TableHeader } from "./parser.ts"
import {
  DxDecodeError, withLine,
  tryParseKeyValue, tryParseTableHeader, isTableHeader, startsObjectBlock,
  parseTableRowValues, parseRawValue, findUnquotedChar, splitInlineTableRows,
} from "./parser.ts"
import { scanLines, computeDepth, stripIndent } from "./scanner.ts"
import { CLOSE_PAREN, OPEN_PAREN, OPEN_BRACKET, EQUALS } from "../constants.ts"

interface DecoderContext { strict: boolean }

export function* decodeStreamSync(
  source: Iterable<string>,
  options?: DecodeStreamOptions,
): Generator<JsonStreamEvent> {
  const resolvedOptions: DecoderContext = { strict: options?.strict ?? true }
  const lines: ParsedLine[] = []
  let lineNumber = 0

  for (const rawLine of scanLines(source)) {
    lineNumber++
    lines.push({
      content: stripIndent(rawLine),
      raw: rawLine,
      lineNumber,
      depth: computeDepth(rawLine),
    })
  }

  if (lines.length === 0) {
    yield { type: "startObject" }
    yield { type: "endObject" }
    return
  }

  const first = lines[0]
  const firstTrimmed = first.content.trim()

  // Root empty array
  if (firstTrimmed === "[]") {
    yield { type: "startArray", length: 0 }
    yield { type: "endArray" }
    return
  }

  // Root table: [col1 col2](...
  const rootTable = tryParseTableHeader(firstTrimmed)
  if (rootTable && !rootTable.key) {
    const parenIdx = firstTrimmed.indexOf(OPEN_PAREN)
    const closeIdx = parenIdx >= 0 ? findMatchingParenInStr(firstTrimmed.slice(parenIdx + 1)) : -1
    if (closeIdx >= 0) {
      const inner = firstTrimmed.slice(parenIdx + 1, parenIdx + 1 + closeIdx).trim()
      const rowValues = splitInlineTableRows(inner, rootTable.columns.length)
      yield { type: "startArray", length: rowValues.length }
      for (const values of rowValues) {
        yield { type: "startObject" }
        for (let c = 0; c < rootTable.columns.length && c < values.length; c++) {
          yield { type: "key", key: rootTable.columns[c] }
          yield* emitValue(values[c], resolvedOptions, first)
        }
        yield { type: "endObject" }
      }
      yield { type: "endArray" }
      return
    }
    const tableEnd = findBlockEnd(lines, 1, lines.length, 0)
    if (tableEnd < 0) {
      throw withLine(first, () => { throw new DxDecodeError("Unclosed root table") })
    }
    const rows = lines.slice(1, tableEnd)
    yield* decodeTableRowsSync(rootTable, rows, resolvedOptions, first)
    return
  }

  // Root primitive array: [val1, val2, val3]
  if (firstTrimmed.startsWith(OPEN_BRACKET) && firstTrimmed.endsWith("]") && lines.length === 1) {
    const inner = firstTrimmed.slice(1, -1).trim()
    if (inner) {
      const items = inner.split(",").map(v => parseRawValue(v.trim()))
      yield { type: "startArray", length: items.length }
      for (const item of items) {
        yield { type: "primitive", value: item as JsonPrimitive }
      }
      yield { type: "endArray" }
    } else {
      yield { type: "startArray", length: 0 }
      yield { type: "endArray" }
    }
    return
  }

  yield { type: "startObject" }
  yield* decodeBlockSync(lines, 0, lines.length, 0, resolvedOptions)
  yield { type: "endObject" }
}

export function decodeFromLines(lines: Iterable<string>, options?: DecodeStreamOptions): unknown {
  const resolvedOptions: DecoderContext = { strict: options?.strict ?? true }
  const parsedLines: ParsedLine[] = []
  let lineNumber = 0

  for (const rawLine of scanLines(lines)) {
    lineNumber++
    parsedLines.push({
      content: stripIndent(rawLine),
      raw: rawLine,
      lineNumber,
      depth: computeDepth(rawLine),
    })
  }

  if (parsedLines.length === 0) return {}

  const first = parsedLines[0].content.trim()

  // Root table: [col1 col2](...
  const rootTable = tryParseTableHeader(first)
  if (rootTable && !rootTable.key) {
    const tableEnd = findBlockEnd(parsedLines, 1, parsedLines.length, 0)
    if (tableEnd < 0) throw new DxDecodeError("Unclosed root table")
    return buildTableRows(rootTable, parsedLines.slice(1, tableEnd), resolvedOptions)
  }

  // Root primitive array: [val1, val2]
  if (first.startsWith(OPEN_BRACKET) && first.endsWith("]") && parsedLines.length === 1) {
    const inner = first.slice(1, -1).trim()
    if (!inner) return []
    return inner.split(",").map(v => parseRawValue(v.trim()))
  }

  return buildValue(parsedLines, 0, parsedLines.length, 0, resolvedOptions)
}

function* decodeBlockSync(
  parsedLines: ParsedLine[],
  start: number,
  end: number,
  baseDepth: Depth,
  options: DecoderContext,
): Generator<JsonStreamEvent> {
  const seenKeys = options.strict ? new Set<string>() : undefined
  let i = start
  while (i < end) {
    const line = parsedLines[i]
    if (!line) break
    const content = line.content

    // Check for closing paren
    if (content.trim() === CLOSE_PAREN) { i++; break }

    // Check for table header: key[cols](
    const tableHeader = withLine(line, () => tryParseTableHeader(content))
    if (tableHeader) {
      const parenIdx = content.indexOf(OPEN_PAREN)
      const closeIdx = parenIdx >= 0 ? findMatchingParenInStr(content.slice(parenIdx + 1)) : -1
      if (closeIdx >= 0) {
        const inner = content.slice(parenIdx + 1, parenIdx + 1 + closeIdx).trim()
        const rowValues = splitInlineTableRows(inner, tableHeader.columns.length)
        if (tableHeader.key) {
          assertNoDuplicateKey(tableHeader.key, line, seenKeys)
          yield { type: "key", key: tableHeader.key }
        }
        yield { type: "startArray", length: rowValues.length }
        for (const values of rowValues) {
          yield { type: "startObject" }
          const seen = options.strict ? new Set<string>() : undefined
          for (let c = 0; c < tableHeader.columns.length && c < values.length; c++) {
            assertNoDuplicateKey(tableHeader.columns[c], line, seen)
            yield { type: "key", key: tableHeader.columns[c] }
            yield* emitValue(values[c], options, line)
          }
          yield { type: "endObject" }
        }
        yield { type: "endArray" }
        i++
        continue
      }
      const tableEnd = findBlockEnd(parsedLines, i + 1, end, line.depth)
      if (tableEnd < 0) {
        throw withLine(line, () => { throw new DxDecodeError("Unclosed table block") })
      }
      const rows = parsedLines.slice(i + 1, tableEnd)
      if (tableHeader.key) {
        assertNoDuplicateKey(tableHeader.key, line, seenKeys)
        yield { type: "key", key: tableHeader.key }
      }
      yield* decodeTableRowsSync(tableHeader, rows, options, line)
      i = tableEnd + 1
      continue
    }

    // Check for object block: key(
    if (startsObjectBlock(content)) {
      const parenIndex = content.indexOf(OPEN_PAREN)
      const key = content.slice(0, parenIndex).trim()
      assertNoDuplicateKey(key, line, seenKeys)
      yield { type: "key", key }

      // Inline object: key(field1=val1 ...)
      const afterParen = content.slice(parenIndex + 1)
      const closeParenIndex = findMatchingParenInStr(afterParen)
      if (closeParenIndex >= 0) {
        // Single-line inline object
        const inner = afterParen.slice(0, closeParenIndex).trim()
        yield* decodeInlineBlockSync(inner, options, line)
        i++
        continue
      }

      // Multi-line block
      const blockEnd = findBlockEnd(parsedLines, i + 1, end, line.depth)
      if (blockEnd < 0) {
        throw withLine(line, () => { throw new DxDecodeError("Unclosed object block") })
      }
      yield { type: "startObject" }
      yield* decodeBlockSync(parsedLines, i + 1, blockEnd, line.depth, options)
      yield { type: "endObject" }
      i = blockEnd + 1
      continue
    }

    // Check for key=value
    const kv = withLine(line, () => tryParseKeyValue(content))
    if (kv) {
      assertNoDuplicateKey(kv.key, line, seenKeys)
      yield { type: "key", key: kv.key }
      yield* emitValue(kv.rawValue, options, line)
      i++
      continue
    }

    // Don't know what this is — skip or error
    if (options.strict) {
      throw withLine(line, () => new DxDecodeError(`Unexpected line: "${content}"`))
    }
    i++
  }
}

function* decodeInlineBlockSync(
  inner: string,
  options: DecoderContext,
  headerLine: ParsedLine,
): Generator<JsonStreamEvent> {
  yield { type: "startObject" }
  if (inner) {
    // Parse inline fields
    const fields = parseInlineObjectFields(inner)
    const seenKeys = options.strict ? new Set<string>() : undefined
    for (const { key, rawValue } of fields) {
      assertNoDuplicateKey(key, headerLine, seenKeys)
      yield { type: "key", key }
      yield* emitValue(rawValue, options, headerLine)
    }
  }
  yield { type: "endObject" }
}

interface InlineField { key: string; rawValue: string }

function parseInlineObjectFields(content: string): InlineField[] {
  const fields: InlineField[] = []
  let i = 0
  while (i < content.length) {
    while (i < content.length && content[i] === " ") i++
    if (i >= content.length) break

    const eqIdx = findEqAtDepth0_local(content, i)
    const parenIdx = findParenAtDepth0_local(content, i)

    if (parenIdx >= 0 && (eqIdx < 0 || parenIdx < eqIdx)) {
      // key(...) pattern
      const key = content.slice(i, parenIdx).trim()
      if (!key) break
      const end = findMatchingParenEnd_local(content, parenIdx)
      const rawValue = content.slice(parenIdx, end + 1).trim()
      fields.push({ key, rawValue })
      i = findNextFieldEnd_local(content, end + 1)
    } else if (eqIdx >= 0) {
      const key = content.slice(i, eqIdx).trim()
      if (!key) break
      i = eqIdx + 1
      const valEnd = findNextFieldEnd_local(content, i)
      const rawValue = content.slice(i, valEnd).trim()
      fields.push({ key, rawValue })
      i = valEnd
    } else {
      break
    }
  }
  return fields
}

function findEqAtDepth0_local(content: string, start: number): number {
  let parenDepth = 0, bracketDepth = 0, inQ = false
  for (let i = start; i < content.length; i++) {
    const ch = content[i]
    if (ch === '"') { inQ = !inQ; continue }
    if (inQ) continue
    if (ch === "(") { parenDepth++; continue }
    if (ch === ")") { parenDepth--; continue }
    if (ch === "[") { bracketDepth++; continue }
    if (ch === "]") { bracketDepth--; continue }
    if (parenDepth === 0 && bracketDepth === 0 && ch === "=") return i
  }
  return -1
}

function findParenAtDepth0_local(content: string, start: number): number {
  let bracketDepth = 0, inQ = false
  for (let i = start; i < content.length; i++) {
    const ch = content[i]
    if (ch === '"') { inQ = !inQ; continue }
    if (inQ) continue
    if (ch === "[") { bracketDepth++; continue }
    if (ch === "]") { bracketDepth--; continue }
    if (ch === "(" && bracketDepth === 0) return i
  }
  return -1
}

function findMatchingParenEnd_local(content: string, start: number): number {
  let depth = 1, inQ = false
  for (let i = start + 1; i < content.length; i++) {
    if (content[i] === '"') { inQ = !inQ; continue }
    if (inQ) continue
    if (content[i] === "(") depth++
    else if (content[i] === ")") { depth--; if (depth === 0) return i }
  }
  return start
}

function findNextFieldEnd_local(content: string, start: number): number {
  let parenDepth = 0, bracketDepth = 0, inQ = false
  for (let i = start; i < content.length; i++) {
    const ch = content[i]
    if (ch === '"') { inQ = !inQ; continue }
    if (inQ) continue
    if (ch === "(") { parenDepth++; continue }
    if (ch === ")") { parenDepth--; continue }
    if (ch === "[") { bracketDepth++; continue }
    if (ch === "]") { bracketDepth--; continue }
    if (parenDepth === 0 && bracketDepth === 0 && ch === " ") {
      if (isNextFieldSep_local(content, i)) return i
    }
  }
  return content.length
}

function isNextFieldSep_local(content: string, spacePos: number): boolean {
  let j = spacePos + 1
  while (j < content.length && content[j] === " ") j++
  if (j >= content.length) return false

  const eqPos = findNextEqDepth0_local(content, j)
  if (eqPos > j) {
    const kp = content.slice(j, eqPos).trim()
    if (kp && !kp.includes(" ") && !kp.includes("(")) return true
  }

  const parenPos = findNextParenDepth0_local(content, j)
  if (parenPos > j) {
    const kp = content.slice(j, parenPos).trim()
    if (kp && !kp.includes(" ") && !kp.includes("(")) return true
  }

  return false
}

function findNextEqDepth0_local(content: string, start: number): number {
  let parenDepth = 0, bracketDepth = 0, inQ = false
  for (let i = start; i < content.length; i++) {
    const ch = content[i]
    if (ch === '"') { inQ = !inQ; continue }
    if (inQ) continue
    if (ch === "(") { parenDepth++; continue }
    if (ch === ")") { parenDepth--; continue }
    if (ch === "[") { bracketDepth++; continue }
    if (ch === "]") { bracketDepth--; continue }
    if (parenDepth === 0 && bracketDepth === 0 && ch === "=") return i
  }
  return -1
}

function findNextParenDepth0_local(content: string, start: number): number {
  let parenDepth = 0, bracketDepth = 0, inQ = false
  for (let i = start; i < content.length; i++) {
    const ch = content[i]
    if (ch === '"') { inQ = !inQ; continue }
    if (inQ) continue
    if (ch === "(") {
      if (parenDepth === 0 && bracketDepth === 0) return i
      parenDepth++
      continue
    }
    if (ch === ")") { parenDepth--; continue }
    if (ch === "[") { bracketDepth++; continue }
    if (ch === "]") { bracketDepth--; continue }
  }
  return -1
}

function findMatchingParenInStr(content: string): number {
  let depth = 1
  let inQuote = false
  for (let i = 0; i < content.length; i++) {
    const ch = content[i]
    if (ch === '"') { inQuote = !inQuote; continue }
    if (inQuote) continue
    if (ch === "(") depth++
    if (ch === ")") { depth--; if (depth === 0) return i }
  }
  return -1
}

function* decodeTableRowsSync(
  header: TableHeader,
  rows: ParsedLine[],
  options: DecoderContext,
  headerLine: ParsedLine,
): Generator<JsonStreamEvent> {
  const colCount = header.columns.length
  yield { type: "startArray", length: rows.length }

  for (const row of rows) {
    const content = row.content.trim()
    if (!content || content === CLOSE_PAREN) continue
    const values = withLine(row, () => parseTableRowValues(content, colCount))
    yield { type: "startObject" }
    const seenKeys = options.strict ? new Set<string>() : undefined
    for (let i = 0; i < colCount; i++) {
      const key = header.columns[i]
      assertNoDuplicateKey(key, row, seenKeys)
      yield { type: "key", key }
      if (i < values.length) {
        yield* emitValue(values[i], options, row)
      } else {
        yield { type: "primitive", value: null }
      }
    }
    yield { type: "endObject" }
  }

  yield { type: "endArray" }
}

function* emitValue(rawValue: string, options: DecoderContext, line: ParsedLine): Generator<JsonStreamEvent> {
  const v = withLine(line, () => parseRawValue(rawValue))
  if (typeof v === "object" && !Array.isArray(v) && v !== null) {
    yield { type: "startObject" }
    for (const [k, val] of Object.entries(v)) {
      yield { type: "key", key: k }
      yield* emitPrimitiveValue(val, options, line)
    }
    yield { type: "endObject" }
  } else if (Array.isArray(v)) {
    yield { type: "startArray", length: v.length }
    for (const item of v) {
      yield* emitPrimitiveValue(item, options, line)
    }
    yield { type: "endArray" }
  } else {
    yield { type: "primitive", value: v as JsonPrimitive }
  }
}

function* emitPrimitiveValue(v: unknown, _options: DecoderContext, _line: ParsedLine): Generator<JsonStreamEvent> {
  if (v === null || typeof v === "string" || typeof v === "number" || typeof v === "boolean") {
    yield { type: "primitive", value: v as JsonPrimitive }
  } else if (Array.isArray(v)) {
    yield { type: "startArray", length: v.length }
    for (const item of v) {
      yield* emitPrimitiveValue(item, _options, _line)
    }
    yield { type: "endArray" }
  } else if (typeof v === "object" && v !== null) {
    yield { type: "startObject" }
    for (const [k, val] of Object.entries(v as Record<string, unknown>)) {
      yield { type: "key", key: k }
      yield* emitPrimitiveValue(val, _options, _line)
    }
    yield { type: "endObject" }
  }
}

function findBlockEnd(parsedLines: ParsedLine[], start: number, end: number, baseDepth: Depth): number {
  for (let i = start; i < end; i++) {
    const line = parsedLines[i]
    if (!line) continue
    // Check for `)` at same or lesser depth
    if (line.content.trim() === CLOSE_PAREN) {
      if (line.depth <= baseDepth) return i
    }
    // Check for non-indented content (back to parent depth)
    if (line.value !== undefined && line.depth <= baseDepth) {
      if (line.content.trim() !== CLOSE_PAREN) break
    }
  }
  // If we never found a closing `)`, scan for any unclosed pattern
  let parens = 0
  for (let i = start; i < end; i++) {
    const line = parsedLines[i]
    if (!line) continue
    if (line.content.includes("(")) parens++
    if (line.content.includes(")")) parens--
    if (parens < 0) return i
  }
  return -1
}

function assertNoDuplicateKey(key: string, line: ParsedLine, seenKeys: Set<string> | undefined): void {
  if (!seenKeys) return
  if (seenKeys.has(key)) {
    throw new DxDecodeError(`Duplicate key "${key}"`, { line: line.lineNumber, source: line.raw })
  }
  seenKeys.add(key)
}

function buildValue(
  parsedLines: ParsedLine[],
  start: number,
  end: number,
  baseDepth: Depth,
  options: DecoderContext,
): Record<string, unknown> {
  const result: Record<string, unknown> = {}
  let i = start
  while (i < end) {
    const line = parsedLines[i]
    if (!line) { i++; continue }
    const content = line.content
    if (content.trim() === CLOSE_PAREN) { i++; break }

    const tableHeader = withLine(line, () => tryParseTableHeader(content))
    if (tableHeader) {
      const parenIdx = content.indexOf(OPEN_PAREN)
      const closeIdx = parenIdx >= 0 ? findMatchingParenInStr(content.slice(parenIdx + 1)) : -1
      if (closeIdx >= 0) {
        const inner = content.slice(parenIdx + 1, parenIdx + 1 + closeIdx).trim()
        const rowValues = splitInlineTableRows(inner, tableHeader.columns.length)
        result[tableHeader.key] = rowValues.map(values => {
          const obj: Record<string, unknown> = {}
          for (let c = 0; c < tableHeader.columns.length && c < values.length; c++) {
            obj[tableHeader.columns[c]] = parseRawValue(values[c])
          }
          return obj
        })
        i++
      } else {
        const tableEnd = findBlockEnd(parsedLines, i + 1, end, line.depth)
        const rows = parsedLines.slice(i + 1, tableEnd >= 0 ? tableEnd : end)
        result[tableHeader.key] = buildTableRows(tableHeader, rows, options)
        i = (tableEnd >= 0 ? tableEnd : end) + 1
      }
      continue
    }

    if (startsObjectBlock(content)) {
      const parenIndex = content.indexOf(OPEN_PAREN)
      const key = content.slice(0, parenIndex).trim()
      const afterParen = content.slice(parenIndex + 1)
      const closeParenIndex = findMatchingParenInStr(afterParen)
      if (closeParenIndex >= 0) {
        result[key] = parseRawValue(content.slice(parenIndex, parenIndex + 1 + closeParenIndex + 1))
        i++
        continue
      }
      const blockEnd = findBlockEnd(parsedLines, i + 1, end, line.depth)
      result[key] = buildValue(parsedLines, i + 1, blockEnd >= 0 ? blockEnd : end, line.depth, options)
      i = (blockEnd >= 0 ? blockEnd : end) + 1
      continue
    }

    const kv = withLine(line, () => tryParseKeyValue(content))
    if (kv) {
      result[kv.key] = parseRawValue(kv.rawValue)
      i++
      continue
    }

    if (options.strict) {
      throw withLine(line, () => new DxDecodeError(`Unexpected line: "${content}"`))
    }
    i++
  }
  return result
}

function buildTableRows(header: TableHeader, rows: ParsedLine[], options: DecoderContext): unknown[] {
  const result: unknown[] = []
  for (const row of rows) {
    const content = row.content.trim()
    if (!content || content === CLOSE_PAREN) continue
    const values = withLine(row, () => parseTableRowValues(content, header.columns.length))
    const obj: Record<string, unknown> = {}
    for (let i = 0; i < header.columns.length; i++) {
      obj[header.columns[i]] = i < values.length ? parseRawValue(values[i]) : null
    }
    result.push(obj)
  }
  return result
}
