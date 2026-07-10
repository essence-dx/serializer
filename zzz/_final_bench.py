import json, tiktoken, subprocess, os
enc = tiktoken.get_encoding("o200k_base")

def mk_wrapped(d):
    lines = []
    for k, v in d.items():
        if isinstance(v, list) and v and isinstance(v[0], dict):
            fields = list(v[0].keys())
            lines.append(k + "[" + " ".join(fields) + "](")
            for item in v:
                row = []
                for f in fields:
                    val = item[f]
                    if isinstance(val, str) and " " in val: row.append('"' + val + '"')
                    elif isinstance(val, bool): row.append("true" if val else "false")
                    elif val is None: row.append("null")
                    else: row.append(str(val))
                lines.append(" ".join(row))
            lines.append(")")
        elif isinstance(v, list):
            items = []
            for x in v:
                if isinstance(x, str) and " " in x: items.append('"' + x + '"')
                elif isinstance(x, bool): items.append("true" if x else "false")
                elif x is None: items.append("null")
                else: items.append(str(x))
            lines.append(k + "=[" + " ".join(items) + "]")
        elif isinstance(v, dict):
            parts = []
            for sk, sv in v.items():
                sv_str = '"' + sv + '"' if isinstance(sv, str) and " " in sv else ("true" if sv is True else "false" if sv is False else ("null" if sv is None else str(sv)))
                parts.append(sk + "=" + sv_str)
            lines.append(k + "(" + " ".join(parts) + ")")
        else:
            s = '"' + v + '"' if isinstance(v, str) and " " in v else ("true" if v is True else "false" if v is False else ("null" if v is None else str(v)))
            lines.append(k + "=" + s)
    return "\n".join(lines) + "\n"

def toon_from_json(d):
    """Generate TOON using real @toon-format/toon library"""
    import tempfile, uuid
    tmpname = os.path.join(tempfile.gettempdir(), "toon_" + str(uuid.uuid4()) + ".json")
    with open(tmpname, "w") as f:
        json.dump(d, f)
    script = 'const {encode}=require("@toon-format/toon");const d=require(' + json.dumps(tmpname.replace("\\", "\\\\")) + ');console.log(encode(d))'
    result = subprocess.run(["node", "-e", script], capture_output=True, text=True)
    os.unlink(tmpname)
    if result.stderr:
        print("STDERR:", result.stderr)
    return result.stdout

def bench(label, data):
    json_c = json.dumps(data, separators=(",", ":"))
    dx_w = mk_wrapped(data)
    toon_s = toon_from_json(data).rstrip() + "\n"
    
    js_t = len(enc.encode(json_c))
    dx_t = len(enc.encode(dx_w))
    toon_t = len(enc.encode(toon_s))
    
    print(f"\n{'='*75}")
    print(f"  {label}")
    print(f"{'='*75}")
    print(f"  {'Format':<30} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10} {'vs TOON':>10}")
    print(f"  {'-'*66}")
    print(f"  {'JSON compact':<30} {js_t:>8} {len(json_c):>8} {'-':>10} {'-':>10}")
    print(f"  {'DX LLM (wrapped DF)':<30} {dx_t:>8} {len(dx_w):>8} {(1-dx_t/js_t)*100:+7.1f}% {(1-dx_t/toon_t)*100:+7.1f}%")
    print(f"  {'TOON (real @toon-format)':<30} {toon_t:>8} {len(toon_s):>8} {(1-toon_t/js_t)*100:+7.1f}% {'-':>10}")
    print(f"\n  TOON output:\n{toon_s[:300]}...")

# --- SPEC EXAMPLE ---
bench("SPEC EXAMPLE", {"name":"MyApp","version":"1.0.0","tags":["rust","performance"],"users":[{"id":1,"name":"Alice","email":"alice@ex.com"},{"id":2,"name":"Bob","email":"bob@ex.com"}]})

# --- 50 ITEMS ---
with open("_big50.json") as f: d50 = json.load(f)
bench("50 ITEMS (packages)", d50)

# --- 200 ITEMS ---
d200 = {"name":"test","items":[{"id":i,"name":"Item "+str(i),"value":i*100,"tag":"test"} for i in range(200)]}
bench("200 ITEMS (table)", d200)
