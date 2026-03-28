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
        /// Show memory profiling info (escape analysis, allocation hints)
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
    },
    /// Type-check an .ark file without compiling
    Check {
        /// Input .ark file
        file: PathBuf,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
    },
    /// List available compile targets
    Targets,
    /// Start the LSP server (stdio transport)
    Lsp,
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
        } => {
            let profile = target.profile();
            let emit_kind = emit_kind.unwrap_or(profile.default_emit_kind);
            commands::cmd_compile(file, output, target, emit_kind, wit_files, profile_mem, time, opt_level, no_pass);
        }
        Commands::Run {
            file,
            target,
            dirs,
            deny_fs,
            deny_clock,
            deny_random,
            profile_mem,
        } => {
            commands::cmd_run(
                file,
                target,
                dirs,
                deny_fs,
                deny_clock,
                deny_random,
                profile_mem,
            );
        }
        Commands::Check { file, target } => {
            commands::cmd_check(file, target);
        }
        Commands::Targets => {
            commands::cmd_targets();
        }
        Commands::Lsp => {
            commands::cmd_lsp();
        }
    }
}

/// Check raw CLI args for deprecated target aliases and emit warnings.
fn check_target_alias_warning() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--target") {
        if let Some(value) = args.get(pos + 1) {
            if let Ok(result) = parse_target(value) {
                if let Some((used_alias, canonical_name)) = result.alias_parts() {
                    emit_target_alias_warning(used_alias, canonical_name);
                }
            }
        }
    }
    // Also check --target=value form
    for arg in &args {
        if let Some(value) = arg.strip_prefix("--target=") {
            if let Ok(result) = parse_target(value) {
                if let Some((used_alias, canonical_name)) = result.alias_parts() {
                    emit_target_alias_warning(used_alias, canonical_name);
                }
            }
        }
    }
}

fn emit_target_alias_warning(used_alias: &str, canonical_name: &str) {
    let mut sink = DiagnosticSink::new();
    let source_map = SourceMap::new();
    sink.emit(alias_warning_diagnostic(used_alias, canonical_name));
    eprint!("{}", render_diagnostics(sink.diagnostics(), &source_map));
}
