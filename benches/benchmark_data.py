"""
Make a dataset that you can run through JSON, YAML, TOON, and DX LLM, 
then we use dx-token to count tokens per format.
"""

import json, os

DIST = os.path.dirname(os.path.abspath(__file__))

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
        {"timestamp": "2025-06-26T10:00:05Z", "level": "ERROR", "service": "api-gateway",     "message": "Internal server error", "duration_ms": 0,"status_code": 500},
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

datasets["deps50"] = {
    "project": "app",
    "version": "1.0.0",
    "dependencies": [{"name": f"dep-{i}", "version": f"{i}.0.{i % 5}", "enabled": i % 3 != 0} for i in range(50)],
}

# ──────────────────────────────────────────────
#  JSON
# ──────────────────────────────────────────────
for name, data in datasets.items():
    path = os.path.join(DIST, f"{name}.json")
    with open(path, "w") as f:
        json.dump(data, f, indent=2)
    path_c = os.path.join(DIST, f"{name}.jsonc")
    with open(path_c, "w") as f:
        json.dump(data, f, separators=(",", ":"))

# ──────────────────────────────────────────────
#  YAML
# ──────────────────────────────────────────────
try:
    import yaml
    for name, data in datasets.items():
        path = os.path.join(DIST, f"{name}.yml")
        with open(path, "w") as f:
            yaml.dump(data, f, default_flow_style=False, sort_keys=False)
except ImportError:
    print("PyYAML not installed — skipping YAML files")

# ──────────────────────────────────────────────
#  TOON
# ──────────────────────────────────────────────
def toon_encode(d, indent=0):
    """Emulate @toon-format/toon v2.x output."""
    pad = "  " * indent
    lines = []
    if isinstance(d, dict):
        for k, v in d.items():
            if isinstance(v, dict):
                lines.append(f"{pad}{k}:")
                lines.append(toon_encode(v, indent + 1))
            elif isinstance(v, list):
                if v and isinstance(v[0], dict):
                    lines.append(f"{pad}{k}:")
                    for item in v:
                        lines.append(f"{pad}-")
                        for sk, sv in item.items():
                            if isinstance(sv, str) and " " in sv:
                                lines.append(f"{pad}  {sk}: \"{sv}\"")
                            elif isinstance(sv, bool):
                                lines.append(f"{pad}  {sk}: {'true' if sv else 'false'}")
                            elif sv is None:
                                lines.append(f"{pad}  {sk}: null")
                            else:
                                lines.append(f"{pad}  {sk}: {sv}")
                else:
                    items = []
                    for x in v:
                        if isinstance(x, str) and " " in x: items.append(f'"{x}"')
                        elif isinstance(x, bool): items.append("true" if x else "false")
                        elif x is None: items.append("null")
                        else: items.append(str(x))
                    lines.append(f"{pad}{k}: [{', '.join(items)}]")
            elif isinstance(v, bool):
                lines.append(f"{pad}{k}: {'true' if v else 'false'}")
            elif v is None:
                lines.append(f"{pad}{k}: null")
            elif isinstance(v, str) and " " in v:
                lines.append(f'{pad}{k}: "{v}"')
            else:
                lines.append(f"{pad}{k}: {v}")
    return "\n".join(lines)

for name, data in datasets.items():
    path = os.path.join(DIST, f"{name}.toon")
    with open(path, "w") as f:
        f.write(toon_encode(data) + "\n")

# ──────────────────────────────────────────────
#  DX LLM format — uses our ConfigFormatter
# ──────────────────────────────────────────────
def dx_value(v, indent=0):
    """Recursively format a value in DX LLM format."""
    pad = "  " * indent
    if isinstance(v, bool):
        return "true" if v else "false"
    if v is None:
        return "null"
    if isinstance(v, str):
        return f'"{v}"' if " " in v else v
    if isinstance(v, (int, float)):
        return str(v)
    if isinstance(v, list):
        if v and isinstance(v[0], dict):
            # Table — this would need schema context, fallback: inline array of objects
            parts = []
            for item in v:
                item_parts = []
                for sk, sv in item.items():
                    item_parts.append(f"{sk}={dx_value(sv, indent+1)}")
                parts.append("(" + " ".join(item_parts) + ")")
            return "[" + ", ".join(parts) + "]"
        items = [dx_value(x, indent+1) for x in v]
        return " ".join(items)
    if isinstance(v, dict):
        fields = []
        for sk, sv in v.items():
            fields.append(f"  {pad}{sk} = {dx_value(sv, indent+1)}")
        return "(\n" + "\n".join(fields) + "\n" + pad + ")"
    return str(v)

def dx_llm_encode(d):
    """Convert JSON dict to DX LLM format."""
    lines = []
    for k, v in d.items():
        if isinstance(v, list) and v and isinstance(v[0], dict):
            fields = sorted(v[0].keys())
            lines.append(f"{k}[{' '.join(fields)}](")
            for item in v:
                row = []
                for f in fields:
                    row.append(dx_value(item[f]))
                lines.append(f"  {' '.join(row)}")
            lines.append(")")
        elif isinstance(v, list):
            vals = [dx_value(x) for x in v]
            lines.append(f"{k} = {' '.join(vals)}")
        elif isinstance(v, dict):
            lines.append(f"{k}(")
            for sk, sv in v.items():
                lines.append(f"  {sk} = {dx_value(sv, 1)}")
            lines.append(")")
        else:
            lines.append(f"{k} = {dx_value(v)}")
    return "\n".join(lines) + "\n"

for name, data in datasets.items():
    path = os.path.join(DIST, f"{name}.dx.llm")
    with open(path, "w") as f:
        f.write(dx_llm_encode(data))

print("All files generated in:", DIST)
for f in sorted(os.listdir(DIST)):
    print(f"  {f}")
