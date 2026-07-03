//! DX Serializer CLI
//!
//! Processes .sr/.dx/JSON files and generates .llm and .machine outputs.

#[path = "../js_cache_artifacts.rs"]
mod js_cache_artifacts;

use serializer::llm::convert::CompressionAlgorithm;
use serializer::llm::serializer::SerializerConfig;
use serializer::{SerializerOutput, SerializerOutputConfig};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: dx-serialize <file.sr|file.dx|file.json> [options]");
        eprintln!("       dx-serialize --dir <directory> [options]");
        eprintln!("       dx-serialize --inputs-file <file> [options]");
        eprintln!("       dx-serialize --write-js-cache-artifacts --catalog-json <file> [options]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --output-dir <dir>       Output directory (default: .dx/serializer)");
        eprintln!("  --js-cache               Default output directory to .dx/js");
        eprintln!("  --javascript-cache       Alias for --js-cache");
        eprintln!("  --machine-only           Generate only .machine output");
        eprintln!("  --metadata               Generate .machine.meta.json validation sidecar");
        eprintln!(
            "  --inputs-file <file>     Process newline-delimited input paths in one process"
        );
        eprintln!("  --write-js-cache-artifacts");
        eprintln!(
            "                           Write Rust rkyv/bytemuck JS cache catalog and shards"
        );
        eprintln!("  --catalog-json <file>    Catalog JSON for --write-js-cache-artifacts");
        eprintln!("  --js-cache-shard-root <dir>");
        eprintln!(
            "                           Final repo-relative shard root used for packed shard identity"
        );
        eprintln!("  --llm-only               Generate only .llm output");
        eprintln!("  --lz4                    Use LZ4 compression (fastest, default)");
        eprintln!("  --zstd                   Use Zstd compression (better ratio)");
        eprintln!("  --speed                  Alias for --lz4");
        eprintln!("  --size                   Alias for --zstd");
        eprintln!("  --no-compression         Disable compression");
        eprintln!("  --compact                Compact mode: single-line sections (rows space-separated)");
        eprintln!("  --beautify               Human-readable output (aligned =, [section] headers)");
        eprintln!("  --format                 Formatted LLM output (spaces around `=`, indented sections)");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  dx-serialize crates/check/rules/javascript-lint.sr");
        eprintln!("  dx-serialize package.json --js-cache --machine-only --metadata");
        eprintln!(
            "  dx-serialize --inputs-file .dx/js/inputs.txt --js-cache --machine-only --metadata"
        );
        eprintln!(
            "  dx-serialize --write-js-cache-artifacts --catalog-json .dx/js/catalog.json --output-dir .dx/js"
        );
        eprintln!("  dx-serialize --dir crates/check/rules --zstd");
        eprintln!("  dx-serialize file.sr --speed --output-dir build/");
        std::process::exit(1);
    }

    let mut output_dir: Option<String> = None;
    let mut compression = CompressionAlgorithm::default();
    let mut input_path: Option<String> = None;
    let mut inputs_file: Option<String> = None;
    let mut catalog_json: Option<String> = None;
    let mut js_cache_shard_root: Option<String> = None;
    let mut write_js_cache_artifacts = false;
    let mut is_dir = false;
    let mut js_cache = false;
    let mut generate_llm = true;
    let mut generate_machine = true;
    let mut generate_metadata = false;
    let mut compact = false;
    let mut beautify = false;
    let mut format_llm = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                is_dir = true;
                if i + 1 < args.len() {
                    input_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--output-dir" => {
                if i + 1 < args.len() {
                    output_dir = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--inputs-file" => {
                if i + 1 < args.len() {
                    inputs_file = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--write-js-cache-artifacts" => {
                write_js_cache_artifacts = true;
            }
            "--catalog-json" => {
                if i + 1 < args.len() {
                    catalog_json = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--js-cache-shard-root" => {
                if i + 1 < args.len() {
                    js_cache_shard_root = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--js-cache" | "--javascript-cache" => {
                js_cache = true;
            }
            "--machine-only" => {
                generate_llm = false;
                generate_machine = true;
            }
            "--metadata" => {
                generate_metadata = true;
            }
            "--llm-only" => {
                generate_llm = true;
                generate_machine = false;
            }
            "--lz4" | "--speed" => {
                compression = CompressionAlgorithm::Lz4;
            }
            "--zstd" | "--size" => {
                compression = CompressionAlgorithm::Zstd;
            }
            "--no-compression" => {
                compression = CompressionAlgorithm::None;
            }
            "--compact" => {
                compact = true;
            }
            "--beautify" => {
                beautify = true;
            }
            "--format" => {
                format_llm = true;
            }
            arg if !arg.starts_with("--") && input_path.is_none() => {
                input_path = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    let input_path = match (input_path, &inputs_file, write_js_cache_artifacts) {
        (Some(p), _, _) => Some(p),
        (None, Some(_), _) => None,
        (None, None, true) => None,
        (None, None, false) => {
            eprintln!("Error: No input file or directory specified");
            std::process::exit(1);
        }
    };

    let output_dir = output_dir.unwrap_or_else(|| {
        if js_cache {
            ".dx/js".to_string()
        } else {
            ".dx/serializer".to_string()
        }
    });

    // Mutual exclusivity: beautify > format > compact
    if beautify && format_llm {
        format_llm = false;
    }
    if format_llm && compact {
        compact = false;
    }

    let serializer_config = if compact {
        SerializerConfig { compact: true }
    } else {
        SerializerConfig::default()
    };

    let config = SerializerOutputConfig::new()
        .with_output_dir(&output_dir)
        .with_llm(generate_llm)
        .with_machine(generate_machine)
        .with_metadata(generate_metadata)
        .with_compression(compression)
        .with_serializer_config(serializer_config)
        .with_beautify(beautify)
        .with_format_llm(format_llm);
    let serializer = SerializerOutput::with_config(config);

    let compression_name = match compression {
        CompressionAlgorithm::Lz4 => "LZ4",
        CompressionAlgorithm::Zstd => "Zstd",
        CompressionAlgorithm::None => "None",
    };

    // Write startup.sr to global cache dir
    let cache_dir = dirs::cache_dir()
        .map(|b| b.join("dx").join("serializer"))
        .unwrap_or_else(|| PathBuf::from("~/.cache/dx/serializer"));
    if let Ok(()) = fs::create_dir_all(&cache_dir) {
        let startup_path = cache_dir.join("startup.sr");
        let _ = fs::write(&startup_path, "tool=serializer\nstatus=ok\n");
    }

    if write_js_cache_artifacts {
        let catalog_json = catalog_json.unwrap_or_else(|| {
            Path::new(&output_dir)
                .join("catalog.json")
                .to_string_lossy()
                .into_owned()
        });
        match js_cache_artifacts::write_js_cache_artifacts(
            Path::new(&catalog_json),
            Path::new(&output_dir),
            js_cache_shard_root.as_deref().map(Path::new),
        ) {
            Ok(()) => {
                println!(
                    "Wrote DX JS cache artifacts for {catalog_json} into {output_dir}"
                );
            }
            Err(error) => {
                eprintln!("Error writing DX JS cache artifacts: {error}");
                std::process::exit(1);
            }
        }
    } else if let Some(inputs_file) = inputs_file {
        let inputs = match read_inputs_file(Path::new(&inputs_file)) {
            Ok(inputs) => inputs,
            Err(error) => {
                eprintln!("Error reading inputs file: {error}");
                std::process::exit(1);
            }
        };

        match process_input_paths(&serializer, &inputs) {
            Ok(results) => print_results(
                &format!("Processed {} input files", results.len()),
                compression_name,
                results,
            ),
            Err((source, error)) => {
                eprintln!("Error processing {}: {}", source.display(), error);
                std::process::exit(1);
            }
        }
    } else if is_dir {
        let input_path = input_path.unwrap();
        let dir = Path::new(&input_path);
        match serializer.process_directory(dir) {
            Ok(results) => {
                print_results(
                    &format!("Processed {} files", results.len()),
                    compression_name,
                    results,
                );
            }
            Err(e) => {
                eprintln!("Error processing directory: {e}");
                std::process::exit(1);
            }
        }
    } else {
        let input_path = input_path.unwrap();
        let source = Path::new(&input_path);
        match serializer.process_file(source) {
            Ok(result) => {
                println!(
                    "Generated outputs for {} (compression: {}):",
                    source.display(),
                    compression_name
                );
                if result.llm_generated {
                    println!(
                        "  LLM:     {} ({} bytes)",
                        result.paths.llm.display(),
                        result.llm_size
                    );
                }
                if result.machine_generated {
                    println!(
                        "  Machine: {} ({} bytes)",
                        result.paths.machine.display(),
                        result.machine_size
                    );
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
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

fn print_results(
    summary: &str,
    compression_name: &str,
    results: Vec<serializer::SerializerResult>,
) {
    println!("{summary} (compression: {compression_name}):");
    for result in results {
        println!("  {}", result.paths.source.display());
        if result.llm_generated {
            println!(
                "    LLM:     {} ({} bytes)",
                result.paths.llm.display(),
                result.llm_size
            );
        }
        if result.machine_generated {
            println!(
                "    Machine: {} ({} bytes)",
                result.paths.machine.display(),
                result.machine_size
            );
        }
    }
}
