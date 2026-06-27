import tiktoken

enc = tiktoken.get_encoding("o200k_base")

json_s = '{"users":[{"id":1,"name":"Alice","role":"Engineer","active":true},{"id":2,"name":"Bob","role":"Designer","active":false},{"id":3,"name":"Carol","role":"Manager","active":true}]}'

dx_cli = 'users=[[id=1,name=Alice,role=Engineer,active=true] [id=2,name=Bob,role=Designer,active=false] [id=3,name=Carol,role=Manager,active=true]]'

dx_wrapped_truefalse = """users[id name role active](
1 Alice Engineer true
2 Bob Designer false
3 Carol Manager true
)"""

toon_real = """users[3]{id,name,role,active}:
  1,Alice,Engineer,true
  2,Bob,Designer,false
  3,Carol,Manager,true"""

for name, s in [("JSON compact", json_s), ("DX CLI (inline)", dx_cli), ("DX wrapped DF (true/false)", dx_wrapped_truefalse), ("TOON (real @toon-format)", toon_real)]:
    toks = enc.encode(s)
    print(f"  {name:<35} -> {len(toks):>2} tokens  ({len(s)} bytes)")
    print(f"    IDs: {toks}")
    print(f"    Texts: {[enc.decode([t]) for t in toks]}")
    print()
