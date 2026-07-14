import { describe, expect, it } from "vitest"
import { decode, encode } from "../src/index"

describe("JavaScript type normalization", () => {
  describe("BigInt normalization", () => {
    it("converts BigInt within safe integer range to number", () => {
      expect(encode(BigInt(123))).toBe("123")
    })
    it("converts BigInt at MAX_SAFE_INTEGER boundary to number", () => {
      expect(encode(BigInt(Number.MAX_SAFE_INTEGER))).toBe("9007199254740991")
    })
    it("converts BigInt beyond safe integer range to quoted string", () => {
      expect(encode(BigInt("9007199254740992"))).toBe('"9007199254740992"')
    })
    it("converts large BigInt to quoted decimal string", () => {
      expect(encode(BigInt("12345678901234567890"))).toBe('"12345678901234567890"')
    })
  })

  describe("Date normalization", () => {
    it("converts Date to ISO 8601 string", () => {
      const result = encode(new Date("2025-01-01T00:00:00.000Z"))
      expect(result).toBe("2025-01-01T00:00:00.000Z")
    })
    it("converts Date with milliseconds", () => {
      const result = encode(new Date("2025-11-05T12:34:56.789Z"))
      expect(result).toBe("2025-11-05T12:34:56.789Z")
    })
  })

  describe("Set normalization", () => {
    it("converts Set to array", () => {
      const input = new Set(["a", "b", "c"])
      const result = encode(input)
      expect(result).toBe("[a b c]")
    })
    it("converts empty Set to empty array", () => {
      expect(encode(new Set())).toBe("[]")
    })
  })

  describe("Map normalization", () => {
    it("converts Map with string keys to object", () => {
      const input = new Map([["a", 1], ["b", 2]])
      const result = encode(input)
      expect(result).toBe("a=1\nb=2")
    })
  })

  describe("Special values", () => {
    it("encodes -0 as 0", () => {
      expect(encode(-0)).toBe("0")
    })
    it("encodes NaN and Infinity as null", () => {
      expect(encode(NaN)).toBe("null")
      expect(encode(Infinity)).toBe("null")
      expect(encode(-Infinity)).toBe("null")
    })
    it("encodes undefined, function, and symbol as null", () => {
      expect(encode(undefined)).toBe("null")
      expect(encode(() => {})).toBe("null")
      expect(encode(Symbol("test"))).toBe("null")
    })
  })

  describe("toJSON support", () => {
    it("calls toJSON method", () => {
      const obj = { toJSON: () => ({ info: "example" }) }
      expect(encode(obj)).toBe("info=example")
    })
    it("handles toJSON returning array", () => {
      const obj = { toJSON: () => ["a", "b", "c"] }
      const result = encode(obj)
      expect(result).toBe("[a b c]")
    })
  })

  describe("Round-trip with special types", () => {
    it("round-trips Set", () => {
      const input = { items: new Set(["a", "b", "c"]) }
      const encoded = encode(input)
      const decoded = decode(encoded)
      expect(decoded).toEqual({ items: ["a", "b", "c"] })
    })
    it("round-trips Map", () => {
      const input = new Map([["x", 1], ["y", 2]])
      const encoded = encode(input)
      const decoded = decode(encoded)
      expect(decoded).toEqual({ x: 1, y: 2 })
    })
    it("round-trips Date", () => {
      const input = { created: new Date("2025-06-15T10:30:00.000Z") }
      const encoded = encode(input)
      const decoded = decode(encoded)
      expect(decoded.created).toBe("2025-06-15T10:30:00.000Z")
    })
  })
})
