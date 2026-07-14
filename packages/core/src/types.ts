export type JsonPrimitive = string | number | boolean | null
export type JsonObject = { [Key in string]: JsonValue }
export type JsonArray = JsonValue[]
export type JsonValue = JsonPrimitive | JsonObject | JsonArray

export type EncodeReplacer = (
  key: string,
  value: JsonValue,
  path: readonly (string | number)[],
) => unknown

export interface EncodeOptions {
  indent?: number
  replacer?: EncodeReplacer
}

export interface ResolvedEncodeOptions {
  indent: number
  replacer?: EncodeReplacer
}

export interface DecodeOptions {
  strict?: boolean
  expandPaths?: "off" | "safe"
}

export interface DecodeStreamOptions {
  strict?: boolean
}

export type Depth = number

export type JsonStreamEvent =
  | { type: "startObject" }
  | { type: "endObject" }
  | { type: "startArray"; length?: number }
  | { type: "endArray" }
  | { type: "key"; key: string; wasQuoted?: boolean }
  | { type: "primitive"; value: JsonPrimitive }
