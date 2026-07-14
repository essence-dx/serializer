import type { ParsedLine } from "./parser.ts"
import { CLOSE_PAREN, OPEN_PAREN } from "../constants.ts"
import { DxDecodeError } from "./parser.ts"

export function validateParenBalance(lines: ParsedLine[]): void {
  let depth = 0
  for (const line of lines) {
    for (const ch of line.content) {
      if (ch === OPEN_PAREN) depth++
      if (ch === CLOSE_PAREN) depth--
    }
    if (depth < 0) {
      throw new DxDecodeError("Unexpected closing parenthesis", {
        line: line.lineNumber,
        source: line.raw,
      })
    }
  }
  if (depth !== 0) {
    throw new DxDecodeError("Unclosed parentheses in document")
  }
}

export function assertExpectedCount(actual: number, expected: number, label: string, line: ParsedLine): void {
  if (actual !== expected) {
    throw new DxDecodeError(
      `Expected ${expected} ${label}, got ${actual}`,
      { line: line.lineNumber, source: line.raw },
    )
  }
}
