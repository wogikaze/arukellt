//! `arukellt` — thin wasm-runner shell.
//!
//! Per #583 (and ADR-029, #585) the legacy Rust compiler binary has been
//! retired. The `arukellt` crate now exists solely as a thin shell that
//! locates the selfhost compiler wasm and runs it under `wasmtime`,
//! forwarding all CLI arguments and exit codes verbatim.
//!
//! This binary intentionally has **no** dependency on `ark-stdlib` or any
//! other compiler-core crate. (`ark-driver` was removed in #560 and
//! `ark-mir` in #561.)
//! All compiler behaviour lives in `src/compiler/main.ark` (the selfhost
//! source), compiled to `bootstrap/arukellt-selfhost.wasm` (the pinned
//! reference; see `bootstrap/PROVENANCE.md`) or to
//! `.build/selfhost/arukellt-s2.wasm` for fresh builds.
//!
//! Resolution order for the wasm artifact:
//!   1. `$ARUKELLT_SELFHOST_WASM` (explicit override)
//!   2. `<repo_root>/.build/selfhost/arukellt-s2.wasm`
//!   3. `<repo_root>/.bootstrap-build/arukellt-s2.wasm`
//!   4. `<repo_root>/bootstrap/arukellt-selfhost.wasm` (committed pin)
//!
//! Wasmtime is invoked as `wasmtime run --dir=<repo_root> <wasm> -- <args...>`.
//! Users who want richer plumbing should prefer `scripts/run/arukellt-selfhost.sh`.

use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const PINNED_WASM_REL: &str = "bootstrap/arukellt-selfhost.wasm";
const STAGE2_WASM_REL: &str = ".build/selfhost/arukellt-s2.wasm";
const BOOTSTRAP_WASM_REL: &str = ".bootstrap-build/arukellt-s2.wasm";

/// Exit status used for Internal Compiler Errors (ICE) per the
/// panic/ICE policy (`docs/compiler/panic-ice-policy.md` section 1).
const ICE_EXIT_STATUS: u8 = 101;

/// Render an ICE message to stderr in the format mandated by the
/// panic/ICE policy:
///   `[BUG] internal compiler error: <reason>`
/// followed by a report URL and a backtrace hint. Returns the policy
/// exit status (101) so call sites can `return report_ice(...)`.
///
/// Invoked by the CLI shim when it detects that the spawned selfhost
/// compiler terminated abnormally (signal, abort, segfault, or wasmtime
/// trap) and therefore could not have produced its own structured
/// `[BUG]` line. When the child exits with code 101 itself, the
/// upstream emitter is trusted and the code is passed through verbatim
/// without prepending another `[BUG]` line.
fn report_ice(reason: &str) -> ExitCode {
    eprintln!("[BUG] internal compiler error: {reason}");
    eprintln!("  please report this at: https://github.com/wogikaze/arukellt/issues/new");
    eprintln!("  hint: re-run with RUST_BACKTRACE=1 for a full trace");
    ExitCode::from(ICE_EXIT_STATUS)
}

/// Classify a child process exit status into either a normal exit code
/// (passed through) or an ICE reason string (caller emits `[BUG]`).
///
/// Returns `Ok(code)` when the child exited normally, including code
/// 101 (the child already emitted its own `[BUG]` line and the shim
/// must not double-prepend).
/// Returns `Err(reason)` when the shim itself must emit a `[BUG]` line
/// because the child died abnormally (signal-killed or fatal-signal exit).
fn classify_child_exit(code: Option<i32>) -> Result<u8, String> {
    match code {
        None => Err("selfhost compiler terminated by signal (no exit code)".to_string()),
        Some(101) => Ok(101),
        // Conventional Unix shell encoding for fatal signals (128 + signo):
        // SIGILL=4 (132), SIGTRAP=5 (133), SIGABRT=6 (134), SIGFPE=8 (136),
        // SIGBUS=10 (138), SIGSEGV=11 (139).
        Some(n @ (132 | 133 | 134 | 136 | 138 | 139)) => Err(format!(
            "selfhost compiler aborted (exit {n}, fatal signal)"
        )),
        Some(n) => Ok(u8::try_from(n & 0xFF).unwrap_or(1)),
    }
}

fn find_repo_root() -> PathBuf {
    // Walk upward from the binary's directory looking for a workspace marker.
    if let Ok(exe) = env::current_exe() {
        let mut p: PathBuf = exe;
        p.pop(); // strip binary name
        while let Some(parent) = p.parent() {
            if p.join("Cargo.lock").is_file() || p.join("ark.toml").is_file() {
                return p;
            }
            p = parent.to_path_buf();
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn find_selfhost_wasm(root: &Path) -> Option<PathBuf> {
    if let Ok(p) = env::var("ARUKELLT_SELFHOST_WASM")
        && !p.is_empty()
    {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    for rel in [STAGE2_WASM_REL, BOOTSTRAP_WASM_REL, PINNED_WASM_REL] {
        let pb = root.join(rel);
        if pb.is_file() {
            return Some(pb);
        }
    }
    None
}

fn find_manifest_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        if current.join("ark.toml").is_file() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn load_manifest_scripts(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let manifest_path = root.join("ark.toml");
    let manifest = load_manifest_value(&manifest_path)?;
    let Some(scripts) = manifest.get("scripts") else {
        return Ok(BTreeMap::new());
    };
    let table = scripts
        .as_table()
        .ok_or_else(|| "[scripts] must be a TOML table".to_string())?;
    let mut result = BTreeMap::new();
    for (name, value) in table {
        let command = value
            .as_str()
            .ok_or_else(|| format!("[scripts].{name} must be a string command"))?;
        result.insert(name.to_string(), command.to_string());
    }
    Ok(result)
}

fn load_manifest_value(manifest_path: &Path) -> Result<toml::Value, String> {
    let content = std::fs::read_to_string(manifest_path)
        .map_err(|e| format!("failed to read {}: {e}", manifest_path.display()))?;
    content
        .parse()
        .map_err(|e| format!("failed to parse {}: {e}", manifest_path.display()))
}

fn manifest_string(manifest: &toml::Value, keys: &[&str]) -> Option<String> {
    let mut value = manifest;
    for key in keys {
        value = value.get(*key)?;
    }
    value.as_str().map(ToString::to_string)
}

fn print_script_usage() {
    eprintln!("Usage:");
    eprintln!("  arukellt script list [--json]");
    eprintln!("  arukellt script run <name> [args...]");
}

fn cmd_script(script_args: &[String]) -> ExitCode {
    if script_args.is_empty() || script_args[0] == "--help" || script_args[0] == "-h" {
        print_script_usage();
        return ExitCode::from(if script_args.is_empty() { 2 } else { 0 });
    }

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = match find_manifest_root(&cwd) {
        Some(root) => root,
        None => {
            eprintln!("arukellt script: ark.toml not found in current directory or any parent");
            return ExitCode::from(2);
        }
    };
    let scripts = match load_manifest_scripts(&root) {
        Ok(scripts) => scripts,
        Err(message) => {
            eprintln!("arukellt script: {message}");
            return ExitCode::from(2);
        }
    };

    match script_args[0].as_str() {
        "list" => {
            let json = script_args.iter().skip(1).any(|arg| arg == "--json");
            if json {
                let mut first = true;
                print!("[");
                for (name, command) in &scripts {
                    if !first {
                        print!(",");
                    }
                    first = false;
                    print!(
                        "{{\"name\":\"{}\",\"command\":\"{}\"}}",
                        json_escape(name),
                        json_escape(command)
                    );
                }
                println!("]");
            } else {
                for name in scripts.keys() {
                    println!("{name}");
                }
            }
            ExitCode::SUCCESS
        }
        "run" => {
            let Some(name) = script_args.get(1) else {
                eprintln!("arukellt script run: missing script name");
                print_script_usage();
                return ExitCode::from(2);
            };
            let Some(command) = scripts.get(name) else {
                eprintln!("arukellt script run: script '{name}' not found");
                if scripts.is_empty() {
                    eprintln!("  no scripts are defined in ark.toml");
                } else {
                    eprintln!(
                        "  available scripts: {}",
                        scripts.keys().cloned().collect::<Vec<_>>().join(", ")
                    );
                }
                return ExitCode::from(2);
            };

            #[cfg(windows)]
            let mut child = {
                let mut cmd = Command::new("cmd");
                cmd.arg("/C").arg(command);
                cmd
            };
            #[cfg(not(windows))]
            let mut child = {
                let mut cmd = Command::new("sh");
                cmd.arg("-c").arg(command).arg(name);
                for arg in script_args.iter().skip(2) {
                    cmd.arg(arg);
                }
                cmd
            };

            child.current_dir(&root);
            child.env("ARUKELLT_PACKAGE_ROOT", &root);
            child.env("ARUKELLT_SCRIPT_NAME", name);
            let status = match child.status() {
                Ok(status) => status,
                Err(e) => {
                    eprintln!("arukellt script run: failed to launch script '{name}': {e}");
                    return ExitCode::from(127);
                }
            };
            match classify_child_exit(status.code()) {
                Ok(0) => ExitCode::SUCCESS,
                Ok(code) => {
                    eprintln!("arukellt script run: script '{name}' failed with exit code {code}");
                    ExitCode::from(code)
                }
                Err(reason) => report_ice(&reason),
            }
        }
        other => {
            eprintln!("arukellt script: unknown subcommand '{other}'");
            print_script_usage();
            ExitCode::from(2)
        }
    }
}

fn cmd_build_project(build_args: &[String], repo_root: &Path) -> ExitCode {
    let user_cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_root = match find_manifest_root(&user_cwd) {
        Some(root) => root,
        None => {
            eprintln!("arukellt build: ark.toml not found in current directory or any parent");
            return ExitCode::from(1);
        }
    };
    let manifest_path = project_root.join("ark.toml");
    let manifest = match load_manifest_value(&manifest_path) {
        Ok(manifest) => manifest,
        Err(message) => {
            eprintln!("arukellt build: {message}");
            return ExitCode::from(2);
        }
    };
    let Some(entry) = manifest_string(&manifest, &["bin", "path"])
        .or_else(|| manifest_string(&manifest, &["entry"]))
    else {
        eprintln!("arukellt build: no 'path' (under [bin]) or 'entry' field found in ark.toml");
        return ExitCode::from(2);
    };
    let package_name =
        manifest_string(&manifest, &["package", "name"]).unwrap_or_else(|| "output".to_string());

    let mut target = "wasm32-wasi-p2".to_string();
    let mut emit = "wasm".to_string();
    let mut output = project_root.join(format!("{package_name}.wasm"));
    let mut passthrough = Vec::new();
    let mut i = 0;
    while i < build_args.len() {
        match build_args[i].as_str() {
            "--target" => {
                i += 1;
                if let Some(value) = build_args.get(i) {
                    target = value.clone();
                }
            }
            "--emit" => {
                i += 1;
                if let Some(value) = build_args.get(i) {
                    emit = value.clone();
                }
            }
            "-o" | "--output" => {
                i += 1;
                if let Some(value) = build_args.get(i) {
                    let path = PathBuf::from(value);
                    output = if path.is_absolute() {
                        path
                    } else {
                        project_root.join(path)
                    };
                }
            }
            "--opt-level" => {
                passthrough.push(build_args[i].clone());
                i += 1;
                if let Some(value) = build_args.get(i) {
                    passthrough.push(value.clone());
                }
            }
            other if other.starts_with('-') => passthrough.push(other.to_string()),
            other => {
                eprintln!("arukellt build: unexpected argument '{other}'");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }

    let wasm = match find_selfhost_wasm(repo_root) {
        Some(p) => p,
        None => {
            eprintln!("arukellt build: selfhost wasm not found (needed to compile).");
            return ExitCode::from(127);
        }
    };
    let mut compile_cmd = Command::new("wasmtime");
    compile_cmd.current_dir("/");
    compile_cmd.arg("run").arg("--dir=/::/");
    compile_cmd.arg(format!("--dir={}", repo_root.display()));
    compile_cmd.arg(format!("--dir={}", project_root.display()));

    let mut compile_args = vec![
        "compile".to_string(),
        rewrite_path_arg(&project_root, &entry),
        "-o".to_string(),
        rewrite_path_arg(&project_root, &output.to_string_lossy()),
        "--emit".to_string(),
        emit,
        "--target".to_string(),
        target,
    ];
    compile_args.extend(passthrough);
    compile_cmd.arg(&wasm).arg("--").args(&compile_args);

    let status = match compile_cmd.status() {
        Ok(status) => status,
        Err(e) => {
            eprintln!("arukellt build: failed to invoke `wasmtime` for compilation: {e}");
            return ExitCode::from(127);
        }
    };
    match classify_child_exit(status.code()) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(code),
        Err(reason) => report_ice(&reason),
    }
}

/// Rewrite a path-like argument so it resolves under wasmtime's `/` cwd.
///
/// The selfhost wasm resolves paths against the wasi cwd preopen and does
/// not consult sibling preopens for absolute paths. Our shim runs wasmtime
/// with `cwd=/` and `--dir=/::/` so the entire host filesystem is reachable;
/// each path argument is rewritten as follows:
///
/// * Absolute path `/tmp/foo.ark` -> `tmp/foo.ark`.
/// * Relative path `src/x.ark`, when the user's cwd is `<USER_CWD>` and
///   the file exists relative to it, becomes `<USER_CWD without leading
///   slash>/src/x.ark`.
/// * `--key=<path>` flag-with-value forms are recognised and rewritten on
///   the value half only.
/// * Args that do not look like paths (no `/`, do not name an existing
///   file, no `.ark`/`.wasm`/`.wat`/`.wit`/`.toml` extension) are
///   forwarded unchanged.
fn rewrite_path_arg(user_cwd: &Path, arg: &str) -> String {
    fn rewrite_value(user_cwd: &Path, value: &str) -> Option<String> {
        if let Some(rest) = value.strip_prefix('/') {
            return Some(rest.to_string());
        }
        let p = Path::new(value);
        // Heuristic: only resolve relative paths that already exist OR that
        // carry one of the recognised compiler input/output extensions.
        // This avoids mangling option values like `wasm32-wasi-p1`.
        let looks_like_path = p
            .components()
            .any(|c| matches!(c, std::path::Component::Normal(_)))
            && (user_cwd.join(p).exists()
                || matches!(
                    p.extension().and_then(|e| e.to_str()),
                    Some("ark" | "wasm" | "wat" | "wit" | "toml" | "json" | "md")
                ));
        if !looks_like_path {
            return None;
        }
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            user_cwd.join(p)
        };
        abs.to_string_lossy()
            .strip_prefix('/')
            .map(|s| s.to_string())
    }

    if let Some(eq_idx) = arg.find('=')
        && arg.starts_with('-')
    {
        let (key, rest) = arg.split_at(eq_idx);
        let value = &rest[1..];
        if let Some(rewritten) = rewrite_value(user_cwd, value) {
            return format!("{key}={rewritten}");
        }
        return arg.to_string();
    }
    if arg.starts_with('-') {
        return arg.to_string();
    }
    rewrite_value(user_cwd, arg).unwrap_or_else(|| arg.to_string())
}

/// Handle `arukellt run <file.ark>` — compile via the selfhost compiler,
/// then execute the resulting wasm with wasmtime, streaming its output.
fn cmd_run(run_args: &[String], root: &Path) -> ExitCode {
    if run_args.is_empty() {
        eprintln!("arukellt run: missing input file\nUsage: arukellt run <file.ark>");
        return ExitCode::from(2);
    }
    let wasm = match find_selfhost_wasm(root) {
        Some(p) => p,
        None => {
            eprintln!("arukellt: selfhost wasm not found (needed to compile).");
            return ExitCode::from(127);
        }
    };
    let input_file = &run_args[0];
    let out_path = {
        let p = Path::new(input_file);
        let mut out = p.file_stem().unwrap_or_default().to_os_string();
        out.push(".wasm");
        root.join(out)
    };
    let user_cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

    // Step 1 — compile.  Tell the selfhost compiler to write to a .wasm file.
    let mut compile_cmd = Command::new("wasmtime");
    compile_cmd.current_dir("/");
    compile_cmd.arg("run").arg("--dir=/::/");
    compile_cmd.arg(format!("--dir={}", root.display()));
    if let Ok(cwd) = env::current_dir()
        && cwd.is_dir()
        && cwd != root
        && cwd != Path::new("/")
    {
        compile_cmd.arg(format!("--dir={}", cwd.display()));
    }
    let mut compile_args = vec![
        "compile".to_string(),
        rewrite_path_arg(&user_cwd, input_file),
        "-o".to_string(),
        rewrite_path_arg(&user_cwd, &out_path.to_string_lossy()),
        "--emit".to_string(),
        "wasm".to_string(),
    ];
    for arg in &run_args[1..] {
        compile_args.push(rewrite_path_arg(&user_cwd, arg));
    }
    compile_cmd.arg(&wasm).arg("--").args(&compile_args);
    let compile_status = match compile_cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("arukellt: failed to invoke `wasmtime` for compilation: {e}");
            return ExitCode::from(127);
        }
    };
    if !compile_status.success() {
        return ExitCode::from(compile_status.code().unwrap_or(1) as u8);
    }

    // Step 2 — run the compiled wasm.
    let mut run_cmd = Command::new("wasmtime");
    run_cmd.current_dir("/");
    run_cmd.arg("run").arg("--dir=/::/");
    run_cmd.arg(rewrite_path_arg(&user_cwd, &out_path.to_string_lossy()));
    let run_status = match run_cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("arukellt: failed to execute compiled wasm via `wasmtime`: {e}");
            let _ = std::fs::remove_file(&out_path);
            return ExitCode::from(127);
        }
    };
    let _ = std::fs::remove_file(&out_path);
    match classify_child_exit(run_status.code()) {
        Ok(code) => ExitCode::from(code),
        Err(reason) => report_ice(&reason),
    }
}

fn main() -> ExitCode {
    // Test/diagnostic hook: when set, emit the policy `[BUG]` ICE format
    // and exit with status 101 without launching the selfhost compiler.
    // Used by the panic/ICE policy smoke test (#615) so the format can be
    // verified end-to-end without having to actually crash the compiler.
    if env::var("ARUKELLT_ICE_SMOKE").is_ok_and(|v| !v.is_empty()) {
        return report_ice("ARUKELLT_ICE_SMOKE hook fired (synthetic ICE for policy smoke test)");
    }

    let args: Vec<String> = env::args().skip(1).collect();

    // Intercept commands that need host-side project/script handling.
    if args.first().map(|s| s.as_str()) == Some("run") {
        return cmd_run(&args[1..], &find_repo_root());
    }
    if args.first().map(|s| s.as_str()) == Some("script") {
        return cmd_script(&args[1..]);
    }
    if args.first().map(|s| s.as_str()) == Some("build") {
        return cmd_build_project(&args[1..], &find_repo_root());
    }

    let root = find_repo_root();

    let wasm = match find_selfhost_wasm(&root) {
        Some(p) => p,
        None => {
            eprintln!(
                "arukellt: selfhost wasm not found.\n\
                 Looked at: $ARUKELLT_SELFHOST_WASM, {}, {}, {}\n\
                 Build it (`python3 scripts/manager.py selfhost fixpoint --build`) \
                 or set ARUKELLT_SELFHOST_WASM.",
                root.join(STAGE2_WASM_REL).display(),
                root.join(BOOTSTRAP_WASM_REL).display(),
                root.join(PINNED_WASM_REL).display(),
            );
            return ExitCode::from(127);
        }
    };

    // Grant filesystem access to the host root so absolute-path arguments
    // remain reachable, plus the original cwd so relative paths still work
    // for invocations like `cd <project> && arukellt build`. We then run
    // wasmtime with cwd=`/` (combined with `--dir=/::/` and stripping the
    // leading `/` from each arg) because the selfhost wasm resolves paths
    // against the cwd preopen and does not consult sibling preopens for
    // absolute paths. This keeps `arukellt check /tmp/foo.ark` working
    // transparently — matching the historical native Rust binary.
    let mut cmd = Command::new("wasmtime");
    cmd.current_dir("/");
    cmd.arg("run").arg("--dir=/::/");
    if root.is_dir() {
        cmd.arg(format!("--dir={}", root.display()));
    }
    if let Ok(cwd) = env::current_dir()
        && cwd.is_dir()
        && cwd != root
        && cwd != Path::new("/")
    {
        cmd.arg(format!("--dir={}", cwd.display()));
    }
    let user_cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let rewritten: Vec<String> = args
        .iter()
        .map(|a| rewrite_path_arg(&user_cwd, a))
        .collect();
    cmd.arg(&wasm).arg("--").args(&rewritten);

    match cmd.status() {
        Ok(status) => match classify_child_exit(status.code()) {
            Ok(code) => ExitCode::from(code),
            Err(reason) => report_ice(&reason),
        },
        Err(e) => {
            eprintln!(
                "arukellt: failed to invoke `wasmtime` (is it on PATH?): {e}\n\
                 Install wasmtime ≥ 30 or use scripts/run/arukellt-selfhost.sh."
            );
            ExitCode::from(127)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: signal-killed child (Unix) must be surfaced as an ICE.
    #[test]
    fn classify_signal_killed_is_ice() {
        let r = classify_child_exit(None);
        assert!(r.is_err(), "signal-killed must be ICE");
        assert!(
            r.as_ref().unwrap_err().contains("signal"),
            "reason should mention signal: {r:?}"
        );
    }

    /// Regression: shell-encoded fatal signal exits (128 + signo) must be ICE.
    #[test]
    fn classify_fatal_signal_codes_are_ice() {
        for code in [132, 133, 134, 136, 138, 139] {
            let r = classify_child_exit(Some(code));
            assert!(r.is_err(), "exit {code} must be ICE");
        }
    }

    /// Regression: normal non-zero exits (compile errors, user errors) must
    /// pass through and NOT be re-flagged as ICE.
    #[test]
    fn classify_normal_exits_pass_through() {
        assert_eq!(classify_child_exit(Some(0)).ok(), Some(0));
        assert_eq!(classify_child_exit(Some(1)).ok(), Some(1));
        assert_eq!(classify_child_exit(Some(2)).ok(), Some(2));
        assert_eq!(classify_child_exit(Some(127)).ok(), Some(127));
    }

    /// Regression: when the child already exits with the ICE convention
    /// (101), trust the upstream `[BUG]` emitter and pass the code
    /// through without prepending a second `[BUG]` line.
    #[test]
    fn classify_child_ice_exit_is_passed_through() {
        assert_eq!(classify_child_exit(Some(101)).ok(), Some(101));
    }
}
