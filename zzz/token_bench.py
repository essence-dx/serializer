import json
import tiktoken
import os

def value_to_toon(v):
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        return str(v)
    if isinstance(v, str):
        escaped = v.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")
        return f'"{escaped}"'
    raise ValueError(f"Unknown type: {type(v)}")

def json_to_toon(data, indent=0):
    indent_str = "  " * indent
    result = ""
    if isinstance(data, dict):
        for key, val in data.items():
            if isinstance(val, list):
                if val and isinstance(val[0], dict):
                    fields = list(val[0].keys())
                    result += f"{indent_str}{key}[{len(val)}]{{{','.join(fields)}}}:\n"
                    for item in val:
                        vals = [value_to_toon(item[f]) for f in fields]
                        result += f"{indent_str}  {','.join(vals)}\n"
                else:
                    items = [value_to_toon(v) for v in val]
                    result += f"{indent_str}{key}[{len(val)}]: {', '.join(items)}\n"
            elif isinstance(val, dict):
                result += f"{indent_str}{key}\n"
                result += json_to_toon(val, indent + 1)
            else:
                result += f"{indent_str}{key} {value_to_toon(val)}\n"
    elif isinstance(data, list):
        for item in data:
            result += json_to_toon(item, indent)
    else:
        result += f"{indent_str}{value_to_toon(data)}\n"
    return result

def json_to_dx_wrapped(data):
    """Convert JSON to DX LLM format using wrapped dataframes (as in spec)"""
    lines = []
    for key, val in data.items():
        if isinstance(val, list):
            if val and isinstance(val[0], dict):
                fields = list(val[0].keys())
                lines.append(f"{key}[{' '.join(fields)}](")
                for item in val:
                    vals = []
                    for f in fields:
                        v = item[f]
                        if isinstance(v, str) and ' ' in v:
                            vals.append(f'"{v}"')
                        elif isinstance(v, bool):
                            vals.append('true' if v else 'false')
                        elif v is None:
                            vals.append('null')
                        else:
                            vals.append(str(v))
                    lines.append(' '.join(vals))
                lines.append(')')
            else:
                items = []
                for v in val:
                    if isinstance(v, str) and ' ' in v:
                        items.append(f'"{v}"')
                    elif isinstance(v, bool):
                        items.append('true' if v else 'false')
                    elif v is None:
                        items.append('null')
                    else:
                        items.append(str(v))
                lines.append(f"{key}=[{' '.join(items)}]")
        elif isinstance(val, dict):
            obj_items = []
            for k, v in val.items():
                sv = f'"{v}"' if isinstance(v, str) and ' ' in v else str(v).lower() if isinstance(v, bool) else str(v)
                obj_items.append(f"{k}={sv}")
            lines.append(f"{key}({' '.join(obj_items)})")
        else:
            sv = f'"{val}"' if isinstance(val, str) and ' ' in val else 'true' if val is True else 'false' if val is False else str(val)
            lines.append(f"{key}={sv}")
    return '\n'.join(lines) + '\n'

def benchmark_file(json_path, dx_llm_path, label):
    with open(json_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    
    with open(dx_llm_path, "r", encoding="utf-8") as f:
        dx_llm = f.read()
    
    json_compact = json.dumps(data, separators=(',', ':'))
    toon_str = json_to_toon(data)
    dx_wrapped_str = json_to_dx_wrapped(data)
    
    print(f"\n{'='*80}")
    print(f"  {label}")
    print(f"{'='*80}")
    
    for model_name, enc_name in [("GPT-4o (o200k_base)", "o200k_base"), ("GPT-4 (cl100k_base)", "cl100k_base")]:
        enc = tiktoken.get_encoding(enc_name)
        
        js_t = len(enc.encode(json_compact))
        dx_t = len(enc.encode(dx_llm))
        dxw_t = len(enc.encode(dx_wrapped_str))
        toon_t = len(enc.encode(toon_str))
        
        print(f"\n  --- {model_name} ---")
        print(f"  {'Format':<25} {'Tokens':>8} {'Bytes':>8} {'vs JSON':>10} {'vs TOON':>10}")
        print(f"  {'-'*61}")
        print(f"  {'JSON (compact)':<25} {js_t:>8} {len(json_compact):>8} {'-':>10} {'-':>10}")
        print(f"  {'DX LLM (auto)':<25} {dx_t:>8} {len(dx_llm):>8} {(1-dx_t/js_t)*100:>+9.1f}% {(1-dx_t/toon_t)*100:>+9.1f}%")
        print(f"  {'DX LLM (wrapped DF)':<25} {dxw_t:>8} {len(dx_wrapped_str):>8} {(1-dxw_t/js_t)*100:>+9.1f}% {(1-dxw_t/toon_t)*100:>+9.1f}%")
        print(f"  {'TOON':<25} {toon_t:>8} {len(toon_str):>8} {(1-toon_t/js_t)*100:>+9.1f}% {'-':>10}")
    
    print(f"\n  --- Sizes ---")
    print(f"  JSON compact:     {len(json_compact)} bytes")
    print(f"  DX LLM auto:      {len(dx_llm)} bytes")
    print(f"  DX LLM wrapped:   {len(dx_wrapped_str)} bytes")
    print(f"  TOON:             {len(toon_str)} bytes")
    
    print(f"\n  --- DX Wrapped DF Output ---")
    print(dx_wrapped_str)

# Benchmark small file
benchmark_file("sample.json", "sample-output/sample-json.llm", "SAMPLE: ZZZ Agents (small)")

# Benchmark large file
benchmark_file("sample_large.json", "sample-output/sample_large-json.llm", "SAMPLE: DX Dependencies (large)")
