"""
Four-Format Token Efficiency Showdown
  DX Serializer (LLM) vs TOON vs TONL vs Tauq

Measures actual tokens via tiktoken cl100k_base (GPT-4/GPT-3.5) and o200k_base (GPT-4o).
"""

import json, os, subprocess, tempfile, sys, time
import tiktoken

PROJECT_ROOT = os.path.dirname(os.path.abspath(__file__))
DIST_DIR     = os.path.join(PROJECT_ROOT, "essence")
os.makedirs(DIST_DIR, exist_ok=True)

TOON_CLI    = os.path.join(PROJECT_ROOT, "..", "inspirations", "toon-rust", "target", "debug", "toon.exe")
TAUQ_CLI    = os.path.join(PROJECT_ROOT, "..", "inspirations", "tauq", "target", "debug", "tauq.exe")
TONL_DIR    = os.path.join(PROJECT_ROOT, "..", "inspirations", "tonl")

enc_cl100k = tiktoken.get_encoding("cl100k_base")
enc_o200k  = tiktoken.get_encoding("o200k_base")

def token_count(text, enc):
    return len(enc.encode(text, disallowed_special=()))

# ---------------------------------------------------------------------------
# 1.  REAL-WORLD TEST DATASETS
# ---------------------------------------------------------------------------
datasets = {}

# Dataset 1: Simple config (small, few keys)
datasets["config"] = {
    "name": "MyService",
    "version": "1.4.2",
    "environment": "production",
    "debug": False,
    "log_level": "info",
    "max_connections": 100,
    "timeout_seconds": 30,
    "feature_flags": ["dark_mode", "beta_api", "analytics", "webhooks"],
}

# Dataset 2: User records (tabular, the bread-and-butter of LLM data)
datasets["users"] = {
    "users": [
        {"id": 1,  "name": "Alice Johnson",   "email": "alice@example.com",   "role": "admin",  "active": True,  "score": 95},
        {"id": 2,  "name": "Bob Smith",       "email": "bob@example.com",     "role": "user",   "active": True,  "score": 82},
        {"id": 3,  "name": "Carol Davis",     "email": "carol@example.com",   "role": "user",   "active": False, "score": 78},
        {"id": 4,  "name": "Dave Wilson",     "email": "dave@example.com",    "role": "editor", "active": True,  "score": 91},
        {"id": 5,  "name": "Eve Brown",       "email": "eve@example.com",     "role": "user",   "active": True,  "score": 88},
        {"id": 6,  "name": "Frank Miller",    "email": "frank@example.com",   "role": "viewer", "active": False, "score": 45},
        {"id": 7,  "name": "Grace Lee",       "email": "grace@example.com",   "role": "admin",  "active": True,  "score": 97},
        {"id": 8,  "name": "Hank Green",      "email": "hank@example.com",    "role": "user",   "active": True,  "score": 73},
        {"id": 9,  "name": "Ivy Chen",        "email": "ivy@example.com",     "role": "editor", "active": False, "score": 69},
        {"id": 10, "name": "Jack Torres",     "email": "jack@example.com",    "role": "user",   "active": True,  "score": 84},
    ]
}

# Dataset 3: Log entries (tabular timestamps)
datasets["logs"] = {
    "logs": [
        {"timestamp": "2025-06-26T10:00:00Z", "level": "INFO",  "service": "api-gateway",    "message": "Request received",      "duration_ms": 45,  "status_code": 200},
        {"timestamp": "2025-06-26T10:00:01Z", "level": "INFO",  "service": "auth-service",    "message": "Token validated",      "duration_ms": 12,  "status_code": 200},
        {"timestamp": "2025-06-26T10:00:02Z", "level": "WARN",  "service": "api-gateway",    "message": "Rate limit approaching","duration_ms": 2,   "status_code": 200},
        {"timestamp": "2025-06-26T10:00:03Z", "level": "ERROR", "service": "payment-worker", "message": "Payment timeout",       "duration_ms": 5034,"status_code": 504},
        {"timestamp": "2025-06-26T10:00:04Z", "level": "INFO",  "service": "auth-service",    "message": "User registered",      "duration_ms": 98,  "status_code": 201},
        {"timestamp": "2025-06-26T10:00:05Z", "level": "ERROR", "service": "api-gateway",     "message": "Internal server error", "duration_ms": 0,"status_code": 500},
    ]
}

# Dataset 4: Nested project config (deep objects)
datasets["project"] = {
    "project": {
        "name": "DX Serializer",
        "version": "1.0.0",
        "repository": "https://github.com/dx-www/dx-www",
        "docs": "https://docs.rs/dx-serializer",
        "keywords": ["serialization", "parser", "performance", "zero-copy", "llm", "rkyv"],
        "features": {"converters": True, "compression": True, "wasm": True, "tiktoken": True, "parallel": False},
    }
}

# Dataset 5: E-commerce (mix of tables, objects, arrays)
datasets["ecommerce"] = {
    "order": {
        "order_id": "ORD-2025-001",
        "customer": {"name": "Alice Johnson", "email": "alice@example.com", "tier": "gold"},
        "items": [
            {"sku": "LAP-001", "name": "ThinkPad X1",     "qty": 1, "price": 1299.99, "category": "electronics"},
            {"sku": "MOUSE-1", "name": "Wireless Mouse",   "qty": 2, "price": 29.99,   "category": "accessories"},
            {"sku": "USB-C-1", "name": "USB-C Hub",       "qty": 1, "price": 49.99,   "category": "accessories"},
        ],
        "shipping": {"address": "123 Main St", "city": "San Francisco", "state": "CA", "zip": "94105", "method": "express"},
        "payment": {"method": "credit_card", "last4": "4242", "status": "paid"},
        "total": 1409.96,
        "currency": "USD",
    }
}

# Dataset 6: CI pipeline config (mixed data)
datasets["ci_pipeline"] = {
    "pipeline": "web-app-build",
    "trigger": {"branches": ["main", "develop"], "events": ["push", "pull_request"]},
    "jobs": [
        {"name": "lint",       "runner": "ubuntu-latest", "steps": ["checkout", "npm install", "npm run lint"],         "timeout": 5},
        {"name": "test",       "runner": "ubuntu-latest", "steps": ["checkout", "npm install", "npm test", "npm run coverage"], "timeout": 10},
        {"name": "build",      "runner": "ubuntu-latest", "steps": ["checkout", "npm install", "npm run build"],        "timeout": 15},
        {"name": "deploy-staging", "runner": "ubuntu-latest", "steps": ["checkout", "npm run build", "deploy to staging"], "timeout": 20, "needs": ["lint", "test", "build"]},
    ],
    "notifications": {"email": ["team@example.com"], "on_failure": True},
}

# ---------------------------------------------------------------------------
# 2.  FORMAT CONVERSION HELPERS
# ---------------------------------------------------------------------------

def format_dx_text(data, name):
    """Produce DX LLM format string from JSON dict."""
    imports = []
    
    # Simple scalar/array context entries
    for k, v in data.items():
        if isinstance(v, list) and v and all(isinstance(x, (str, int, float, bool, type(None))) for x in v):
            items = " ".join(str(x).lower() if isinstance(x, bool) else json.dumps(x) if isinstance(x, str) and " " in x else str(x) for x in v)
            imports.append(f"{k}=[{items}]")
        elif isinstance(v, list) and v and all(isinstance(x, dict) for x in v):
            # Section: collect all unique keys for schema
            keys = list(dict.fromkeys(k for d in v for k in d.keys()))
            # Use unique section id char
            section_id = name[0] if name else 'a'
            section_name = k
            rows_parts = []
            for item in v:
                row = " ".join(str(item.get(key, "null")).lower() if isinstance(item.get(key), bool) else '"' + str(item.get(key)) + '"' if isinstance(item.get(key), str) and " " in str(item.get(key)) else str(item.get(key)) for key in keys)
                rows_parts.append(row)
            rows_text = "\n".join(rows_parts)
            imports.append(f"{section_name}[{' '.join(keys)}](\n{rows_text}\n)")
        elif isinstance(v, dict):
            fields = " ".join(f"{sk}={sv}" if not (isinstance(sv, str) and " " in sv) else f'{sk}="{sv}"' for sk, sv in flatten_dict(v).items())
            imports.append(f"{k}({fields})")
        elif isinstance(v, str) and " " in v:
            imports.append(f'{k}="{v}"')
        else:
            imports.append(f"{k}={str(v).lower() if isinstance(v, bool) else v}")
    
    return "\n".join(imports)

def flatten_dict(d, prefix=""):
    items = {}
    for k, v in d.items():
        fk = f"{prefix}.{k}" if prefix else k
        if isinstance(v, dict):
            items.update(flatten_dict(v, fk))
        else:
            items[fk] = v
    return items

def run_toon(json_str):
    p = subprocess.run([TOON_CLI, "-e", "--stats"], input=json_str, capture_output=True, text=True, timeout=30)
    return p.stdout.strip()

def run_tonl(json_str):
    data = json.loads(json_str)
    js = f"const t = require('./dist/index.js'); console.log(t.encodeTONL({json.dumps(data)}));"
    p = subprocess.run(["node", "-e", js], capture_output=True, text=True, timeout=30, cwd=TONL_DIR)
    return p.stdout.strip()

def run_tauq(json_str):
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
        f.write(json_str)
        in_path = f.name
    out_path = in_path + ".tqn"
    subprocess.run([TAUQ_CLI, "format", in_path, "-o", out_path], capture_output=True, timeout=30)
    with open(out_path) as f:
        result = f.read().strip()
    os.unlink(in_path)
    if os.path.exists(out_path):
        os.unlink(out_path)
    return result

def run_dx(json_str, name):
    data = json.loads(json_str)
    return format_dx_text(data, name)

# ---------------------------------------------------------------------------
# 3.  RUN BENCHMARK
# ---------------------------------------------------------------------------

print("=" * 100)
print("  TOKEN EFFICIENCY SHOWDOWN: DX Serializer vs TOON vs TONL vs Tauq")
print("=" * 100)
print()
print(f"{'Dataset':<18} {'Format':<12} {'Bytes':>8} {'cl100k':>8} {'o200k':>8} {'Char/Token':>10}")
print("-" * 100)

results = []

for ds_name, data in datasets.items():
    json_str = json.dumps(data, separators=(',', ':'))
    
    formats = {
        "JSON": json_str,
        "DX":   run_dx(json_str, ds_name),
        "TOON": run_toon(json_str),
        "TONL": run_tonl(json_str),
        "Tauq": run_tauq(json_str),
    }
    
    for fmt_name, text in formats.items():
        bytes_len = len(text.encode("utf-8"))
        c100k = token_count(text, enc_cl100k)
        o200k = token_count(text, enc_o200k)
        chars_per_tok = round(len(text) / c100k, 1) if c100k else 0
        
        print(f"{ds_name:<18} {fmt_name:<12} {bytes_len:>8} {c100k:>8} {o200k:>8} {chars_per_tok:>9.1f}")
        results.append((ds_name, fmt_name, bytes_len, c100k, o200k))
    
    print()

# ---------------------------------------------------------------------------
# 4.  SUMMARY - Savings vs JSON
# ---------------------------------------------------------------------------

print()
print("=" * 100)
print("  SAVINGS vs JSON (lower is better for bytes/tokens)")
print("=" * 100)
print()
print(f"{'Dataset':<18} {'Metric':<8} {'JSON':>8} {'DX':>8} {'TOON':>8} {'TONL':>8} {'Tauq':>8} {'Best':>10}")
print("-" * 100)

for ds_name in datasets:
    ds_results = [r for r in results if r[0] == ds_name]
    json_res = next(r for r in ds_results if r[1] == "JSON")
    
    for metric_idx, metric_name in enumerate(["Bytes", "cl100k", "o200k"]):
        json_val = json_res[metric_idx + 2]
        vals = {}
        for r in ds_results:
            if r[1] != "JSON":
                v = r[metric_idx + 2]
                pct = ((json_val - v) / json_val) * 100
                vals[r[1]] = f"{v:>6} ({pct:+.1f}%)"
        
        best_fmt = max(
            ((r[1], r[metric_idx + 2]) for r in ds_results if r[1] != "JSON"),
            key=lambda x: (json_val - x[1]) / json_val
        )[0]
        
        print(f"{ds_name:<18} {metric_name:<8} {json_val:>8} {vals.get('DX', ''):>15} {vals.get('TOON', ''):>15} {vals.get('TONL', ''):>15} {vals.get('Tauq', ''):>15} {best_fmt:>10}")

print()

# ---------------------------------------------------------------------------
# 5.  OVERALL TOTALS
# ---------------------------------------------------------------------------

print("=" * 100)
print("  OVERALL TOTALS & AVERAGES")
print("=" * 100)
print()
print(f"{'Format':<12} {'Total Bytes':>12} {'Total cl100k':>12} {'Total o200k':>12} {'Avg cl100k%':>12} {'Avg o200k%':>12}")
print("-" * 60)

formats_list = ["JSON", "DX", "TOON", "TONL", "Tauq"]
totals = {}

for fmt in formats_list:
    fmt_results = [r for r in results if r[1] == fmt]
    total_bytes = sum(r[2] for r in fmt_results)
    total_c100k = sum(r[3] for r in fmt_results)
    total_o200k = sum(r[4] for r in fmt_results)
    totals[fmt] = (total_bytes, total_c100k, total_o200k)

json_tot = totals["JSON"]
for fmt in formats_list:
    tb, tc, to = totals[fmt]
    cp = ((json_tot[1] - tc) / json_tot[1]) * 100
    op = ((json_tot[2] - to) / json_tot[2]) * 100
    print(f"{fmt:<12} {tb:>12} {tc:>12} {to:>12} {cp:>11.1f}% {op:>11.1f}%")

print()
print("=" * 100)
print("  DX vs TOON DIRECT HEAD-TO-HEAD")
print("=" * 100)
print()

ds_head = [("config", "Config"), ("users", "Users"), ("logs", "Logs"), ("project", "Project"), ("ecommerce", "E-commerce"), ("ci_pipeline", "CI Pipeline")]
print(f"{'Dataset':<18} {'DX cl100k':>10} {'TOON cl100k':>12} {'Savings':>8} {'DX o200k':>10} {'TOON o200k':>12} {'Savings':>8}")
print("-" * 80)

for ds_name, label in ds_head:
    dx_r = next(r for r in results if r[0] == ds_name and r[1] == "DX")
    tn_r = next(r for r in results if r[0] == ds_name and r[1] == "TOON")
    
    c100k_sav = ((tn_r[3] - dx_r[3]) / tn_r[3]) * 100
    o200k_sav = ((tn_r[4] - dx_r[4]) / tn_r[4]) * 100
    
    print(f"{label:<18} {dx_r[3]:>10} {tn_r[3]:>12} {c100k_sav:>7.1f}% {dx_r[4]:>10} {tn_r[4]:>12} {o200k_sav:>7.1f}%")

dx_all = [r for r in results if r[1] == "DX"]
tn_all = [r for r in results if r[1] == "TOON"]
dx_c = sum(r[3] for r in dx_all)
tn_c = sum(r[3] for r in tn_all)
dx_o = sum(r[4] for r in dx_all)
tn_o = sum(r[4] for r in tn_all)
print("-" * 80)
print(f"{'TOTAL':<18} {dx_c:>10} {tn_c:>12} {((tn_c-dx_c)/tn_c)*100:>7.1f}% {dx_o:>10} {tn_o:>12} {((tn_o-dx_o)/tn_o)*100:>7.1f}%")

print()
print("=" * 100)
print("  DX vs Tauq DIRECT HEAD-TO-HEAD")
print("=" * 100)
print()

print(f"{'Dataset':<18} {'DX cl100k':>10} {'Tauq cl100k':>12} {'Savings':>8} {'DX o200k':>10} {'Tauq o200k':>12} {'Savings':>8}")
print("-" * 80)

for ds_name, label in ds_head:
    dx_r = next(r for r in results if r[0] == ds_name and r[1] == "DX")
    tq_r = next(r for r in results if r[0] == ds_name and r[1] == "Tauq")
    
    c100k_sav = ((tq_r[3] - dx_r[3]) / tq_r[3]) * 100
    o200k_sav = ((tq_r[4] - dx_r[4]) / tq_r[4]) * 100
    
    print(f"{label:<18} {dx_r[3]:>10} {tq_r[3]:>12} {c100k_sav:>7.1f}% {dx_r[4]:>10} {tq_r[4]:>12} {o200k_sav:>7.1f}%")

tq_all = [r for r in results if r[1] == "Tauq"]
tq_c = sum(r[3] for r in tq_all)
tq_o = sum(r[4] for r in tq_all)
print("-" * 80)
print(f"{'TOTAL':<18} {dx_c:>10} {tq_c:>12} {((tq_c-dx_c)/tq_c)*100:>7.1f}% {dx_o:>10} {tq_o:>12} {((tq_o-dx_o)/tq_o)*100:>7.1f}%")

print()
print("=" * 100)
print("  RANKING (overall cl100k tokens, lower is better)")
print("=" * 100)
print()

fmt_totals = []
for fmt in formats_list:
    fr = [r for r in results if r[1] == fmt]
    tc = sum(r[3] for r in fr)
    to = sum(r[4] for r in fr)
    tb = sum(r[2] for r in fr)
    fmt_totals.append((fmt, tb, tc, to))

fmt_totals.sort(key=lambda x: x[2])
rank = 1
for fmt, tb, tc, to in fmt_totals:
    vs_json_c = ((json_tot[1] - tc) / json_tot[1]) * 100
    print(f"  {rank}. {fmt:<12} {tb:>8} bytes  {tc:>6} cl100k tokens ({vs_json_c:+.1f}% vs JSON)")
    rank += 1
