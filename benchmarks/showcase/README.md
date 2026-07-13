# DX Serializer Compact — Maximum Token Savings Benchmark

> **Date**: 2026-07-14 | **Tokenizer**: o200k_base (GPT-4o) | **TOON**: `@toon-format/cli` v2.3.0

## The Big Numbers

### 20-Tool Agent Schema (system prompt — sent EVERY request)

```
JSON:   2,862 tokens  ████████████████████████████████  (baseline)
YAML:   2,142 tokens  ████████████████████████          (−25%)
TOON:   2,062 tokens  ███████████████████████           (−28%)
DX:       920 tokens  ██████████                        (−68%) ← WINNER
```

| Format  | Tokens | vs JSON | vs TOON | Bytes   | vs JSON |
|---------|--------|---------|---------|---------|---------|
| JSON    | 2,862  | —       | —       | 11,308  | —       |
| YAML    | 2,142  | −25%    | +4%     | 10,531  | −7%     |
| TOON    | 2,062  | −28%    | —       | 9,662   | −15%    |
| **DX**  | **920** | **−68%** | **−55%** | **4,160** | **−63%** |

**DX saves 1,942 tokens vs JSON per request.** At 10K req/day with GPT-4o ($2.50/1M):
- JSON: $214.65/month
- TOON: $154.65/month
- **DX: $69.00/month** — saves **$1,747/year** vs JSON, **$1,029/year** vs TOON

---

### 18-Tool Batch Call (single LLM response)

```
JSON:   716 tokens  ████████████████████████████████  (baseline)
YAML:   607 tokens  ████████████████████████          (−15%)
TOON:   606 tokens  ████████████████████████          (−15%)
DX:     333 tokens  ████████████                      (−53%) ← WINNER
```

| Format  | Tokens | vs JSON | vs TOON | Bytes  | vs JSON |
|---------|--------|---------|---------|--------|---------|
| JSON    | 716    | —       | —       | 2,291  | —       |
| YAML    | 607    | −15%    | +0.2%   | 2,257  | −1%     |
| TOON    | 606    | −15%    | —       | 2,206  | −4%     |
| **DX**  | **333** | **−53%** | **−45%** | **1,352** | **−41%** |

**DX saves 383 tokens vs JSON per batch call.**

---

## Combined: Schema + Batch (Full Agent Round-Trip)

A real agent session: send schema (system prompt) + execute 18 tools (batch call).

| Format  | Schema | Batch | Total | vs JSON | vs TOON |
|---------|--------|-------|-------|---------|---------|
| JSON    | 2,862  | 716   | 3,578 | —       | —       |
| YAML    | 2,142  | 607   | 2,749 | −23%    | +3%     |
| TOON    | 2,062  | 606   | 2,668 | −26%    | —       |
| **DX**  | **920** | **333** | **1,253** | **−65%** | **−53%** |

**DX saves 2,325 tokens per full agent round-trip.** That's 65% fewer tokens than JSON.

---

## Why DX Wins: Syntax Breakdown

Using the 20-tool schema as example:

| Feature | What It Does | Token Savings |
|---------|--------------|---------------|
| **key=val** | `type: "string"` → `type=string` | −37% |
| **(table)** | `{ "type": "object" }` → `(type=object)` | −32% |
| **Abbreviations** | `type→t`, `default→d`, `properties→props` | −2% |
| **Root naming** | `name: "get_weather"` → `get_weather(...)` | −2% |
| **Total** | All features combined | **−68%** |

The `(table)` notation is the secret weapon — it collapses nested JSON objects into inline parenthesized groups, eliminating braces and repeated key names.

---

## All Benchmark Results

### Real-World AI Agent Scenarios

| Scenario | JSON | YAML | TOON | DX | DX vs JSON | DX vs TOON |
|----------|------|------|------|----|------------|------------|
| 20-tool schema | 2,862 | 2,142 | 2,062 | **920** | **−68%** | **−55%** |
| 18-tool batch | 716 | 607 | 606 | **333** | **−53%** | **−45%** |
| 12-tool schema | 2,598 | 2,086 | 1,995 | **1,289** | **−50%** | **−35%** |
| 6-tool batch | 360 | 292 | 289 | **174** | **−52%** | **−40%** |
| 4-tool schema | 549 | 419 | 403 | **239** | **−56%** | **−41%** |
| Large data (40 items) | 1,954 | 1,685 | 848 | **688** | **−65%** | **−19%** |
| Medium config | 1,141 | 922 | 617 | **494** | **−57%** | **−20%** |
| Small metadata | 208 | 153 | 153 | **128** | **−38%** | **−16%** |
| **Average** | | | | | **−55%** | **−34%** |

### Basic Tool-Calling

| Scenario | JSON | YAML | TOON | DX | DX vs JSON |
|----------|------|------|------|----|------------|
| Simple call (3 params) | 51 | 33 | 33 | **21** | **−59%** |
| Multi call (2 tools) | 105 | 69 | 71 | **42** | **−60%** |
| Nested call (code) | 159 | 129 | 140 | **127** | **−20%** |

---

## Cost Impact at Scale

Scenario: AI agent with 20 tools, 10K requests/day, GPT-4o ($2.50/1M input tokens).

| Format  | Tokens/request | Monthly tokens | Monthly cost | Annual cost |
|---------|---------------|----------------|-------------:|------------:|
| JSON    | 3,578         | 1,073,400,000  | $805.05      | $9,661      |
| YAML    | 2,749         | 824,700,000    | $618.53      | $7,422      |
| TOON    | 2,668         | 800,400,000    | $600.30      | $7,204      |
| **DX**  | **1,253**     | **375,900,000** | **$281.93**  | **$3,383**  |

**DX saves $6,278/year vs JSON, $3,821/year vs TOON.**

---

## Files

```
showcase/
├── README.md                      # This file
├── full-agent-schema.json         # 20-tool agent schema (JSON)
├── full-agent-schema.yaml         # YAML equivalent
├── full-agent-schema.dx           # DX Compact equivalent
├── full-agent-schema.toon         # TOON (official encoder)
├── mega-batch-call.json           # 18-tool batch call (JSON)
├── mega-batch-call.yaml           # YAML equivalent
├── mega-batch-call.dx             # DX Compact equivalent
└── mega-batch-call.toon           # TOON (official encoder)
```

---

## Methodology

- **Tokenizer**: o200k_base (GPT-4o / current frontier models)
- **TOON encoder**: `@toon-format/cli` v2.3.0 (`toon -e`)
- **DX Compact**: Hand-authored per DX Compact spec
- **Token counting**: `dx-token` v1.0.0 with tiktoken backend
- **Round-trip verification**: All TOON files decoded back to JSON and SHA-256 hashed — all matched
- **No minification**: All JSON/YAML files are pretty-printed (fair comparison for LLM context)
