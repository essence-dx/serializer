import tiktoken
enc = tiktoken.get_encoding("o200k_base")

# ── DATA ──────────────────────────────────────────────────
small = {
    "name": "MyApp",
    "version": "1.0.0",
    "tags": ["rust", "performance"],
    "users": [
        {"id": 1, "name": "Alice", "email": "alice@ex.com", "active": True},
        {"id": 2, "name": "Bob", "email": "bob@ex.com", "active": False},
    ]
}

big50 = {
    "project": "dx-serializer",
    "version": "1.0.0",
    "packages": [{"name": f"dep-{i}", "version": f"{i}.0.{i % 5}", "enabled": i % 3 != 0} for i in range(50)],
}

big200 = {
    "name": "test",
    "items": [{"id": i, "name": f"Item {i}", "value": i * 100, "tag": "test"} for i in range(200)],
}

# ── HELPERS ───────────────────────────────────────────────
def dx_wrapped(d):
    lines = []
    for k, v in d.items():
        if isinstance(v, list) and v and isinstance(v[0], dict):
            fields = list(v[0].keys())
            lines.append(f"{k}[{' '.join(fields)}](")
            for item in v:
                row = []
                for f in fields:
                    val = item[f]
                    if isinstance(val, str) and " " in val: row.append(f'"{val}"')
                    elif isinstance(val, bool): row.append("true" if val else "false")
                    elif val is None: row.append("null")
                    else: row.append(str(val))
                lines.append(" ".join(row))
            lines.append(")")
        elif isinstance(v, list):
            items = []
            for x in v:
                if isinstance(x, str) and " " in x: items.append(f'"{x}"')
                elif isinstance(x, bool): items.append("true" if x else "false")
                elif x is None: items.append("null")
                else: items.append(str(x))
            lines.append(f"{k}=[{' '.join(items)}]")
        elif isinstance(v, dict):
            parts = []
            for sk, sv in v.items():
                if isinstance(sv, str) and " " in sv: sv_out = f'"{sv}"'
                elif sv is True: sv_out = "true"
                elif sv is False: sv_out = "false"
                elif sv is None: sv_out = "null"
                else: sv_out = str(sv)
                parts.append(f"{sk}={sv_out}")
            lines.append(f"{k}({' '.join(parts)})")
        else:
            s = f'"{v}"' if isinstance(v, str) and " " in v else ("true" if v is True else "false" if v is False else ("null" if v is None else str(v)))
            lines.append(f"{k}={s}")
    return "\n".join(lines) + "\n"

def toon_from_data(d):
    import tempfile, subprocess, os, json, uuid
    tmp = os.path.join(tempfile.gettempdir(), f"t{uuid.uuid4()}.json")
    with open(tmp, "w") as f: json.dump(d, f)
    script = f'const {{encode}}=require("@toon-format/toon");const d=require("{tmp.replace(chr(92), chr(92)+chr(92))}");console.log(encode(d))'
    r = subprocess.run(["node", "-e", script], capture_output=True, text=True)
    os.unlink(tmp)
    return r.stdout.rstrip() + "\n"

# ── COMPARE ───────────────────────────────────────────────
def compare(label, data):
    dx = dx_wrapped(data)
    toon = toon_from_data(data)
    dx_t = len(enc.encode(dx))
    toon_t = len(enc.encode(toon))
    dx_better = (1 - dx_t / toon_t) * 100
    json_t = len(enc.encode(__import__("json").dumps(data, separators=(",", ":"))))

    print(f"\n{'='*80}")
    print(f"  {label}")
    print(f"{'='*80}")
    print(f"  {'Format':<25} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10} {'vs TOON':>10}" if label == "SPEC EXAMPLE" else f"  {'Format':<25} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10} {'vs TOON':>10}")
    print(f"  {'-'*61}")
    print(f"  {'JSON compact':<25} {json_t:>8} {len(__import__('json').dumps(data, separators=(',',':'))):>8} {'-':>10} {'-':>10}")
    print(f"  {'DX LLM':<25} {dx_t:>8} {len(dx):>8} {f'{(1-dx_t/json_t)*100:+6.1f}%':>10} {f'{dx_better:+6.1f}%':>10}")
    print(f"  {'TOON':<25} {toon_t:>8} {len(toon):>8} {f'{(1-toon_t/json_t)*100:+6.1f}%':>10} {'-':>10}")
    
    print(f"\n  ┌─── DX LLM FORMAT ({len(dx)} bytes) ──────────────────────┐")
    for line in dx.strip().split("\n"):
        print(f"  │ {line}")
    print(f"  └────────────────────────────────────────────────────────┘")
    
    print(f"\n  ┌─── TOON FORMAT (@toon-format/toon v2.3) ({len(toon)} bytes) ─┐")
    for line in toon.strip().split("\n"):
        print(f"  │ {line}")
    print(f"  └───────────────────────────────────────────────────────────────┘")

    print(f"\n  ► DX beats TOON by {abs(dx_better):.1f}%")
    print(f"  ► DX beats JSON by {(1-dx_t/json_t)*100:.1f}%")
    print(f"  ► TOON beats JSON by {(1-toon_t/json_t)*100:.1f}%")

compare("SPEC EXAMPLE — 2 users with 4 fields", small)
compare("50 ITEMS — 3-field table", big50)
compare("200 ITEMS — 4-field table", big200)
