---
layout: home
titleTemplate: DX Serializer
hero:
  name: DX Serializer
  text: Token-Efficient Serialization for LLMs
  tagline: A compact, human-readable encoding of the JSON data model that saves 50-86% tokens vs JSON.
  image:
    dark: /logo-index-dark.svg
    light: /logo-index-light.svg
    alt: DX Serializer
  actions:
    - theme: brand
      text: What is DX Serializer?
      link: /guide/getting-started
    - theme: alt
      text: Benchmarks
      link: /guide/benchmarks
    - theme: alt
      text: Playground
      link: /playground
    - theme: alt
      text: CLI
      link: /cli/

features:
  - title: Token-Efficient
    icon: 📊
    details: DX Serializer saves 50-86% tokens vs JSON pretty, 20-75% vs JSON compact, 33% vs DX Serializer on average. Beats every format on every benchmark.
    link: /guide/benchmarks
  - title: JSON Data Model
    icon: 🔁
    details: Encodes the same objects, arrays, and primitives as JSON with deterministic, lossless round-trips. 100% fidelity guaranteed.
    link: /guide/format-overview
  - title: Table Format
    icon: 🧺
    details: Uniform arrays of objects collapse into tables with shared column headers, eliminating repeated key names and saving massive tokens.
    link: /guide/format-overview#tables
  - title: Minimal Syntax
    icon: 📐
    details: Uses key=value, inline () blocks, and space-separated arrays. No quotes, no braces, no commas — just the data.
    link: /guide/format-overview
  - title: LLM-Optimized
    icon: 🛤️
    details: BPE tokenizers compress DX Serializer efficiently at 4+ bytes/token. Abbreviation-free — full key names are preserved for LLM comprehension.
    link: /guide/format-overview
  - title: Multi-Language
    icon: 🌐
    details: Bun TypeScript encoder/decoder library with Rust implementation for maximum performance.
    link: /ecosystem/implementations
---
