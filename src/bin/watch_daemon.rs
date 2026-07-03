//! DX SR Watch Daemon
//!
//! Long-running daemon that watches `.dx/serializer/` for `.sr` file changes
//! and auto-compiles them to `.machine` (and `.llm`) outputs.
//!
//! Usage:
//!   dx-sr-watch                       # Watch .dx/serializer/ in current dir
//!   dx-sr-watch --dir crates/check    # Watch a specific project
//!   dx-sr-watch --machine-only        # Generate only .machine (skip .llm)

use serializer::watch::{DxWatcher, FileChange};
use serializer::{SerializerOutput, SerializerOutputConfig};
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut watch_dir: Option<String> = None;
    let mut machine_only = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                if i + 1 < args.len() {
                    watch_dir = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--machine-only" => {
                machine_only = true;
            }
            "--help" | "-h" => {
                eprintln!("DX SR Watch Daemon");
                eprintln!();
                eprintln!("Usage: dx-sr-watch [options]");
                eprintln!();
                eprintln!("Options:");
                eprintln!("  --dir <path>         Watch directory (default: .dx/serializer)");
                eprintln!("  --machine-only       Generate only .machine output (skip .llm)");
                eprintln!("  --help, -h           Show this help");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    let serializer_dir = watch_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".dx/serializer"));

    let abs_dir = serializer_dir.canonicalize().unwrap_or_else(|_| {
        // Directory may not exist yet — create and try again
        let _ = std::fs::create_dir_all(&serializer_dir);
        serializer_dir.canonicalize().unwrap_or(serializer_dir.clone())
    });

    eprintln!("DX SR Watch Daemon v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("  Watch dir: {}", abs_dir.display());
    eprintln!("  Mode: {}", if machine_only { "machine-only" } else { "machine + llm" });
    eprintln!("  Watching for .sr changes (Ctrl+C to stop)...");
    eprintln!();

    // Create the serializer config for compiling .sr -> .machine / .llm
    let config = SerializerOutputConfig::new()
        .with_output_dir(&serializer_dir)
        .with_llm(!machine_only)
        .with_machine(true);
    let serializer = SerializerOutput::with_config(config);

    // Initial compilation of existing .sr files
    match serializer.process_directory(&serializer_dir) {
        Ok(results) => {
            for r in &results {
                eprintln!("  [init] {} -> .machine", r.paths.source.display());
            }
            if results.is_empty() {
                eprintln!("  [init] No .sr files found in {}", abs_dir.display());
            } else {
                eprintln!("  [init] Compiled {} file(s)", results.len());
            }
        }
        Err(e) => {
            eprintln!("  [init] Warning: {e}");
        }
    }

    // Start the file watcher
    let mut watcher = match DxWatcher::new(500) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error creating file watcher: {e}");
            eprintln!("Build with --features watch to enable file watching");
            std::process::exit(1);
        }
    };

    if let Err(e) = watcher.watch_directory(&serializer_dir) {
        eprintln!("Error watching directory {}: {e}", serializer_dir.display());
        std::process::exit(1);
    }

    eprintln!("  Ready. Waiting for changes...");

    // Event loop — blocks on changes() iterator
    for change in watcher.changes() {
        match change {
            FileChange::RuleFileChanged(path) | FileChange::RuleFileCreated(path) => {
                eprintln!("  [change] {} -> compiling...", path.display());
                match serializer.process_file(&path) {
                    Ok(result) => {
                        eprint!("    OK");
                        if result.machine_generated {
                            eprint!(" (.machine: {} bytes)", result.machine_size);
                        }
                        if result.llm_generated {
                            eprint!(" (.llm: {} bytes)", result.llm_size);
                        }
                        eprintln!();
                    }
                    Err(e) => {
                        eprintln!("    ERROR: {e}");
                    }
                }
            }
            FileChange::RuleFileDeleted(path) => {
                eprintln!("  [delete] {}", path.display());
                let paths = serializer.get_paths(&path);
                let _ = std::fs::remove_file(&paths.llm);
                let _ = std::fs::remove_file(&paths.machine);
                let _ = std::fs::remove_file(&paths.metadata);
                eprintln!("    Removed output files");
            }
            FileChange::ConfigChanged(path) => {
                eprintln!("  [config] {} changed — recompiling all .sr files", path.display());
                let _ = serializer.process_directory(&serializer_dir);
            }
        }
    }
}
