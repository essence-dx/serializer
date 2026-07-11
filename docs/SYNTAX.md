# DX Serializer Syntax Reference

**Version:** 1.0  
**3 Formats:** Human, LLM, Machine

---

## 3 Formats

### Human — Visually beautiful (default for `dx` extensionless files & `.sr`)

```
project(
  name    = dx-tree
  version = 1.0.0
)

tree(
  exclude   = node_modules target .git dist build
  prune     = true
  icons     = false
  level     = 3
  dirsfirst = true
  noreport  = true
)
```

- Spaces around `=`
- Multi-line parenthesized objects
- Aligned indentation
- **Used for:** hand-edited `dx` config files, `.sr` files meant for human review

### LLM — Token efficient

```
project:
  name: dx-tree
  version: 1.0.0

tree:
  exclude: node_modules target .git dist build
  prune: true
```

- YAML-style `:` for small objects (<8 children)
- Parens `()` for large objects (8+ children)
- Auto-selects between token efficiency and readability
- **Used for:** generated `.llm` files in `.dx/serializer/`

### Machine — Most performance

```
(binary RKYV + optional LZ4/Zstd compression)
```

- Zero-copy runtime deserialization
- **Used for:** generated `.machine` files in `.dx/serializer/`

---

## Separator Styles

Both space and comma separators are supported and auto-detected per row.

### Space-separated (lower token count)

```
users[id name email](
  1 "Alice Johnson" alice@example.com
  2 "Bob Smith" bob@example.com
)
```

- Lower token count (commas add BPE tokens)
- Ideal when no value contains spaces

### Comma-separated (recommended for complex data)

```
recipes[name,group,doc,script](
  build,all,"Build all workspace crates","cargo build --workspace"
  test,all,"Run all workspace tests","cargo test --workspace"
)
```

- Clear visual field separation
- Values with spaces don't need quoting
- **Recommended** when doc strings or multi-word values are present
- Parser auto-detects comma vs space per row

---

## Data Types

### Scalar

```
key = value
key=value  (LLM mode)
```

### String

```
name    = "Multi word value"
name    = simple_value
```

Quotes required when value contains spaces, commas, or special chars.

### Number

```
count = 42
pi    = 3.14
```

### Boolean

```
active  = true
enabled = false
```

### Null

```
result = null
```

### Array

```
tags = [rust performance serialization]
```

Space-separated inside brackets. Comma-separated also supported:

```
tags = [rust, performance, serialization]
```

### Object

```
config(
  host    = localhost
  port    = 8080
  debug   = true
)
```

### Nested Object

```
project(
  name     = dx-tree
  version  = 1.0.0
  authors(
    name  = "Alice Smith"
    email = alice@example.com
  )
)
```

### Wrapped Dataframe (Table)

```
users[id,name,email](
  1,"Alice Johnson",alice@example.com
  2,"Bob Smith",bob@example.com
)
```

Schema defined in brackets, rows inside parentheses. Parser auto-detects space or comma separator.

---

## EBNF Grammar

```
document       = statement* ;
statement      = root_pair | section ;
root_pair      = key "=" value | key ":" value ;
section        = identifier "(" pairs ")" ;
pairs          = pair (" " pair)* ;
pair           = key "=" value ;
table          = identifier "[" headers "]" "(" rows ")" ;
headers        = identifier ("," identifier)* | identifier (" " identifier)* ;
rows           = row* ;
row            = value ("," value)* | value (" " value)* ;
value          = string | identifier | number | "true" | "false" | "null" ;
string         = '"' [^"]* '"' ;
key            = identifier ;
identifier     = [a-zA-Z_][a-zA-Z0-9_-]* ;
number         = [0-9]+ ("." [0-9]+)? ;
```
