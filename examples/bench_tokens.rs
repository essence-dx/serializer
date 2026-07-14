use serializer::llm::tokens::{TokenCounter, ModelType};
use serializer::llm::convert::{document_to_human, document_to_llm, document_to_machine, llm_to_document};
use serializer::llm::serializer::{LlmSerializer, SerializerConfig};
use serializer::llm::types::OptimizationLevel;
use serializer::converters::*;

fn main() {
    let src = include_str!("clean.dx");
    let doc = llm_to_document(src).expect("Failed to parse source");

    let cfg = SerializerConfig { level: OptimizationLevel::Low, compact: false };
    let llm = document_to_llm(&doc);
    let compact = LlmSerializer::with_config(cfg).serialize(&doc);
    let human = document_to_human(&doc);
    let machine = document_to_machine(&doc);

    let json = dx_to_json(&llm).unwrap();
    let json_min = dx_to_json_min(&llm).unwrap();
    let yaml = dx_to_yaml(&llm).unwrap();
    let toml = dx_to_toml(&llm).unwrap();
    let toon = dx_to_toon(src).unwrap();

    let counter = TokenCounter::new();

    let header = "=".repeat(75);
    let sep = "-".repeat(73);
    println!("{}", header);
    println!("  FORMAT TOKEN EFFICIENCY — tiktoken o200k_base (GPT-4o)");
    println!("  Source: examples/clean.dx (10 flat k=v pairs)");
    println!("{}", header);
    println!("{:<22} {:>8} {:>10} {:>10} {:>10}", "Format", "Bytes", "GPT-4o", "Claude4", "vs Best");
    println!("{}", sep);

    let text_formats: Vec<(&str, &str)> = vec![
        ("YAML", &yaml),
        ("TOML", &toml),
        ("JSON (pretty)", &json),
        ("JSON (minified)", &json_min),
        ("TOON", &toon),
        ("DX Human", &human),
        ("DX LLM", &llm),
        ("DX Compact", &compact),
    ];

    let mut results: Vec<(&str, usize, usize, usize)> = text_formats.iter().map(|(name, content)| {
        let gpt = counter.count(content, ModelType::Gpt4o).count;
        let claude = counter.count(content, ModelType::ClaudeSonnet4).count;
        (*name, content.len(), gpt, claude)
    }).collect();

    results.sort_by_key(|(_, _, gpt, _)| *gpt);

    let best_gpt = results.first().unwrap().2;
    for (name, bytes, gpt, claude) in &results {
        let pct = 100.0 * (*gpt as f64 / best_gpt as f64 - 1.0);
        println!("{:<22} {:>8} {:>10} {:>10} {:>+9.1}%", name, bytes, gpt, claude, pct);
    }

    println!("{}", sep);
    println!("{:<22} {:>8} {:>10}", "DX Machine (binary)", machine.as_bytes().len(), "N/A");

    let worst = results.last().unwrap();
    let best = results.first().unwrap();
    let savings = 100.0 * (1.0 - best.2 as f64 / worst.2 as f64);
    println!("{}", sep);
    println!();
    println!("  WINNER:  {} — {} GPT-4o tokens, {} bytes", best.0, best.2, best.1);
    println!("  LOSER:   {} — {} GPT-4o tokens, {} bytes", worst.0, worst.2, worst.1);
    println!("  SAVINGS: {:.1}% fewer tokens", savings);
}
