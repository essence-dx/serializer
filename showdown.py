import tiktoken

enc = tiktoken.get_encoding("o200k_base")

# Same data in both formats - a user list with 3 entries
data = {
    "users": [
        {"id": 1, "name": "Alice", "role": "Engineer", "active": True},
        {"id": 2, "name": "Bob", "role": "Designer", "active": False},
        {"id": 3, "name": "Carol", "role": "Manager", "active": True},
    ]
}

# DX LLM wrapped DF format
dx = """users[id name role active](
1 Alice Engineer +
2 Bob Designer -
3 Carol Manager +
)"""

# Real TOON format (@toon-format/toon)
toon = """users[3]{id,name,role,active}:
  1,Alice,Engineer,true
  2,Bob,Designer,false
  3,Carol,Manager,true"""

# Count per-format
dx_tokens = enc.encode(dx)
toon_tokens = enc.encode(toon)

print("=" * 80)
print("  SAME DATA — SIDE BY SIDE TOKEN COMPARISON")
print("=" * 80)

print("\n  DX LLM FORMAT (wrapped dataframe):")
print("-" * 50)
print(dx)
print(f"\n  Tokens: {len(dx_tokens)}")
print(f"  Token IDs: {dx_tokens}")
print(f"  Token texts: {[enc.decode([t]) for t in dx_tokens]}")

print("\n" + "=" * 80)
print("\n  TOON FORMAT (real @toon-format/toon):")
print("-" * 50)
print(toon)
print(f"\n  Tokens: {len(toon_tokens)}")
print(f"  Token IDs: {toon_tokens}")
print(f"  Token texts: {[enc.decode([t]) for t in toon_tokens]}")

print("\n" + "=" * 80)
print("  TOKEN-BY-TOKEN BREAKDOWN")
print("=" * 80)

print("\n  --- DX ---")
for i, t in enumerate(dx_tokens):
    text = enc.decode([t])
    print(f"    [{i:2d}] id={t:5d}  text={repr(text)}")

print("\n  --- TOON ---")
for i, t in enumerate(toon_tokens):
    text = enc.decode([t])
    print(f"    [{i:2d}] id={t:5d}  text={repr(text)}")

print("\n" + "=" * 80)
print("  VERDICT")
print("=" * 80)

savings = (1 - len(dx_tokens) / len(toon_tokens)) * 100
print(f"\n  DX:   {len(dx_tokens)} tokens")
print(f"  TOON: {len(toon_tokens)} tokens")
print(f"  DX is {abs(savings):.1f}% {'more' if savings > 0 else 'less'} efficient than TOON")
print(f"  (vs JSON: {len(enc.encode('{\"users\":[{\"id\":1,\"name\":\"Alice\",\"role\":\"Engineer\",\"active\":true},{\"id\":2,\"name\":\"Bob\",\"role\":\"Designer\",\"active\":false},{\"id\":3,\"name\":\"Carol\",\"role\":\"Manager\",\"active\":true}]}'))} tokens)")
