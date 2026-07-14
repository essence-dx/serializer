import { describe, expect, it } from "vitest"
import { encode, decode, DxDecodeError } from "../src/index"

function roundTrip(name: string, input: unknown): void {
  it(`round-trips ${name}`, () => {
    const dx = encode(input)
    const output = decode(dx)
    expect(JSON.stringify(output)).toBe(JSON.stringify(input))
  })
}

describe("DX Compact — encode", () => {
  it("encodes empty object", () => {
    expect(encode({})).toBe("")
  })

  it("encodes simple values", () => {
    expect(encode({ name: "test" })).toBe("name=test")
    expect(encode({ count: 42 })).toBe("count=42")
    expect(encode({ pi: 3.14 })).toBe("pi=3.14")
    expect(encode({ active: true })).toBe("active=true")
    expect(encode({ empty: false })).toBe("empty=false")
    expect(encode({ val: null })).toBe("val=null")
  })

  it("encodes quoted strings", () => {
    expect(encode({ greeting: "hello world" })).toBe('greeting="hello world"')
    expect(encode({ eq: "a=b" })).toBe('eq="a=b"')
  })

  it("encodes nested objects", () => {
    const result = encode({ config: { host: "localhost", port: 8080 } })
    expect(result).toContain("config(")
    expect(result).toContain("host=localhost")
    expect(result).toContain("port=8080")
  })

  it("encodes deeply nested objects", () => {
    const result = encode({ a: { b: { c: "deep" } } })
    expect(result).toContain("a(")
    expect(result).toContain("b(")
    expect(result).toContain("c=deep")
  })

  it("encodes empty nested objects", () => {
    expect(encode({ x: {} })).toBe("x=()")
  })

  it("encodes primitive arrays", () => {
    expect(encode({ tags: ["a", "b"] })).toBe("tags=[a, b]")
    expect(encode({ nums: [1, 2, 3] })).toBe("nums=[1, 2, 3]")
    expect(encode({ empty: [] })).toBe("empty=[]")
  })

  it("encodes arrays of objects as tables", () => {
    const result = encode({ items: [{ id: 1, name: "A" }, { id: 2, name: "B" }] })
    expect(result).toContain("items[id name](")
    expect(result).toContain("1 A")
    expect(result).toContain("2 B")
  })

  it("encodes tables with inline objects", () => {
    const result = encode({ tools: [{ n: "read", p: { t: "obj" } }] })
    expect(result).toContain("tools[n p](")
    expect(result).toContain("read")
    expect(result).toContain("(t=obj)")
  })

  it("encodes tables with quoted values", () => {
    const result = encode({ items: [{ name: "hello world", val: 1 }] })
    expect(result).toContain('"hello world"')
  })
})

describe("DX Compact — decode", () => {
  it("decodes empty object", () => {
    expect(decode("")).toEqual({})
  })

  it("decodes simple values", () => {
    expect(decode("name=test")).toEqual({ name: "test" })
    expect(decode("count=42")).toEqual({ count: 42 })
    expect(decode("pi=3.14")).toEqual({ pi: 3.14 })
    expect(decode("active=true")).toEqual({ active: true })
    expect(decode("val=null")).toEqual({ val: null })
  })

  it("decodes quoted strings", () => {
    expect(decode('greeting="hello world"')).toEqual({ greeting: "hello world" })
  })

  it("decodes nested objects", () => {
    const result = decode("config(\n  host=localhost\n  port=8080\n)")
    expect(result).toEqual({ config: { host: "localhost", port: 8080 } })
  })

  it("decodes deeply nested objects", () => {
    const result = decode("a(\n  b(\n    c=deep\n  )\n)")
    expect(result).toEqual({ a: { b: { c: "deep" } } })
  })

  it("decodes empty nested objects", () => {
    expect(decode("x=()")).toEqual({ x: {} })
  })

  it("decodes primitive arrays", () => {
    expect(decode("tags=[a, b, c]")).toEqual({ tags: ["a", "b", "c"] })
    expect(decode("nums=[1, 2, 3]")).toEqual({ nums: [1, 2, 3] })
  })

  it("decodes empty arrays", () => {
    expect(decode("empty=[]")).toEqual({ empty: [] })
  })

  it("decodes tables", () => {
    const result = decode("items[id name](\n  1 A\n  2 B\n)")
    expect(result).toEqual({ items: [{ id: 1, name: "A" }, { id: 2, name: "B" }] })
  })

  it("decodes tables with inline objects", () => {
    const result = decode('tools[name params](\n  read (type=object required=[path])\n)')
    expect(result).toEqual({ tools: [{ name: "read", params: { type: "object", required: ["path"] } }] })
  })

  it("decodes inline objects with nested fields", () => {
    const result = decode('items(\n  x (type=object properties(path(type=string)) required=[path])\n)')
    expect(result).toEqual({ items: { x: { type: "object", properties: { path: { type: "string" } }, required: ["path"] } } })
  })
})

describe("DX Compact — round-trip", () => {
  roundTrip("empty object", {})
  roundTrip("simple values", { name: "test", count: 42, active: true, val: null })
  roundTrip("quoted strings", { greeting: "hello world", eq: "a=b" })
  roundTrip("nested objects", { config: { host: "localhost", port: 8080 } })
  roundTrip("deeply nested", { a: { b: { c: { d: "deep" } } } })
  roundTrip("empty nested", { x: {} })
  roundTrip("primitive arrays", { tags: ["a", "b", "c"], nums: [1, 2] })
  roundTrip("empty arrays", { items: [] })
  roundTrip("tables", { items: [{ id: 1, name: "A" }, { id: 2, name: "B" }] })
  roundTrip("table single row", { items: [{ x: 1 }] })
  roundTrip("table with inline objects", {
    tools: [
      { name: "read", params: { type: "object", required: ["path"] } },
      { name: "write", params: { type: "object", required: ["path", "content"] } },
    ],
  })
  roundTrip("table with deep inline", {
    schema: {
      type: "object",
      properties: {
        path: { type: "string", description: "file path" },
        encoding: { type: "string", enum: ["utf-8", "ascii"], default: "utf-8" },
      },
      required: ["path"],
    },
  })
  roundTrip("mixed types", { n: "app", v: 1.5, b: true, nl: null, e: {}, tags: ["a"] })
  roundTrip("array of objects with empty", { items: [{ n: "a", e: {} }, { n: "b", e: { x: 1 } }] })
})

describe("DX Compact — error handling", () => {
  it("throws on unexpected line in strict mode", () => {
    expect(() => decode("name: Alice")).toThrow(DxDecodeError)
  })

  it("throws on invalid input", () => {
    expect(() => decode("x(\n  y=z\n  \n  a=b")).toThrow()
  })
})
