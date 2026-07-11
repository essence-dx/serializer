//! DX Serializer CLI
//!
//! Processes .sr/.dx/JSON files and generates .llm and .machine outputs.
//! Also converts JSON, YAML, TOON, and JSONC to DX LLM format.
//!
//! Usage:
//!   dx-serialize <file> [options]                 Process file (auto-detect format)
//!   dx-serialize human <file> [options]           Process as Human format (readable)
//!   dx-serialize llm <file> [options]             Generate LLM format output
//!   dx-serialize machine <file> [options]         Generate Machine format output
//!   dx-serialize convert json <file> [options]    Convert JSON to DX LLM
//!   dx-serialize convert yml <file> [options]     Convert YAML to DX LLM
//!   dx-serialize convert toon <file> [options]    Convert TOON to DX LLM
//!   dx-serialize convert jsonc <file> [options]   Convert JSONC to DX LLM

#[path = "../js_cache_artifacts.rs"]
mod js_cache_artifacts;

use serializer::llm::convert::{CompressionAlgorithm, document_to_human};
use serializer::llm::serializer::{LlmSerializer, SerializerConfig};
use serializer::llm::types::OptimizationLevel;
use serializer::human::parser::HumanParser;
use serializer::{SerializerOutput, SerializerOutputConfig};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();
    let app_name = "dx-serialize";

    if args.len() == 1 || (args.len() > 1 && args[1] == "--help") {
        print_help(&args[0]);
        return;
    }

    if args.len() > 1 && args[1] == "--version" {
        println!("dx-serialize 1.0.0");
        return;
    }

    // Route subcommands
    match args[1].as_str() {
        "watch" => {
            let dir = if args.len() > 2 { args[2].clone() } else { ".".to_string() };
            let interval: u64 = if args.len() > 3 { args[3].parse().unwrap_or(2) } else { 2 };
            cmd_watch(&dir, interval);
        }
        "human" | "llm" | "machine" => {
            let format = args[1].clone();
            let file_args = &args[2..];
            let file = file_args.iter().find(|a| !a.starts_with("--"));
            let file = match file {
                Some(f) => f.clone(),
                None => {
                    eprintln!("Error: No input file specified for '{format}' mode");
                    eprintln!("Usage: {app_name} {format} <file> [options]");
                    std::process::exit(1);
                }
            };
            run_serialize_with_format(&file, &format, &parse_extra_flags(file_args));
        }
        "convert" => {
            if args.len() < 3 {
                eprintln!("Error: Missing format for 'convert'");
                eprintln!("Usage: {app_name} convert <json|yml|toon|jsonc> <file> [options]");
                std::process::exit(1);
            }
            let format = &args[2];
            let convert_args = &args[3..];
            let file = convert_args.iter().find(|a| !a.starts_with("--"));
            let file = match file {
                Some(f) => f.clone(),
                None => {
                    eprintln!("Error: No input file specified for convert {format}");
                    eprintln!("Usage: {app_name} convert {format} <file> [options]");
                    std::process::exit(1);
                }
            };
            run_convert(&file, format, &parse_extra_flags(convert_args));
        }
        _ => {
            // Fallback to legacy mode: first arg is the file
            run_serialize_legacy(&args[1..]);
        }
    }
}

fn print_help(bin: &str) {
    let name = Path::new(bin).file_stem().unwrap_or(std::ffi::OsStr::new("dx-serializer")).to_string_lossy();
    eprintln!("DX Serializer — token-efficient LLM serialization");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  {name} <file> [options]               Process file (auto-detect format)");
    eprintln!("  {name} human <file> [options]         Process as Human format (readable)");
    eprintln!("  {name} llm <file> [options]           Generate LLM format output");
    eprintln!("  {name} machine <file> [options]       Generate Machine format output");
    eprintln!("  {name} watch <dir> [interval]         Watch dir, auto-process .sr/.dx files");
    eprintln!("  {name} convert json <file> [options]  Convert JSON to DX LLM format");
    eprintln!("  {name} convert yml <file> [options]   Convert YAML to DX LLM format");
    eprintln!("  {name} convert toon <file> [options]  Convert TOON to DX LLM format");
    eprintln!("  {name} convert jsonc <file> [options] Convert JSONC to DX LLM format");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --output-dir <dir>       Output directory (default: .dx/serializer)");
    eprintln!("  --js-cache               Default output directory to .dx/js");
    eprintln!("  --machine-only           Generate only .machine output");
    eprintln!("  --metadata               Generate .machine.meta.json validation sidecar");
    eprintln!("  --llm-only               Generate only .llm output");
    eprintln!("  --lz4 | --speed          Use LZ4 compression (fastest, default)");
    eprintln!("  --zstd | --size          Use Zstd compression (better ratio)");
    eprintln!("  --no-compression         Disable compression");
    eprintln!("  --beautify               Human-readable output");
    eprintln!("  --compact                Compact LLM output (single-line, minified)");
    eprintln!("  --format                 Formatted LLM output");
    eprintln!("  --stdout                 Print LLM output to stdout instead of writing files");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {name} config.dx");
    eprintln!("  {name} human project.dx --stdout");
    eprintln!("  {name} llm deps50.json");
    eprintln!("  {name} convert json package.json --stdout");
    eprintln!("  {name} convert yml config.yml --stdout");
}

fn parse_extra_flags(args: &[String]) -> ExtraFlags {
    let mut flags = ExtraFlags::defaults_with_generation();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--output-dir" => {
                if i + 1 < args.len() { flags.output_dir = Some(args[i + 1].clone()); i += 1; }
            }
            "--js-cache" | "--javascript-cache" => flags.js_cache = true,
            "--machine-only" => { flags.generate_llm = false; flags.generate_machine = true; }
            "--metadata" => flags.generate_metadata = true,
            "--llm-only" => { flags.generate_llm = true; flags.generate_machine = false; }
            "--lz4" | "--speed" => flags.compression = CompressionAlgorithm::Lz4,
            "--zstd" | "--size" => flags.compression = CompressionAlgorithm::Zstd,
            "--no-compression" => flags.compression = CompressionAlgorithm::None,
            "--beautify" => flags.beautify = true,
            "--compact" => flags.compact = true,
            "--format" => flags.format_llm = true,
            "--stdout" => flags.stdout = true,
            arg if !arg.starts_with("--") && flags.input_file.is_none() => {
                flags.input_file = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }
    flags
}

#[derive(Default)]
struct ExtraFlags {
    input_file: Option<String>,
    output_dir: Option<String>,
    js_cache: bool,
    generate_llm: bool,
    generate_machine: bool,
    generate_metadata: bool,
    compression: CompressionAlgorithm,
    beautify: bool,
    compact: bool,
    format_llm: bool,
    stdout: bool,
}

impl ExtraFlags {
    fn defaults_with_generation() -> Self {
        Self {
            generate_llm: true,
            generate_machine: true,
            ..Self::default()
        }
    }
}

impl ExtraFlags {
    fn output_dir_or_default(&self) -> String {
        self.output_dir.clone().unwrap_or_else(|| {
            if self.js_cache { ".dx/js".to_string() } else { ".dx/serializer".to_string() }
        })
    }
}

fn run_serialize_with_format(file: &str, format: &str, flags: &ExtraFlags) {
    let source_path = Path::new(file);
    let output_dir = flags.output_dir.clone().unwrap_or_else(|| {
        let parent = source_path.parent().unwrap_or(Path::new("."));
        if flags.js_cache {
            parent.join(".dx/js").to_string_lossy().to_string()
        } else {
            parent.join(".dx/serializer").to_string_lossy().to_string()
        }
    });

    let compact = if format == "machine" { true } else { flags.compact };
    let level = match format {
        "human" => if compact { OptimizationLevel::Low } else { OptimizationLevel::Medium },
        "llm" => if compact { OptimizationLevel::Low } else { OptimizationLevel::Medium },
        "machine" => OptimizationLevel::Low,
        _ => OptimizationLevel::Medium,
    };

    let mut extra_generate_llm = flags.generate_llm;
    let mut extra_generate_machine = flags.generate_machine;

    if format == "machine" {
        extra_generate_llm = false;
        extra_generate_machine = true;
    }

    let serializer_config = SerializerConfig { compact, level };
    let config = SerializerOutputConfig::new()
        .with_output_dir(&output_dir)
        .with_llm(extra_generate_llm)
        .with_machine(extra_generate_machine)
        .with_metadata(flags.generate_metadata)
        .with_compression(flags.compression)
        .with_serializer_config(serializer_config.clone())
        .with_beautify(flags.beautify)
        .with_format_llm(flags.format_llm);
    let serializer = SerializerOutput::with_config(config);
    let source = Path::new(file);

    if flags.stdout {
        match fs::read_to_string(source) {
            Ok(content) => {
                let doc = match serializer::llm::parser::LlmParser::parse(&content) {
                    Ok(d) => d,
                    Err(e) => {
                        let text = match serializer::converters::convert_to_dx(&content, "json")
                            .or_else(|_| serializer::converters::convert_to_dx(&content, "yaml"))
                            .or_else(|_| serializer::converters::convert_to_dx(&content, "toml"))
                            .or_else(|_| serializer::converters::convert_to_dx(&content, "toon"))
                        {
                            Ok(dx) => dx,
                            Err(_) => {
                                eprintln!("Error: Could not parse or convert '{}': {e}", source.display());
                                std::process::exit(1);
                            }
                        };
                        println!("{text}");
                        return;
                    }
                };
                let ser = serializer::llm::serializer::LlmSerializer::with_config(serializer_config);
                println!("{}", ser.serialize(&doc));
            }
            Err(e) => {
                eprintln!("Error reading '{}': {e}", source.display());
                std::process::exit(1);
            }
        }
        return;
    }

    // Parse source for additional file generation — HumanParser first for .sr/dx files
    let source_content = fs::read_to_string(source).ok();
    let doc = source_content.as_ref().and_then(|c| {
        let parser = HumanParser::new();
        parser.parse(c).ok().or_else(|| {
            serializer::llm::parser::LlmParser::parse(c).ok()
        })
    });

    let loose_path = source.parent().unwrap_or(Path::new(".")).join("dx.loose");
    let compact_llm_path = source.parent().unwrap_or(Path::new(".")).join("dx.compact");

    match serializer.process_file(source) {
        Ok(result) => {
            let compression_name = match flags.compression {
                CompressionAlgorithm::Lz4 => "LZ4",
                CompressionAlgorithm::Zstd => "Zstd",
                CompressionAlgorithm::None => "None",
            };
            println!("Generated outputs (compression: {compression_name}, format: {format}):");
            if result.llm_generated {
                println!("  LLM:       {} ({} bytes)", result.paths.llm.display(), result.llm_size);
            }
            if result.machine_generated {
                println!("  Machine:   {} ({} bytes)", result.paths.machine.display(), result.machine_size);
            }

            // Generate human loose (TOML-like) format
            if let Some(ref d) = doc {
                let loose = serializer::llm::document_to_human(d);
                if fs::write(&loose_path, &loose).is_ok() {
                    println!("  Loose:     {} ({} bytes)", loose_path.display(), loose.len());
                }
            }

            // Generate compact LLM if not already
            if let Some(ref d) = doc {
                let compact_config = SerializerConfig { compact: true, level: OptimizationLevel::Low };
                let compact_ser = LlmSerializer::with_config(compact_config);
                let compact = compact_ser.serialize(d);
                if fs::write(&compact_llm_path, &compact).is_ok() {
                    println!("  Compact:   {} ({} bytes)", compact_llm_path.display(), compact.len());
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

fn run_serialize_legacy(args: &[String]) {
    // Legacy mode: args[0] is the file or --flag
    let file = match args.iter().find(|a| !a.starts_with("--")) {
        Some(f) => f.clone(),
        None => {
            eprintln!("Error: No input file specified");
            std::process::exit(1);
        }
    };
    let flags = parse_extra_flags(args);
    run_serialize_with_format(&file, "human", &flags);
}

fn run_convert(file: &str, format: &str, flags: &ExtraFlags) {
    let source = Path::new(file);
    let content = match fs::read_to_string(source) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading '{}': {e}", source.display());
            std::process::exit(1);
        }
    };

    let format_normalized = match format {
        "jsonc" => "json", // JSONC is just compact JSON
        f => f,
    };

    let dx_output = match serializer::converters::convert_to_dx(&content, format_normalized) {
        Ok(dx) => dx,
        Err(e) => {
            eprintln!("Error converting {format} to DX: {e}");
            std::process::exit(1);
        }
    };

    if flags.stdout {
        println!("{dx_output}");
        return;
    }

    // Write .llm file alongside the source
    let output_path = source.with_extension("llm");
    match fs::write(&output_path, &dx_output) {
        Ok(()) => {
            println!("Converted {} to DX LLM format: {}", source.display(), output_path.display());
            println!("  {} bytes", dx_output.len());
        }
        Err(e) => {
            eprintln!("Error writing '{}': {e}", output_path.display());
            std::process::exit(1);
        }
    }
}

fn read_inputs_file(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect())
}

#[cfg(feature = "parallel")]
fn process_input_paths(
    serializer: &SerializerOutput,
    inputs: &[PathBuf],
) -> Result<Vec<serializer::SerializerResult>, (PathBuf, serializer::SerializerOutputError)> {
    use rayon::prelude::*;

    inputs
        .par_iter()
        .map(|source| {
            serializer
                .process_file(source)
                .map_err(|error| (source.clone(), error))
        })
        .collect()
}

#[cfg(not(feature = "parallel"))]
fn process_input_paths(
    serializer: &SerializerOutput,
    inputs: &[PathBuf],
) -> Result<Vec<serializer::SerializerResult>, (PathBuf, serializer::SerializerOutputError)> {
    inputs
        .iter()
        .map(|source| {
            serializer
                .process_file(source)
                .map_err(|error| (source.clone(), error))
        })
        .collect()
}

fn cmd_watch(dir: &str, interval: u64) {
    use std::collections::HashMap;
    use std::time::SystemTime;

    let watch_path = Path::new(dir);
    if !watch_path.is_dir() {
        eprintln!("Error: '{}' is not a directory", watch_path.display());
        std::process::exit(1);
    }

    println!("Watching {} for .sr / .dx files (refresh: {}s)  Press Ctrl+C to stop", watch_path.display(), interval);

    let mut last_mtimes: HashMap<std::path::PathBuf, SystemTime> = HashMap::new();
    let mut processed: HashMap<std::path::PathBuf, bool> = HashMap::new();

    loop {
        let mut entries: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(read) = watch_path.read_dir() {
            for entry in read.flatten() {
                let path = entry.path();
                let ext = path.extension().map(|e| e.to_string_lossy().to_string());
                let is_sr = ext.as_deref() == Some("sr");
                let is_dx = path.file_name().map(|n| n == "dx").unwrap_or(false);
                if is_sr || is_dx {
                    entries.push(path);
                }
            }
        }

        for path in &entries {
            let changed = match (last_mtimes.get(path), path.metadata().ok().and_then(|m| m.modified().ok())) {
                (Some(old), Some(new)) => *old != new,
                (None, Some(_)) => true,
                _ => false,
            };

            if changed || !*processed.get(path).unwrap_or(&false) {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let parser = HumanParser::new();
                    match parser.parse(&content) {
                        Ok(doc) => {
                            let parent = path.parent().unwrap_or(Path::new("."));
                            let output_dir = parent.join(".dx/serializer");

                            // Generate .llm
                            let llm = serializer::llm::document_to_llm(&doc);
                            if let Ok(()) = std::fs::create_dir_all(&output_dir) {
                                let _ = std::fs::write(output_dir.join("dx.llm"), &llm);
                            }

                            // Generate .machine
                            let machine = serializer::llm::document_to_machine(&doc);
                            if let Ok(()) = std::fs::create_dir_all(&output_dir) {
                                let _ = std::fs::write(output_dir.join("dx.machine"), &machine.data);
                            }

                            // Generate .loose
                            let loose = document_to_human(&doc);
                            let _ = std::fs::write(parent.join("dx.loose"), &loose);

                            // Generate .compact
                            let compact_config = SerializerConfig { compact: true, level: OptimizationLevel::Low };
                            let compact_ser = LlmSerializer::with_config(compact_config);
                            let compact = compact_ser.serialize(&doc);
                            let _ = std::fs::write(parent.join("dx.compact"), &compact);

                            print!("  \x1b[K• {} ", path.file_name().unwrap().to_string_lossy());
                            println!("→ .llm .machine .loose .compact");
                        }
                        Err(e) => {
                            eprintln!("  ✗ {} parse error: {}", path.display(), e);
                        }
                    }
                }
                if let Ok(mt) = path.metadata().and_then(|m| m.modified()) {
                    last_mtimes.insert(path.clone(), mt);
                }
                processed.insert(path.clone(), true);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(interval));
    }
}
