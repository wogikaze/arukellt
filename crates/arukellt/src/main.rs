//! Arukellt compiler CLI.
//!
//! Subcommands: compile, run, check

mod commands;
mod native;
mod runtime;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use ark_diagnostics::{DiagnosticSink, SourceMap, alias_warning_diagnostic, render_diagnostics};
use ark_target::{EmitKind, TargetId, parse_target};

#[derive(Parser)]
#[command(name = "arukellt", version, about = "The Arukellt compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum ScriptCommands {
    /// List all scripts in ark.toml
    List {
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Run a script from ark.toml
    Run {
        /// Script name
        name: String,
        /// Additional arguments passed to the script
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Compile an .ark file to Wasm
    Compile {
        /// Input .ark file
        file: PathBuf,
        /// Output file (default: <input>.wasm for T1, <input>.component.wasm for T3)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
        /// Emit kind (core-wasm, component, wit, all)
        #[arg(long)]
        emit: Option<EmitKind>,
        /// WIT file(s) for host import binding
        #[arg(long = "wit", value_name = "PATH")]
        wit_files: Vec<PathBuf>,
        /// Show memory profiling info (escape analysis, allocation hints, compiler RSS)
        #[arg(long)]
        profile_mem: bool,
        /// Show per-phase compilation time
        #[arg(long)]
        time: bool,
        /// Optimization level (0=none, 1=safe, 2=all). Default: 1
        #[arg(long, default_value = "1")]
        opt_level: u8,
        /// Disable specific optimization pass by name
        #[arg(long = "no-pass", value_name = "NAME")]
        no_pass: Vec<String>,
        /// MIR lowering path: legacy (default) or corehir
        #[arg(long = "mir-select", value_name = "PATH", default_value = "legacy")]
        mir_select: String,
        /// WASI world to target (e.g., wasi:cli/command, wasi:http/proxy)
        #[arg(long)]
        world: Option<String>,
        /// Generate P2-native component (skip P1 adapter, ~100KB smaller)
        #[arg(long)]
        p2_native: bool,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Compile and run an .ark file
    Run {
        /// Input .ark file
        file: PathBuf,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
        /// Grant directory access (format: path or path:ro or path:rw)
        #[arg(long = "dir", value_name = "PATH[:PERMS]")]
        dirs: Vec<String>,
        /// Deny filesystem access (overrides --dir)
        #[arg(long)]
        deny_fs: bool,
        /// Deny clock/time access
        #[arg(long)]
        deny_clock: bool,
        /// Deny random number access
        #[arg(long)]
        deny_random: bool,
        /// Show memory profiling info (escape analysis, allocation hints)
        #[arg(long)]
        profile_mem: bool,
        /// MIR lowering path: legacy or corehir (default)
        #[arg(long = "mir-select", value_name = "PATH", default_value = "corehir")]
        mir_select: String,
        /// Watch file for changes and recompile automatically
        #[arg(long)]
        watch: bool,
    },
    /// Type-check an .ark file without compiling
    Check {
        /// Input .ark file
        file: PathBuf,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
    },
    /// Discover and run tests
    Test {
        /// Input .ark file or directory
        file: PathBuf,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
        /// List tests without running them
        #[arg(long)]
        list: bool,
    },
    /// List available compile targets
    Targets,
    /// Manage and run project scripts
    Script {
        #[command(subcommand)]
        subcommand: ScriptCommands,
    },
    /// Start the LSP server (stdio transport)
    Lsp,
    /// Start the DAP debug adapter (stdio transport)
    DebugAdapter,
    /// Analyze a compiled Wasm binary
    Analyze {
        /// Analysis to perform
        #[arg(long = "wasm-size", value_name = "FILE")]
        wasm_size: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    // Check for alias warnings in raw args (clap already parsed TargetId,
    // but we want to warn on deprecated aliases)
    check_target_alias_warning();

    match cli.command {
        Commands::Compile {
            file,
            output,
            target,
            emit: emit_kind,
            wit_files,
            profile_mem,
            time,
            opt_level,
            no_pass,
            mir_select,
            world,
            p2_native,
            json,
        } => {
            let profile = target.profile();
            let emit_kind = emit_kind.unwrap_or(profile.default_emit_kind);
            commands::cmd_compile(
                file,
                output,
                target,
                emit_kind,
                wit_files,
                world,
                p2_native,
                profile_mem,
                time,
                opt_level,
                no_pass,
                &mir_select,
                json,
            );
        }
        Commands::Run {
            file,
            target,
            dirs,
            deny_fs,
            deny_clock,
            deny_random,
            profile_mem,
            mir_select,
            watch,
        } => {
            commands::cmd_run(
                file,
                target,
                dirs,
                deny_fs,
                deny_clock,
                deny_random,
                profile_mem,
                &mir_select,
                watch,
            );
        }
        Commands::Check { file, target } => {
            commands::cmd_check(file, target);
        }
        Commands::Test {
            file,
            target,
            json,
            list,
        } => {
            commands::cmd_test(file, target, json, list);
        }
        Commands::Targets => {
            commands::cmd_targets();
        }
        Commands::Script { subcommand } => match subcommand {
            ScriptCommands::List { json } => {
                commands::cmd_script_list(json);
            }
            ScriptCommands::Run { name, args } => {
                commands::cmd_script_run(name, args);
            }
        },
        Commands::Lsp => {
            commands::cmd_lsp();
        }
        Commands::DebugAdapter => {
            commands::cmd_debug_adapter();
        }
        Commands::Analyze { wasm_size } => {
            commands::cmd_analyze_wasm_size(&wasm_size);
        }
    }
}

/// Check raw CLI args for deprecated target aliases and emit warnings.
fn check_target_alias_warning() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--target")
        && let Some(value) = args.get(pos + 1)
        && let Ok(result) = parse_target(value)
        && let Some((used_alias, canonical_name)) = result.alias_parts()
    {
        emit_target_alias_warning(used_alias, canonical_name);
    }
    // Also check --target=value form
    for arg in &args {
        if let Some(value) = arg.strip_prefix("--target=")
            && let Ok(result) = parse_target(value)
            && let Some((used_alias, canonical_name)) = result.alias_parts()
        {
            emit_target_alias_warning(used_alias, canonical_name);
        }
    }
}

fn emit_target_alias_warning(used_alias: &str, canonical_name: &str) {
    let mut sink = DiagnosticSink::new();
    let source_map = SourceMap::new();
    sink.emit(alias_warning_diagnostic(used_alias, canonical_name));
    eprint!("{}", render_diagnostics(sink.diagnostics(), &source_map));
}
