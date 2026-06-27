import json
import tiktoken
import os

# ── HELPERS ──────────────────────────────────────────────

def value_to_toon(v):
    if v is None: return "null"
    if isinstance(v, bool): return "true" if v else "false"
    if isinstance(v, (int, float)): return str(v)
    if isinstance(v, str):
        escaped = v.replace("\\","\\\\").replace('"','\\"').replace("\n","\\n").replace("\r","\\r").replace("\t","\\t")
        return f'"{escaped}"'
    raise ValueError(type(v))

def json_to_toon_rs(data, indent=0):
    """TOON as defined by src/converters/toon.rs"""
    istr = "  " * indent
    out = ""
    if isinstance(data, dict):
        for k, v in data.items():
            if isinstance(v, list):
                if v and isinstance(v[0], dict):
                    fields = list(v[0].keys())
                    out += f"{istr}{k}[{len(v)}]{{{','.join(fields)}}}:\n"
                    for item in v:
                        out += f"{istr}  {','.join(value_to_toon(item[f]) for f in fields)}\n"
                else:
                    items = [value_to_toon(x) for x in v]
                    out += f"{istr}{k}[{len(v)}]: {', '.join(items)}\n"
            elif isinstance(v, dict):
                out += f"{istr}{k}\n"
                out += json_to_toon_rs(v, indent + 1)
            else:
                out += f"{istr}{k} {value_to_toon(v)}\n"
    elif isinstance(data, list):
        for item in data: out += json_to_toon_rs(item, indent)
    else:
        out += f"{istr}{value_to_toon(data)}\n"
    return out

def json_to_dx_wrapped(data):
    """DX LLM format with wrapped dataframes (spec style)"""
    lines = []
    for k, v in data.items():
        if isinstance(v, list):
            if v and isinstance(v[0], dict):
                fields = list(v[0].keys())
                lines.append(f"{k}[{' '.join(fields)}](")
                for item in v:
                    vals = []
                    for f in fields:
                        val = item[f]
                        if isinstance(val, str) and ' ' in val: vals.append(f'"{val}"')
                        elif isinstance(val, bool): vals.append('true' if val else 'false')
                        elif val is None: vals.append('null')
                        else: vals.append(str(val))
                    lines.append(' '.join(vals))
                lines.append(')')
            else:
                items = []
                for x in v:
                    if isinstance(x, str) and ' ' in x: items.append(f'"{x}"')
                    elif isinstance(x, bool): items.append('true' if x else 'false')
                    elif x is None: items.append('null')
                    else: items.append(str(x))
                lines.append(f"{k}=[{' '.join(items)}]")
        elif isinstance(v, dict):
            parts = []
            for sk, sv in v.items():
                s = f'"{sv}"' if isinstance(sv, str) and ' ' in sv else 'true' if sv is True else 'false' if sv is False else ('null' if sv is None else str(sv))
                parts.append(f"{sk}={s}")
            lines.append(f"{k}({' '.join(parts)})")
        else:
            s = f'"{v}"' if isinstance(v, str) and ' ' in v else 'true' if v is True else 'false' if v is False else ('null' if v is None else str(v))
            lines.append(f"{k}={s}")
    return '\n'.join(lines) + '\n'

def json_to_dx_inline(data):
    """DX LLM format with inline objects (CLI style)"""
    def ser(v, quote_str=True):
        if v is None: return 'null'
        if isinstance(v, bool): return 'true' if v else 'false'
        if isinstance(v, (int, float)): return str(v)
        if isinstance(v, str):
            if quote_str and ' ' in v: return f'"{v}"'
            return v
        if isinstance(v, list):
            if v and isinstance(v[0], dict):
                items = []
                for item in v:
                    parts = [f"{sk}={ser(sv)}" for sk, sv in item.items()]
                    items.append(f"[{','.join(parts)}]")
                return f"[{' '.join(items)}]"
            else:
                items = [ser(x) for x in v]
                return f"[{' '.join(items)}]"
        if isinstance(v, dict):
            parts = [f"{sk}={ser(sv)}" for sk, sv in v.items()]
            return f"{' '.join(parts)}"
    lines = []
    for k, v in data.items():
        if isinstance(v, dict):
            lines.append(f"{k}({ser(v)})")
        else:
            lines.append(f"{k}={ser(v)}")
    return '\n'.join(lines) + '\n'

# ── BENCHMARK ────────────────────────────────────────────

def benchmark(label, json_path, dx_llm_cli_path=None):
    with open(json_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    
    dx_cli = None
    if dx_llm_cli_path and os.path.exists(dx_llm_cli_path):
        with open(dx_llm_cli_path, "r", encoding="utf-8") as f:
            dx_cli = f.read()
    
    json_compact = json.dumps(data, separators=(',', ':'))
    json_pretty = json.dumps(data, indent=2)
    dx_wrapped = json_to_dx_wrapped(data)
    dx_inline = json_to_dx_inline(data)
    toon_rs = json_to_toon_rs(data)
    
    print(f"\n{'='*80}")
    print(f"  {label}")
    print(f"{'='*80}")
    
    for mname, enc_name in [("GPT-4o (o200k_base)", "o200k_base")]:
        enc = tiktoken.get_encoding(enc_name)
        
        entries = [
            ("JSON compact", json_compact, True),
            ("JSON pretty", json_pretty, True),
            ("DX LLM wrapped DF", dx_wrapped, True),
            ("DX LLM inline", dx_inline, True),
        ]
        if dx_cli:
            entries.append(("DX LLM CLI (auto)", dx_cli, False))
        entries.append(("TOON (rs converter)", toon_rs, False))
        
        print(f"\n  --- {mname} ---")
        print(f"  {'Format':<27} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10}")
        print(f"  {'-'*53}")
        
        vals = []
        for name, txt, save in entries:
            t = len(enc.encode(txt))
            b = len(txt)
            vals.append((name, t, b))
        
        js_t = vals[0][1]  # JSON compact as baseline
        for name, t, b in vals:
            pct = (1 - t/js_t)*100 if name != "JSON compact" else 0
            print(f"  {name:<27} {t:>8} {b:>8} {f'{pct:+.1f}%' if pct else '-':>10}")

# ── RUN ──────────────────────────────────────────────────

enc = tiktoken.get_encoding("o200k_base")

# Spec's own example data
spec_data = {
    "name": "MyApp",
    "version": "1.0.0",
    "tags": ["rust", "performance"],
    "users": [
        {"id": 1, "name": "Alice", "email": "alice@ex.com"},
        {"id": 2, "name": "Bob", "email": "bob@ex.com"}
    ]
}

# Use spec data directly
with open("_spec_data.json", "w") as f:
    json.dump(spec_data, f)

# Manually compute for spec data
json_c = json.dumps(spec_data, separators=(',', ':'))
json_p = json.dumps(spec_data, indent=2)
dx_w = json_to_dx_wrapped(spec_data)
dx_i = json_to_dx_inline(spec_data)
toon = json_to_toon_rs(spec_data)

enc = tiktoken.get_encoding("o200k_base")
js_t = len(enc.encode(json_c))

print(f"\n  {'Format':<30} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10}")
print(f"  {'-'*56}")
for name, txt in [("JSON compact", json_c), ("JSON pretty", json_p),
                  ("DX LLM wrapped DF", dx_w), ("DX LLM inline", dx_i),
                  ("TOON (rs converter)", toon)]:
    t = len(enc.encode(txt))
    pct = (1 - t/js_t)*100 if name != "JSON compact" else 0
    print(f"  {name:<30} {t:>8} {len(txt):>8} {f'{pct:+.1f}%':>10}")

# Also test with 50-dependency-like data to see if savings increase
print(f"\n{'#'*80}")
print(f"  LARGE REPETITIVE DATASET (50 items)")
print(f"{'#'*80}")
large_data = {
    "project": "dx-serializer",
    "version": "1.0.0",
    "packages": [{"name": f"dep-{i}", "version": f"{i}.0.{i % 5}", "enabled": i % 3 != 0} for i in range(50)]
}
json_c = json.dumps(large_data, separators=(',', ':'))
dx_w = json_to_dx_wrapped(large_data)
dx_i = json_to_dx_inline(large_data)
toon = json_to_toon_rs(large_data)

js_t = len(enc.encode(json_c))
print(f"  {'Format':<30} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10}")
print(f"  {'-'*56}")
for name, txt in [("JSON compact", json_c), ("DX LLM wrapped DF", dx_w),
                  ("DX LLM inline", dx_i), ("TOON (rs converter)", toon)]:
    t = len(enc.encode(txt))
    pct = (1 - t/js_t)*100 if name != "JSON compact" else 0
    print(f"  {name:<30} {t:>8} {len(txt):>8} {f'{pct:+.1f}%':>10}")

# 200 rows for convergence
print(f"\n{'#'*80}")
print(f"  200-ITEM TABLE")
print(f"{'#'*80}")
huge = {
    "name": "huge test",
    "items": [{"id": i, "name": f"Item {i}", "value": i * 100, "tag": "test"} for i in range(200)]
}
json_c = json.dumps(huge, separators=(',', ':'))
dx_w = json_to_dx_wrapped(huge)
dx_i = json_to_dx_inline(huge)
toon = json_to_toon_rs(huge)

js_t = len(enc.encode(json_c))
print(f"  {'Format':<30} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10}")
print(f"  {'-'*56}")
for name, txt in [("JSON compact", json_c), ("DX LLM wrapped DF", dx_w),
                  ("DX LLM inline", dx_i), ("TOON (rs converter)", toon)]:
    t = len(enc.encode(txt))
    pct = (1 - t/js_t)*100 if name != "JSON compact" else 0
    print(f"  {name:<30} {t:>8} {len(txt):>8} {f'{pct:+.1f}%':>10}")
