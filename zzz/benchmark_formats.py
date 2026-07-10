"""
Token Efficiency Showdown — v3 (actual CLI tools, all DX modes)
DX (default / --format / --compact) vs TOON vs TONL vs Tauq
"""

import json, os, subprocess, sys, tempfile, random
import tiktoken

PROJECT_ROOT = os.path.dirname(os.path.abspath(__file__))

DX_CLI    = os.path.join(PROJECT_ROOT, "target", "release", "dx-serialize.exe")
TOON_CLI  = os.path.join(PROJECT_ROOT, "..", "inspirations", "toon-rust", "target", "debug", "toon.exe")
TAUQ_CLI  = os.path.join(PROJECT_ROOT, "..", "inspirations", "tauq", "target", "debug", "tauq.exe")
TONL_DIR  = os.path.join(PROJECT_ROOT, "..", "inspirations", "tonl")

TMP       = os.path.join(PROJECT_ROOT, "tmp_bench")
os.makedirs(TMP, exist_ok=True)

enc_cl100k = tiktoken.get_encoding("cl100k_base")
enc_o200k  = tiktoken.get_encoding("o200k_base")

def token_count(text, enc):
    return len(enc.encode(text, disallowed_special=()))

# ---------------------------------------------------------------------------
# DATASETS
# ---------------------------------------------------------------------------
datasets = {}

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

datasets["logs"] = {
    "logs": [
        {"timestamp": "2025-06-26T10:00:00Z", "level": "INFO",  "service": "api-gateway",    "message": "Request received",      "duration_ms": 45,  "status_code": 200},
        {"timestamp": "2025-06-26T10:00:01Z", "level": "INFO",  "service": "auth-service",    "message": "Token validated",      "duration_ms": 12,  "status_code": 200},
        {"timestamp": "2025-06-26T10:00:02Z", "level": "WARN",  "service": "api-gateway",    "message": "Rate limit approaching","duration_ms": 2,   "status_code": 200},
        {"timestamp": "2025-06-26T10:00:03Z", "level": "ERROR", "service": "payment-worker", "message": "Payment timeout",       "duration_ms": 5034,"status_code": 504},
        {"timestamp": "2025-06-26T10:00:04Z", "level": "INFO",  "service": "auth-service",    "message": "User registered",      "duration_ms": 98,  "status_code": 201},
        {"timestamp": "2025-06-26T10:00:05Z", "level": "ERROR", "service": "api-gateway",    "message": "Internal server error", "duration_ms": 0,   "status_code": 500},
    ]
}

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

datasets["ci_pipeline"] = {
    "pipeline": "web-app-build",
    "trigger": {"branches": ["main", "develop"], "events": ["push", "pull_request"]},
    "jobs": [
        {"name": "lint",       "runner": "ubuntu-latest", "steps": ["checkout", "npm install", "npm run lint"],         "timeout": 5,  "needs": []},
        {"name": "test",       "runner": "ubuntu-latest", "steps": ["checkout", "npm install", "npm test", "npm run coverage"], "timeout": 10, "needs": []},
        {"name": "build",      "runner": "ubuntu-latest", "steps": ["checkout", "npm install", "npm run build"],        "timeout": 15, "needs": []},
        {"name": "deploy-staging", "runner": "ubuntu-latest", "steps": ["checkout", "npm run build", "deploy to staging"], "timeout": 20, "needs": ["lint", "test", "build"]},
    ],
    "notifications": {"email": ["team@example.com"], "on_failure": True},
}

def gen_users(n):
    names = ["Alice Johnson", "Bob Smith", "Carol Davis", "Dave Wilson", "Eve Brown",
             "Frank Miller", "Grace Lee", "Hank Green", "Ivy Chen", "Jack Torres"]
    roles = ["admin", "user", "editor", "viewer"]
    return [{
        "id": i,
        "name": random.choice(names),
        "email": f"user{i}@example.com",
        "role": random.choice(roles),
        "active": random.random() > 0.3,
        "score": random.randint(0, 100),
    } for i in range(1, n + 1)]

datasets["users_1000"] = {"users": gen_users(1000)}

# ---------------------------------------------------------------------------
# CONVERTERS
# ---------------------------------------------------------------------------

def run_dx(json_str, ds_name, extra_flags=None):
    in_path = os.path.join(TMP, f"{ds_name}.json")
    with open(in_path, "w") as f:
        f.write(json_str)
    out_dir = os.path.join(TMP, f"{ds_name}_out")
    os.makedirs(out_dir, exist_ok=True)
    cmd = [DX_CLI, in_path, "--llm-only", "--output-dir", out_dir]
    if extra_flags:
        cmd.extend(extra_flags)
    subprocess.run(cmd, capture_output=True, timeout=120)
    for fname in os.listdir(out_dir):
        if fname.endswith(".llm"):
            with open(os.path.join(out_dir, fname)) as f:
                return f.read().strip()
    return ""

def run_toon(json_str):
    p = subprocess.run([TOON_CLI, "-e"], input=json_str, capture_output=True, text=True, timeout=60)
    return p.stdout.strip()

def run_tonl(json_str):
    fd, tmp_path = tempfile.mkstemp(suffix=".json", text=True)
    with os.fdopen(fd, "w") as f:
        f.write(json_str)
    js = ("const t = require('./dist/index.js'); const fs = require('fs'); "
          "const d = JSON.parse(fs.readFileSync(process.argv[1],'utf-8')); console.log(t.encodeTONL(d));")
    p = subprocess.run(["node", "-e", js, tmp_path], capture_output=True, text=True, timeout=60, cwd=TONL_DIR)
    os.unlink(tmp_path)
    return p.stdout.strip()

def run_tauq(json_str):
    fd, tmp_path = tempfile.mkstemp(suffix=".json", text=True)
    with os.fdopen(fd, "w") as f:
        f.write(json_str)
    out_path = tmp_path + ".tqn"
    subprocess.run([TAUQ_CLI, "format", tmp_path, "-o", out_path], capture_output=True, timeout=60)
    result = ""
    if os.path.exists(out_path):
        with open(out_path) as f:
            result = f.read().strip()
        os.unlink(out_path)
    os.unlink(tmp_path)
    return result

# ---------------------------------------------------------------------------
# RUN
# ---------------------------------------------------------------------------

print("=" * 120)
print("  TOKEN EFFICIENCY SHOWDOWN — v3 (all DX modes via actual CLI)")
print("=" * 120)

dx_modes = [
    ("DX-default",  None),
    ("DX-format",   ["--format"]),
    ("DX-compact",  ["--compact"]),
]

results = []

for ds_name, data in datasets.items():
    json_str = json.dumps(data, separators=(',', ':'))

    converters = {"JSON": lambda: json_str}
    for label, flags in dx_modes:
        converters[label] = lambda js=json_str, dn=ds_name, f=flags: run_dx(js, dn, f)
    converters["TOON"] = lambda js=json_str: run_toon(js)
    converters["TONL"] = lambda js=json_str: run_tonl(js)
    converters["Tauq"] = lambda js=json_str: run_tauq(js)

    print(f"\n{ds_name}:")
    print(f"  {'Format':<16} {'Bytes':>8} {'cl100k':>8} {'o200k':>8} {'Ch/Tok':>8}")
    print(f"  {'-'*52}")
    for fmt_name, conv in converters.items():
        try:
            text = conv()
        except Exception as e:
            print(f"  {fmt_name:<16} {'ERROR: ' + str(e)[:40]:>40}")
            continue
        if not text:
            print(f"  {fmt_name:<16} {'(empty)':>8}")
            continue
        b = len(text.encode("utf-8"))
        c = token_count(text, enc_cl100k)
        o = token_count(text, enc_o200k)
        cpt = round(len(text) / c, 1) if c else 0
        print(f"  {fmt_name:<16} {b:>8} {c:>8} {o:>8} {cpt:>8.1f}")
        results.append((ds_name, fmt_name, b, c, o))

# ---------------------------------------------------------------------------
# SAVINGS vs JSON
# ---------------------------------------------------------------------------
print(f"\n{'='*120}")
print(f"  SAVINGS vs JSON (positive = fewer tokens)")
print(f"{'='*120}")
fmts = ["DX-default", "DX-format", "DX-compact", "TOON", "TONL", "Tauq"]
header = f"{'Dataset':<18} {'Metric':<8} {'JSON':>8}"
for f in fmts:
    header += f" {f:>15}"
header += f" {'Best':>10}"
print()
print(header)
print("-" * 120)

for ds_name in datasets:
    dsr = [r for r in results if r[0] == ds_name]
    jr = next(r for r in dsr if r[1] == "JSON")
    for mi, mn in enumerate(["Bytes", "cl100k"]):
        jv = jr[mi + 2]
        best_fmt, best_pct = None, -999
        line = f"{ds_name:<18} {mn:<8} {jv:>8}"
        for f in fmts:
            r = next((x for x in dsr if x[1] == f), None)
            if r:
                v = r[mi + 2]
                pct = ((jv - v) / jv) * 100
                line += f" {v:>6} ({pct:>+5.1f}%)"
                if pct > best_pct:
                    best_pct = pct
                    best_fmt = f
            else:
                line += f" {'N/A':>15}"
        line += f" {best_fmt:>10}"
        print(line)
    print()

# ---------------------------------------------------------------------------
# OVERALL TOTALS
# ---------------------------------------------------------------------------
print(f"{'='*120}")
print(f"  OVERALL TOTALS")
print(f"{'='*120}")
print(f"\n  {'Format':<16} {'Bytes':>10} {'cl100k':>10} {'o200k':>10} {'vs JSON%':>10}")
print(f"  {'-'*60}")

all_fmts = ["JSON"] + fmts
json_tc = sum(r[3] for r in results if r[1] == "JSON")
for f in all_fmts:
    fr = [r for r in results if r[1] == f]
    tb = sum(r[2] for r in fr)
    tc = sum(r[3] for r in fr)
    to = sum(r[4] for r in fr)
    pct = ((json_tc - tc) / json_tc) * 100 if json_tc else 0
    print(f"  {f:<16} {tb:>10} {tc:>10} {to:>10} {pct:>9.1f}%")

print(f"\n  {'RANKING (cl100k, lower better)':}")
ranks = [(f, sum(r[2] for r in results if r[1] == f),
           sum(r[3] for r in results if r[1] == f),
           sum(r[4] for r in results if r[1] == f))
         for f in all_fmts]
ranks.sort(key=lambda x: x[2])
for i, (f, tb, tc, to) in enumerate(ranks, 1):
    pct = ((json_tc - tc) / json_tc) * 100
    print(f"    {i}. {f:<16} {tb:>9} B  {tc:>7} cl100k ({pct:+.1f}%)")

# ---------------------------------------------------------------------------
# SMALL vs LARGE
# ---------------------------------------------------------------------------
print(f"\n{'='*120}")
print(f"  SMALL DATASETS ONLY (config + users + logs + project + ecommerce + ci_pipeline)")
print(f"{'='*120}")
print(f"\n  {'Format':<16} {'Bytes':>10} {'cl100k':>10} {'vs JSON%':>10}")
print(f"  {'-'*60}")
small_ds = ["config", "users", "logs", "project", "ecommerce", "ci_pipeline"]
small_tc = sum(r[3] for r in results if r[1] == "JSON" and r[0] in small_ds)
for f in all_fmts:
    fr = [r for r in results if r[1] == f and r[0] in small_ds]
    tb = sum(r[2] for r in fr)
    tc = sum(r[3] for r in fr)
    pct = ((small_tc - tc) / small_tc) * 100 if small_tc else 0
    print(f"  {f:<16} {tb:>10} {tc:>10} {pct:>9.1f}%")

print(f"\n  {'LARGE DATASET ONLY (users_1000)':}")
large_ds = ["users_1000"]
large_tc = sum(r[3] for r in results if r[1] == "JSON" and r[0] in large_ds)
for f in all_fmts:
    fr = [r for r in results if r[1] == f and r[0] in large_ds]
    tb = sum(r[2] for r in fr)
    tc = sum(r[3] for r in fr)
    pct = ((large_tc - tc) / large_tc) * 100 if large_tc else 0
    print(f"  {f:<16} {tb:>10} {tc:>10} {pct:>9.1f}%")
