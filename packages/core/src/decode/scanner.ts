import type { ParsedLine } from "./parser.ts"

export function* scanLines(source: Iterable<string>): Generator<string> {
  for (const line of source) {
    const trimmed = line.trim()
    if (trimmed === "" || trimmed.startsWith("#")) continue
    yield line.replace(/\r$/, "")
  }
}

export async function* scanLinesAsync(source: AsyncIterable<string>): AsyncGenerator<string> {
  for await (const line of source) {
    const trimmed = line.trim()
    if (trimmed === "" || trimmed.startsWith("#")) continue
    yield line
  }
}

export function computeDepth(line: string): number {
  let depth = 0
  for (const ch of line) {
    if (ch === " ") depth++
    else if (ch === "\t") depth += 2
    else break
  }
  return depth
}

export function stripIndent(line: string): string {
  return line.trimStart()
}
