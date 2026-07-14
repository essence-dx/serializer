# BRUTAL TRUTHS — DX Serializer (Bun) — FINAL

> Brutally honest audit of `@dx-serializer/core` and `@dx-serializer/cli`.
> Generated 2026-07-14 from hands-on testing with `dx-token` CLI + `dx` CLI.
> **Encoder now matches and BEATS the hand-written benchmark format.**

---

## FINAL RESULTS

### Round-Trip: 9/9 PASS (100%)

| Before | After |
|--------|-------|
| 7/9 (77.8%) | **9/9 (100%)** |

### Token Efficiency (o200k_base — GPT-4o)

| Format | Tokens | vs DX |
|--------|--------|-------|
| **DX (new encoder)** | **3,154** | — |
| DX (hand-written) | 3,202 | **-1.5%** (BEATS hand-written) |
| JSON pretty | 8,153 | **-61.3%** |
| JSON compact | 4,752 | **-33.6%** |
| YAML | 5,788 | **-45.5%** |
| TOON | 4,549 | **-30.7%** |

### All Tests: 54/54 Pass

---

## Changes Made

### Files Modified (5 files, ~200 lines total)

| File | Changes |
|------|---------|
| `src/shared/validation.ts` | Added `-` to `isValidUnquotedKey` regex; restored missing `isNumericLike` check in `isSafeUnquoted` |
| `src/encode/encoders.ts` | Inline shallow objects; inline tables (all rows on one line); no table row indent; no-space-before-`(` in table cells; space-separated arrays |
| `src/decode/parser.ts` | Paren/bracket boundary in table row parser; space-separated array support in `splitArrayValues`; added `splitInlineTableRows` helper |
| `src/decode/decoders.ts` | Inline table support (same-line closing paren); fixed nested inline table consuming parent closing `)`; root table inline support |
| `test/dx-compact.test.ts` | Updated test expectations for new array/table format |
| `test/normalization.test.ts` | Updated test expectations for space-separated arrays |

### Key Bug Fixes

1. **Hyphen keys broken** (`validation.ts`): `isValidUnquotedKey` didn't allow hyphens, causing keys like `react-dom` to be quoted as `"react-dom"` which decoder couldn't handle
2. **Table type coercion** (`parser.ts`): `parseTableRowValues` stripped quotes from quoted strings, causing `"1.0"` to be decoded as number `1`
3. **isSafeUnquoted missing check** (`validation.ts`): `isNumericLike` check was missing, causing BigInt values like `9007199254740992` to be output without required quotes
4. **Nested inline tables eat parent `)`** (`decoders.ts`): `findBlockEnd` for an inline table consumed the parent block's closing `)` because depth comparison was wrong

### New Features

1. **Inline shallow objects**: Objects with only primitives/arrays now rendered as `key(f=val f=val)` instead of multi-line
2. **Inline tables**: Table rows on same line as header: `key[cols](val val val)`
3. **No-space paren boundary**: `name(args)` in table cells instead of `name (args)` — decoder splits at `(` 
4. **Space-separated arrays**: `[a b c]` instead of `[a, b, c]` — decoder handles both formats

### Per-File Token Comparison

| File | Old Encoder | New Encoder | Hand-written | Δ vs Old | Δ vs Hand |
|------|------------|------------|-------------|---------|-----------|
| small.json | 155 | **130** | 128 | -16.1% | +1.6% |
| medium.json | 588 | **502** | 494 | -14.6% | +1.6% |
| large.json | 752 | **667** | 688 | -11.3% | **-3.1%** |
| tool-schema.json | 251 | **234** | 239 | -6.8% | **-2.1%** |
| coding-assistant-tools.json | 1,337 | **1,268** | 1,289 | -5.2% | **-1.6%** |
| batch-toolcall.json | 181 | **166** | 174 | -8.3% | **-4.6%** |
| toolcall-simple.json | 33 | **21** | 21 | -36.4% | 0% |
| toolcall-multi.json | 46 | **40** | 42 | -13.0% | **-4.8%** |
| toolcall-nested.json | 140 | **126** | 127 | -10.0% | **-0.8%** |
| **TOTAL** | **3,483** | **3,154** | **3,202** | **-9.4%** | **-1.5%** |
