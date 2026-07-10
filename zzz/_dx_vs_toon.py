import tiktoken, json, subprocess, tempfile, os, uuid
enc = tiktoken.get_encoding("o200k_base")

def dx_w(d):
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
                if isinstance(sv, str) and " " in sv: sv_out = '"' + sv + '"'
                elif sv is True: sv_out = "true"
                elif sv is False: sv_out = "false"
                elif sv is None: sv_out = "null"
                else: sv_out = str(sv)
                parts.append(sk + "=" + sv_out)
            lines.append(k + "(" + " ".join(parts) + ")")
        else:
            if isinstance(v, str) and " " in v: s = '"' + v + '"'
            elif v is True: s = "true"
            elif v is False: s = "false"
            elif v is None: s = "null"
            else: s = str(v)
            lines.append(k + "=" + s)
    return "\n".join(lines) + "\n"

def toon(d):
    tmp = os.path.join(tempfile.gettempdir(), "t" + str(uuid.uuid4()) + ".json")
    with open(tmp, "w") as f: json.dump(d, f)
    r = subprocess.run(["node", "-e", 'const {encode}=require("@toon-format/toon");const d=require("' + tmp.replace("\\", "\\\\") + '");console.log(encode(d))'], capture_output=True, text=True)
    os.unlink(tmp)
    return r.stdout.rstrip() + "\n"

datasets = [
    ("SPEC EXAMPLE (2 users, 4 fields)", {"name":"MyApp","version":"1.0.0","tags":["rust","performance"],"users":[{"id":1,"name":"Alice","email":"alice@ex.com","active":True},{"id":2,"name":"Bob","email":"bob@ex.com","active":False}]}),
]

with open("_big50.json") as f: d50 = json.load(f)
datasets.append(("50 PACKAGES (name/version/enabled)", d50))

d200 = {"name":"test","items":[{"id":i,"name":"Item "+str(i),"value":i*100,"tag":"test"} for i in range(200)]}
datasets.append(("200 ITEMS (id/name/value/tag)", d200))

for label, data in datasets:
    dx = dx_w(data)
    tn = toon(data)
    jc = json.dumps(data, separators=(",", ":"))

    dx_t = len(enc.encode(dx))
    tn_t = len(enc.encode(tn))
    jc_t = len(enc.encode(jc))

    print()
    print("=" * 75)
    print("  " + label)
    print("=" * 75)
    print()
    print(f"  {'':30} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10} {'vs TOON':>10}")
    print(f"  {'-'*66}")
    print(f"  {'JSON compact':30} {jc_t:>8} {len(jc):>8} {'-':>10} {'-':>10}")
    print(f"  {'DX LLM (wrapped DF)':30} {dx_t:>8} {len(dx):>8} {f'{(1-dx_t/jc_t)*100:+7.1f}%':>10} {f'{(1-dx_t/tn_t)*100:+7.1f}%':>10}")
    print(f"  {'TOON (@toon-format)':30} {tn_t:>8} {len(tn):>8} {f'{(1-tn_t/jc_t)*100:+7.1f}%':>10} {'-':>10}")
    
    print(f"\n  >>> DX LLM ({len(dx)} bytes, {dx_t} tokens) <<<")
    for line in dx.strip().split("\n"):
        print(f"    {line}")
    
    print(f"\n  >>> TOON ({len(tn)} bytes, {tn_t} tokens) <<<")
    for line in tn.strip().split("\n"):
        print(f"    {line}")
    
    dx_over_tn = (1 - dx_t / tn_t) * 100
    print(f"\n  >>> DX is {abs(dx_over_tn):.1f}% more token-efficient than TOON <<<")
