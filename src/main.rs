//! Tsuchinoko CLI - Python to Rust Transpiler
//!
//! Author: Tane Channel Technology

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tsuchinoko::transpile_with_diagnostics;

/// Tsuchinoko - Python to Rust Transpiler
#[derive(Parser, Debug)]
#[command(name = "tnk")]
#[command(author = "Tane Channel Technology")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Transpile Python code to Rust", long_about = None)]
struct Cli {
    /// Input Python file
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output Rust file (default: <INPUT>.rs)
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Generate a complete Rust project folder
    #[arg(short, long, value_name = "PROJECT_NAME")]
    project: Option<String>,

    /// Show debug information
    #[arg(short, long)]
    debug: bool,

    /// Dump Intermediate Representation (IR) and exit
    #[arg(long)]
    dump_ir: bool,

    /// Check only (don't generate output)
    #[arg(short, long)]
    check: bool,

    /// Emit JSON diagnostics to stderr (on failure only)
    #[arg(long)]
    diag_json: bool,

    /// PyO3 version for generated project (default: "0" = latest major)
    #[arg(long, default_value = "0")]
    pyo3_version: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.debug {
        println!("[DEBUG] Input: {:?}", cli.input);
        println!("[DEBUG] Output: {:?}", cli.output);
        println!("[DEBUG] Project: {:?}", cli.project);
    }

    // Read input file
    let source = std::fs::read_to_string(&cli.input)?;

    if cli.debug {
        println!("[DEBUG] Source length: {} bytes", source.len());
    }

    // V1.7.0: IR Dump mode
    if cli.dump_ir {
        let ir = match tsuchinoko::analyze_to_ir_with_diagnostics(&source, Some(&cli.input)) {
            Ok(ir) => ir,
            Err(diags) => {
                print!("{}", diags.to_text());
                if cli.diag_json {
                    eprintln!("{}", diags.to_json());
                }
                std::process::exit(1);
            }
        };
        println!("=== Intermediate Representation (IR) ===");
        for (i, node) in ir.iter().enumerate() {
            println!("[{:03}] {:?}", i, node);
        }
        return Ok(());
    }

    // Transpile Python to Rust
    let rust_code = match transpile_with_diagnostics(&source, Some(&cli.input)) {
        Ok(code) => code,
        Err(diags) => {
            print!("{}", diags.to_text());
            if cli.diag_json {
                eprintln!("{}", diags.to_json());
            }
            std::process::exit(1);
        }
    };

    if cli.debug {
        println!("[DEBUG] Generated Rust code:");
        println!("{rust_code}");
    }

    // Check mode
    if cli.check {
        println!("✅ Transpilation successful!");
        return Ok(());
    }

    // V1.4.0: Check if external libraries are used (PythonBridge indicates external imports)
    let uses_external_libs = rust_code.contains("PythonBridge");

    // V1.6.0: Check if **kwargs is used (HashMap<String, TnkValue>)
    let uses_kwargs = rust_code.contains("HashMap<String, TnkValue>");

    // V1.4.0: Enforce --project when external libraries are used
    if uses_external_libs && cli.project.is_none() {
        eprintln!("Error: This code uses external Python libraries via PythonBridge.");
        eprintln!("       Please use --project option to generate a complete project:");
        eprintln!();
        eprintln!(
            "       tnk {} --project ./output_project",
            cli.input.display()
        );
        eprintln!();
        eprintln!("       The --project option generates a Cargo project with:");
        eprintln!("         - bridge/ mod.rs: Python worker communication module");
        eprintln!("         - Cargo.toml: Dependencies (serde, serde_json, uuid, thiserror)");
        eprintln!();
        eprintln!("       After generation, run:");
        eprintln!("         source venv/bin/activate");
        eprintln!("         cd ./output_project && cargo run --release");
        std::process::exit(1);
    }

    // V1.6.0: Enforce --project when **kwargs is used
    if uses_kwargs && cli.project.is_none() {
        eprintln!("Error: This code uses **kwargs (dynamic keyword arguments).");
        eprintln!("       Please use --project option to generate a complete project:");
        eprintln!();
        eprintln!(
            "       tnk {} --project ./output_project",
            cli.input.display()
        );
        eprintln!();
        eprintln!("       The --project option generates a Cargo project with:");
        eprintln!("         - bridge/ mod.rs: Python protocol module (TnkValue)");
        eprintln!("         - Cargo.toml: Dependencies (serde, serde_json)");
        eprintln!();
        eprintln!("       After generation, run:");
        eprintln!("         cd ./output_project && cargo run --release");
        std::process::exit(1);
    }

    // Project generation mode
    if let Some(project_name) = &cli.project {
        generate_project(project_name, &rust_code, &cli.pyo3_version)?;
        println!("✅ Generated project: {project_name}/");
        println!("   Run: cd {project_name} && cargo build --release");
        return Ok(());
    }

    // Single file transpilation
    let output_path = cli.output.unwrap_or_else(|| {
        // Default: output to current directory with same filename.rs
        let mut p = cli.input.clone();
        p.set_extension("rs");
        // If input has a path, use just the filename in current dir
        if let Some(filename) = p.file_name() {
            std::path::PathBuf::from(filename)
        } else {
            p
        }
    });

    std::fs::write(&output_path, &rust_code)?;
    println!("✅ Transpiled to: {output_path:?}");

    Ok(())
}

/// Generate a complete Rust project
fn generate_project(name: &str, rust_code: &str, pyo3_version: &str) -> Result<()> {
    use std::fs;

    // Extract just the project name from path (e.g., "tmp/myproj" -> "myproj")
    let project_name = std::path::Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(name);

    // V1.7.0 Fix: Clean up output directory to prevent stale file conflicts (e.g. bridge.rs vs bridge/mod.rs)
    if std::path::Path::new(name).exists() {
        let _ = fs::remove_dir_all(name);
    }

    // Create project directory
    fs::create_dir_all(format!("{name}/src"))?;

    // Check dependencies
    let uses_pyo3 = rust_code.contains("use pyo3");
    let uses_serde = rust_code.contains("serde");
    let uses_bridge = rust_code.contains("PythonBridge");

    let mut dependencies = String::new();

    // Minimal dependencies (no tsuchinoko - self-contained project)
    if uses_serde || uses_bridge {
        dependencies.push_str("serde = { version = \"1.0\", features = [\"derive\"] }\n");
        dependencies.push_str("serde_json = \"1.0\"\n");
    }
    
    if uses_bridge {
        dependencies.push_str("uuid = { version = \"1.19.0\", features = [\"v4\"] }\n");
        dependencies.push_str("thiserror = \"1.0\"\n"); // bridge_error uses thiserror
    }

    if uses_pyo3 {
        dependencies.push_str(&format!(
            "pyo3 = {{ version = \"{pyo3_version}\", features = [\"auto-initialize\"] }}\n"
        ));
    }

    let cargo_toml = format!(
        r#"[package]
name = "{project_name}"
version = "0.1.0"
edition = "2021"

[dependencies]
{dependencies}"#
    );
    fs::write(format!("{name}/Cargo.toml"), cargo_toml)?;

    // Create .gitignore
    let gitignore = "/target\n";
    fs::write(format!("{name}/.gitignore"), gitignore)?;

    // If using PythonBridge, generate embedded bridge module structure
    if uses_bridge {
        fs::create_dir_all(format!("{name}/src/bridge/python"))?;
        fs::create_dir_all(format!("{name}/src/bridge/strategies"))?;
        
        // Copy Bridge Files
        fs::write(format!("{name}/src/bridge/mod.rs"), include_str!("bridge/runtime_mod.rs"))?;
        fs::write(format!("{name}/src/bridge/protocol.rs"), include_str!("bridge/protocol.rs"))?;
        fs::write(format!("{name}/src/bridge/bridge_error.rs"), include_str!("bridge/bridge_error.rs"))?;
        fs::write(format!("{name}/src/bridge/tsuchinoko_error.rs"), include_str!("bridge/tsuchinoko_error.rs"))?;
        // fs::write(format!("{name}/src/bridge/module_table.rs"), include_str!("bridge/module_table.rs"))?; // Compiler-only
        // fs::write(format!("{name}/src/bridge/builtin_table.rs"), include_str!("bridge/builtin_table.rs"))?; // Compiler-only
        fs::write(format!("{name}/src/bridge/type_inference.rs"), include_str!("bridge/type_inference.rs"))?;
        
        // Copy Strategies
        fs::write(format!("{name}/src/bridge/strategies/mod.rs"), include_str!("bridge/strategies/mod.rs"))?;
        fs::write(format!("{name}/src/bridge/strategies/native.rs"), include_str!("bridge/strategies/native.rs"))?;
        fs::write(format!("{name}/src/bridge/strategies/pyo3.rs"), include_str!("bridge/strategies/pyo3.rs"))?;
        fs::write(format!("{name}/src/bridge/strategies/resident.rs"), include_str!("bridge/strategies/resident.rs"))?;

        // Copy Python Worker
        fs::write(format!("{name}/src/bridge/python/worker.py"), include_str!("bridge/python/worker.py"))?;
    }

    // Create main.rs with transpiled code
    // Replace tsuchinoko::bridge with local bridge module
    let fixed_code = rust_code.replace(
        "use tsuchinoko::bridge::PythonBridge;",
        "mod bridge;\nuse bridge::PythonBridge;",
    );
    // Replace specific imports if needed, but mod bridge; makes bridge module available.
    // The transpiled code usually uses tsuchinoko::bridge::protocol::TnkValue etc.
    // We need to support `tsuchinoko::bridge::` path replacement for full compatibility.
    // Replace full path with `bridge::`
    let fixed_code = fixed_code.replace("tsuchinoko::bridge::", "bridge::");
    // Just in case (double check)
    let fixed_code = fixed_code.replace("use tsuchinoko::bridge::", "use bridge::");


    let main_rs = format!(
        r#"// Generated by Tsuchinoko - Python to Rust Transpiler
// Author: Tane Channel Technology

{fixed_code}
"#
    );
    fs::write(format!("{name}/src/main.rs"), main_rs)?;

    Ok(())
}
