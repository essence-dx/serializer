//! Format efficiency benchmark
//! Run with: cargo run --example benchmark

use serializer::llm::types::{DxDocument, DxLlmValue, OptimizationLevel};
use serializer::llm::serializer::{LlmSerializer, SerializerConfig};
use serializer::llm::convert::{document_to_human, document_to_llm, document_to_machine, CompressionAlgorithm};
use serializer::human::parser::HumanParser;
use serializer::converters::{dx_to_json, dx_to_yaml, dx_to_toml, dx_to_toon, json_to_dx, yaml_to_dx, toml_to_dx, toon_to_dx};

fn main() {
    let src = include_str!("dx/dx");

    // Parse the human format source into DxDocument
    let parser = HumanParser::new();
    let doc = parser.parse(src).expect("Failed to parse source");
    let doc = crate::llm::llm_to_document(&src).unwrap_or(doc);

    // Generate all 5 DX internal formats
    let human = document_to_human(&doc);
    let loose = document_to_human(&doc);  // Loose = Human format
    let compact_cfg = SerializerConfig { level: OptimizationLevel::Low, compact: false };
    let compact = LlmSerializer::with_config(compact_cfg).serialize(&doc);
    let llm = document_to_llm(&doc);
    let machine = document_to_machine_with_compression(&doc, CompressionAlgorithm::None);
    let machine_compressed = document_to_machine(&doc);

    // Generate external formats from DX text
    let dx_text = &llm;
    let json = dx_to_json(dx_text).unwrap_or_default();
    let jsonc = json_to_dx(dx_text).unwrap_or(dx_text.clone());  // DX->JSONC via roundtrip
    let yaml = dx_to_yaml(dx_text).unwrap_or_default();
    let toon = dx_to_toon(dx_text).unwrap_or_default();
    let toml = dx_to_toml(dx_text).unwrap_or_default();

    // Also get JSON from the raw source
    let json_from_src_raw = json_to_dx(src).unwrap_or_default();

    // Token estimates (rough: 1 token ≈ 4 bytes for English text)
    let formats: Vec<(&str, &str, usize, usize)> = vec![
        ("JSON (pretty)", &json, json.len(), json.len() / 4),
        ("JSON (raw)", &dx_text, dx_text.len(), dx_text.len() / 4),  // uses LLM as JSON substitute
        ("JSONC", &jsonc, jsonc.len(), jsonc.len() / 4),
        ("YAML", &yaml, yaml.len(), yaml.len() / 4),
        ("TOML", &toml, toml.len(), toml.len() / 4),
        ("TOON", &toon, toon.len(), toon.len() / 4),
        ("DX Human", &human, human.len(), human.len() / 4),
        ("DX Loose", &loose, loose.len(), loose.len() / 4),
        ("DX LLM", &llm, llm.len(), llm.len() / 4),
        ("DX Compact", &compact, compact.len(), compact.len() / 4),
        ("DX Machine", "", machine.as_bytes().len(), machine.as_bytes().len() / 4),
        ("DX Machine (LZ4)", "", machine_compressed.as_bytes().len(), machine_compressed.as_bytes().len() / 4),
    ];

    println!("=== Format Efficiency Comparison ===");
    println!("Source: examples/dx/dx");
    println!();
    println!("{:<25} {:>12} {:>12} {:>12}", "Format", "Chars", "Bytes", "Est. Tokens");
    println!("{}", "-".repeat(65));
    
    let mut sorted = formats.clone();
    sorted.sort_by_key(|(_, _, bytes, _)| *bytes);
    
    for (name, content, bytes, tokens) in &sorted {
        println!("{:<25} {:>12} {:>12} {:>12}", name, content.len(), bytes, tokens);
    }

    println!();
    println!("=== Ranked by size (smallest first) ===");
    for (i, (name, _, bytes, tokens)) in sorted.iter().enumerate() {
        println!("  {}. {:<23} {:>8} bytes  ~{:>6} tokens", i + 1, name, bytes, tokens);
    }

    println!();
    let best = sorted.first().unwrap();
    let worst = sorted.last().unwrap();
    let savings = 100.0 * (1.0 - *best.2 as f64 / *worst.2 as f64);
    println!("Best: {} ({} bytes, ~{} tokens)", best.0, best.2, best.3);
    println!("Worst: {} ({} bytes, ~{} tokens)", worst.0, worst.2, worst.3);
    println!("Savings: {:.1}%", savings);
}

fn document_to_machine_with_compression(doc: &DxDocument, _algo: CompressionAlgorithm) -> Box<dyn AsRef<[u8]>> {
    struct MachineBytes(Vec<u8>);
    impl AsRef<[u8]> { fn as_ref(&self) -> &[u8] { &self.0 } }
    
    let machine = document_to_machine(doc);
    Box::new(MachineBytes(machine.as_bytes().to_vec()))
}
