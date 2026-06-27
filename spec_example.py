import tiktoken

# Exact example from LLM_FORMAT_SPEC.md
json_str = r'{"name":"MyApp","version":"1.0.0","tags":["rust","performance"],"users":[{"id":1,"name":"Alice","email":"alice@ex.com"},{"id":2,"name":"Bob","email":"bob@ex.com"}]}'

dx_llm_spec = """name=MyApp
version=1.0.0
tags=[rust performance]
users[id name email](
1 Alice alice@ex.com
2 Bob bob@ex.com
)"""

# TOON from spec (YAML-like)
toon_spec = """name: MyApp
version: 1.0.0
tags:
  - rust
  - performance
users:
  - id: 1
    name: Alice
    email: alice@ex.com
  - id: 2
    name: Bob
    email: bob@ex.com"""

# TOON from toon.rs converter
toon_rs = 'name "MyApp"\nversion "1.0.0"\ntags[2]: "rust", "performance"\nusers[2]{id,name,email}:\n  1,"Alice","alice@ex.com"\n  2,"Bob","bob@ex.com"\n'

# Also CLI-style inline objects
dx_cli_inline = """name=MyApp
version=1.0.0
tags=[rust performance]
users=[[id=1,name=Alice,email=alice@ex.com] [id=2,name=Bob,email=bob@ex.com]]"""

encoders = [("GPT-4o (o200k_base)", "o200k_base"), ("GPT-4 (cl100k_base)", "cl100k_base"), ("Claude (cl100k_base sim)", "cl100k_base")]

for model, enc_name in encoders:
    enc = tiktoken.get_encoding(enc_name)
    print(f"\n{'='*70}")
    print(f"  {model}")
    print(f"{'='*70}")
    print(f"  {'Format':<25} {'Tokens':>8}")
    print(f"  {'-'*33}")
    print(f"  {'JSON (compact)':<25} {len(enc.encode(json_str)):>8}")
    print(f"  {'DX LLM (spec w/ DF)':<25} {len(enc.encode(dx_llm_spec)):>8}")
    print(f"  {'DX LLM (CLI inline)':<25} {len(enc.encode(dx_cli_inline)):>8}")
    print(f"  {'TOON (spec YAML-like)':<25} {len(enc.encode(toon_spec)):>8}")
    print(f"  {'TOON (rs converter)':<25} {len(enc.encode(toon_rs)):>8}")
    
    js_t = len(enc.encode(json_str))
    dx_spec_t = len(enc.encode(dx_llm_spec))
    dx_cli_t = len(enc.encode(dx_cli_inline))
    toon_spec_t = len(enc.encode(toon_spec))
    toon_rs_t = len(enc.encode(toon_rs))
    
    print(f"\n  {'Comparison':<25} {'Savings':>10}")
    print(f"  {'-'*35}")
    print(f"  {'DX spec DF vs JSON':<25} {(1-dx_spec_t/js_t)*100:>+9.1f}%")
    print(f"  {'DX CLI vs JSON':<25} {(1-dx_cli_t/js_t)*100:>+9.1f}%")
    print(f"  {'TOON spec vs JSON':<25} {(1-toon_spec_t/js_t)*100:>+9.1f}%")
    print(f"  {'TOON rs vs JSON':<25} {(1-toon_rs_t/js_t)*100:>+9.1f}%")
    print(f"  {'DX spec vs TOON rs':<25} {(1-dx_spec_t/toon_rs_t)*100:>+9.1f}%")

print(f"\n{'='*70}")
print("  RAW OUTPUTS (for verification)")
print(f"{'='*70}")
print(f"\nJS: {len(json_str)}b → {json_str}")
print(f"\nDX spec DF ({len(dx_llm_spec)}b):\n{dx_llm_spec}")
print(f"\nDX CLI inline ({len(dx_cli_inline)}b):\n{dx_cli_inline}")
print(f"\nTOON rs ({len(toon_rs)}b):\n{toon_rs}")
