# DX Serializer LLM Format

Token-optimized text format for AI context windows.

---

## Three-Format Architecture

```
Human  (.sr / .dx on disk)  ─── source of truth, 2 flavors: Normal + Loose
LLM    (.llm / .compact)    ─── token-optimized, 2 flavors: Normal + Compact
Machine (.machine)          ─── binary RKYV, zero-copy runtime access
```

---

## LLM Normal Flavor

`: yml` multi-line style, auto-generated as `.llm`.

```
project:
  name: dx-os
  version: 1.0.0
```

## LLM Compact Flavor

`()` single-line minified, generated as `dx.compact`. Use `--compact` flag.

```
project(name=dx-os version=1.0.0)
```

---

## File Extensions

| Extension | Format | Flavor |
|-----------|--------|--------|
| `dx` (none) | Human | Normal |
| `.sr` | Human | Normal |
| `.llm` | LLM | Normal |
| `.compact` | LLM | Compact |
| `.machine` | Machine | — |
| `.loose` | Human | Loose |

---

## Best Practices

1. **Use Human (Normal) for `dx` / `.sr` files** — humans edit these, readability matters
2. **Use LLM (Normal) for AI context** — token-efficient `.llm` files
3. **Use LLM (Compact) for minified output** — most token-efficient `dx.compact`
4. **Use comma-separated tables** when values contain spaces (no quotes needed)
5. **Use space-separated tables** for simple values (lower token count)
