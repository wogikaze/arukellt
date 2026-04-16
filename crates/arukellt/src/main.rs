//! Arukellt compiler CLI.
//!
//! Subcommands: compile, run, check

mod cmd_doc;
mod commands;
mod native;
mod runtime;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use ark_diagnostics::{DiagnosticSink, SourceMap, alias_warning_diagnostic, render_diagnostics};
use ark_target::{EmitKind, TargetId, WasiVersion, parse_target};

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

/// Template to use when initializing a new Arukellt project.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum InitTemplate {
    /// Minimal Hello World project (default)
    Minimal,
    /// CLI tool with argument parsing boilerplate
    Cli,
    /// Project with test functions (run with `arukellt test`)
    #[value(name = "with-tests")]
    WithTests,
    /// WASI host API usage example (requires --target wasm32-wasi-p2)
    #[value(name = "wasi-host")]
    WasiHost,
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
        /// Strip Name Section from output Wasm (omit debug symbols)
        #[arg(long)]
        strip_debug: bool,
        /// Disable specific optimization pass by name
        #[arg(long = "no-pass", value_name = "NAME")]
        no_pass: Vec<String>,
        /// MIR lowering path: legacy or corehir (default)
        #[arg(long = "mir-select", value_name = "PATH", default_value = "corehir")]
        mir_select: String,
        /// WASI world to target (e.g., wasi:cli/command, wasi:http/proxy)
        #[arg(long)]
        world: Option<String>,
        /// Generate P2-native component (skip P1 adapter, ~100KB smaller)
        #[arg(long)]
        p2_native: bool,
        /// WASI version for component output: p1 (default) or p2.
        /// `--wasi-version p2` is equivalent to `--p2-native` and requires
        /// `--target wasm32-wasi-p2`.  Full P2 import-table switching in the
        /// T3 emitter is deferred; see issues/open/510-t3-p2-import-table-switch.md.
        #[arg(long, value_name = "VERSION", default_value_t = WasiVersion::P1)]
        wasi_version: WasiVersion,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
        /// Resolve only reachable symbols in multi-module crates (compile-speed; opt-in)
        #[arg(long)]
        lazy_resolve: bool,
        /// Force full-crate resolve (default). Overrides `--lazy-resolve`.
        #[arg(long)]
        no_lazy_resolve: bool,
    },
    /// Initialize a new Arukellt project in the specified directory
    Init {
        /// Project directory
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Project template to use
        #[arg(long, value_name = "TEMPLATE", default_value = "minimal")]
        template: InitTemplate,
        /// List available project templates
        #[arg(long)]
        list_templates: bool,
    },
    /// Build the project in the current directory (requires ark.toml)
    Build {
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
        /// Optimization level (0=none, 1=safe, 2=all). Default: 1
        #[arg(long, default_value = "1")]
        opt_level: u8,
        /// Strip Name Section from output Wasm (omit debug symbols)
        #[arg(long)]
        strip_debug: bool,
        /// MIR lowering path: legacy or corehir (default)
        #[arg(long = "mir-select", value_name = "PATH", default_value = "corehir")]
        mir_select: String,
        /// Show memory profiling info
        #[arg(long)]
        profile_mem: bool,
        /// Show per-phase compilation time
        #[arg(long)]
        time: bool,
        /// Resolve only reachable symbols in multi-module crates (compile-speed; opt-in)
        #[arg(long)]
        lazy_resolve: bool,
        /// Force full-crate resolve (default). Overrides `--lazy-resolve`.
        #[arg(long)]
        no_lazy_resolve: bool,
        /// WASI interface version for the compile pipeline (default p1).
        #[arg(long, value_name = "VERSION", default_value_t = WasiVersion::P1)]
        wasi_version: WasiVersion,
    },
    /// Format .ark source files
    Fmt {
        /// Input .ark file(s). If omitted, formats all .ark files in the project.
        #[arg(value_name = "FILE")]
        files: Vec<PathBuf>,
        /// Check formatting without modifying files (exit 1 if not formatted)
        #[arg(long)]
        check: bool,
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
        /// Strip Name Section from compiled Wasm (omit debug symbols)
        #[arg(long)]
        strip_debug: bool,
        /// MIR lowering path: legacy or corehir (default)
        #[arg(long = "mir-select", value_name = "PATH", default_value = "corehir")]
        mir_select: String,
        /// Watch file for changes and recompile automatically
        #[arg(long)]
        watch: bool,
        /// Resolve only reachable symbols in multi-module crates (compile-speed; opt-in)
        #[arg(long)]
        lazy_resolve: bool,
        /// Force full-crate resolve (default). Overrides `--lazy-resolve`.
        #[arg(long)]
        no_lazy_resolve: bool,
    },
    /// Type-check an .ark file without compiling
    Check {
        /// Input .ark file
        file: PathBuf,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
        /// Resolve only reachable symbols in multi-module crates (compile-speed; opt-in)
        #[arg(long)]
        lazy_resolve: bool,
        /// Force full-crate resolve (default). Overrides `--lazy-resolve`.
        #[arg(long)]
        no_lazy_resolve: bool,
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
        /// Only run tests whose name contains this substring
        #[arg(long, value_name = "NAME")]
        filter: Option<String>,
        /// Resolve only reachable symbols in multi-module crates (compile-speed; opt-in)
        #[arg(long)]
        lazy_resolve: bool,
        /// Force full-crate resolve (default). Overrides `--lazy-resolve`.
        #[arg(long)]
        no_lazy_resolve: bool,
    },
    /// List available compile targets
    Targets,
    /// Manage and run project scripts
    Script {
        #[command(subcommand)]
        subcommand: ScriptCommands,
    },
    /// Start the LSP server (stdio transport)
    Lsp {
        /// Use stdio transport (default and only supported transport; accepted for compatibility)
        #[arg(long)]
        stdio: bool,
    },
    /// Start the DAP debug adapter (stdio transport)
    DebugAdapter,
    /// Run lint rules on .ark source files
    Lint {
        /// Input .ark file
        file: Option<PathBuf>,
        /// Compile target
        #[arg(long, default_value = "wasm32-wasi-p1")]
        target: TargetId,
        /// List available lint rules
        #[arg(long)]
        list: bool,
        /// Resolve only reachable symbols in multi-module crates (compile-speed; opt-in)
        #[arg(long)]
        lazy_resolve: bool,
        /// Force full-crate resolve (default). Overrides `--lazy-resolve`.
        #[arg(long)]
        no_lazy_resolve: bool,
    },
    /// Analyze a compiled Wasm binary
    Analyze {
        /// Analysis to perform
        #[arg(long = "wasm-size", value_name = "FILE")]
        wasm_size: PathBuf,
    },
    /// Look up standard library documentation for a symbol or module
    Doc {
        /// Symbol or module to look up (e.g. "println", "std::host::http::get", "std::host::http")
        symbol: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Show availability for a specific target
        #[arg(long, value_name = "TARGET")]
        target: Option<TargetId>,
        /// Show all matching candidates even if an exact match exists
        #[arg(long)]
        all: bool,
    },
    /// Link multiple Wasm components into a single composed component
    Compose {
        /// Input component .wasm files (two or more)
        #[arg(value_name = "COMPONENT", required = true)]
        inputs: Vec<PathBuf>,
        /// Output composed component file
        #[arg(
            short,
            long,
            value_name = "FILE",
            default_value = "composed.component.wasm"
        )]
        output: PathBuf,
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
            strip_debug,
            no_pass,
            mir_select,
            world,
            p2_native,
            wasi_version,
            json,
            lazy_resolve,
            no_lazy_resolve,
        } => {
            let profile = target.profile();
            let emit_kind = emit_kind.unwrap_or(profile.default_emit_kind);
            // --wasi-version p2 is equivalent to --p2-native
            let p2_native = p2_native || wasi_version == WasiVersion::P2;
            let lazy_reachability =
                commands::effective_lazy_reachability(lazy_resolve, no_lazy_resolve);
            commands::cmd_compile(
                file,
                output,
                target,
                emit_kind,
                wit_files,
                world,
                p2_native,
                wasi_version,
                profile_mem,
                time,
                opt_level,
                strip_debug,
                no_pass,
                &mir_select,
                json,
                lazy_reachability,
            );
        }
        Commands::Init {
            path,
            template,
            list_templates,
        } => {
            if list_templates {
                commands::cmd_list_templates();
            } else {
                commands::cmd_init(path, template);
            }
        }
        Commands::Fmt { files, check } => {
            commands::cmd_fmt(files, check);
        }
        Commands::Build {
            target,
            opt_level,
            strip_debug,
            mir_select,
            profile_mem,
            time,
            lazy_resolve,
            no_lazy_resolve,
            wasi_version,
        } => {
            let lazy_reachability =
                commands::effective_lazy_reachability(lazy_resolve, no_lazy_resolve);
            commands::cmd_build(
                target,
                opt_level,
                strip_debug,
                &mir_select,
                profile_mem,
                time,
                lazy_reachability,
                wasi_version,
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
            strip_debug,
            mir_select,
            watch,
            lazy_resolve,
            no_lazy_resolve,
        } => {
            let lazy_reachability =
                commands::effective_lazy_reachability(lazy_resolve, no_lazy_resolve);
            commands::cmd_run(
                file,
                target,
                dirs,
                deny_fs,
                deny_clock,
                deny_random,
                profile_mem,
                strip_debug,
                &mir_select,
                watch,
                lazy_reachability,
            );
        }
        Commands::Check {
            file,
            target,
            lazy_resolve,
            no_lazy_resolve,
        } => {
            let lazy_reachability =
                commands::effective_lazy_reachability(lazy_resolve, no_lazy_resolve);
            commands::cmd_check(file, target, lazy_reachability);
        }
        Commands::Test {
            file,
            target,
            json,
            list,
            filter,
            lazy_resolve,
            no_lazy_resolve,
        } => {
            let lazy_reachability =
                commands::effective_lazy_reachability(lazy_resolve, no_lazy_resolve);
            commands::cmd_test(file, target, json, list, filter, lazy_reachability);
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
        Commands::Lsp { .. } => {
            commands::cmd_lsp();
        }
        Commands::DebugAdapter => {
            commands::cmd_debug_adapter();
        }
        Commands::Analyze { wasm_size } => {
            commands::cmd_analyze_wasm_size(&wasm_size);
        }
        Commands::Lint {
            file,
            target,
            list,
            lazy_resolve,
            no_lazy_resolve,
        } => {
            let lazy_reachability =
                commands::effective_lazy_reachability(lazy_resolve, no_lazy_resolve);
            commands::cmd_lint(file, target, list, lazy_reachability);
        }
        Commands::Doc {
            symbol,
            json,
            target,
            all,
        } => {
            let found = cmd_doc::cmd_doc(&symbol, json, target.as_ref(), all);
            if !found {
                std::process::exit(1);
            }
        }
        Commands::Compose { inputs, output } => {
            commands::cmd_compose(inputs, output);
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
