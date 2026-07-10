import json, tiktoken
enc = tiktoken.get_encoding("o200k_base")
with open("_big50.json") as f: data = json.load(f)
with open("sample-output/big50.toon") as f: toon = f.read()

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
                if isinstance(sv, str) and " " in sv: s = '"' + sv + '"'
                elif isinstance(sv, bool): s = "true" if sv else "false"
                elif sv is None: s = "null"
                else: s = str(sv)
                parts.append(sk + "=" + s)
            lines.append(k + "(" + " ".join(parts) + ")")
        else:
            if isinstance(v, str) and " " in v: s = '"' + v + '"'
            elif isinstance(v, bool): s = "true" if v else "false"
            elif v is None: s = "null"
            else: s = str(v)
            lines.append(k + "=" + s)
    return "\n".join(lines) + "\n"

json_c = json.dumps(data, separators=(",", ":"))
dx_w = mk_wrapped(data)
js_t = len(enc.encode(json_c))
dx_t = len(enc.encode(dx_w))
toon_t = len(enc.encode(toon))

print(f"  {'Format':<30} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10}")
print(f"  {'-'*56}")
print(f"  {'JSON compact':<30} {js_t:>8} {len(json_c):>8} {'-':>10}")
print(f"  {'DX LLM wrapped DF':<30} {dx_t:>8} {len(dx_w):>8} {(1-dx_t/js_t)*100:+7.1f}%")
print(f"  {'TOON (real spec)':<30} {toon_t:>8} {len(toon):>8} {(1-toon_t/js_t)*100:+7.1f}%")
