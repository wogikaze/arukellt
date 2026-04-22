//! `arukellt` — thin wasm-runner shell.
//!
//! Per #583 (and ADR-029, #585) the legacy Rust compiler binary has been
//! retired. The `arukellt` crate now exists solely as a thin shell that
//! locates the selfhost compiler wasm and runs it under `wasmtime`,
//! forwarding all CLI arguments and exit codes verbatim.
//!
//! This binary intentionally has **no** dependency on `ark-driver`,
//! `ark-mir`, `ark-wasm`, `ark-stdlib`, or any other compiler-core crate.
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

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const PINNED_WASM_REL: &str = "bootstrap/arukellt-selfhost.wasm";
const STAGE2_WASM_REL: &str = ".build/selfhost/arukellt-s2.wasm";
const BOOTSTRAP_WASM_REL: &str = ".bootstrap-build/arukellt-s2.wasm";

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

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
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
        Ok(status) => {
            let code = status.code().unwrap_or(1);
            ExitCode::from(u8::try_from(code & 0xFF).unwrap_or(1))
        }
        Err(e) => {
            eprintln!(
                "arukellt: failed to invoke `wasmtime` (is it on PATH?): {e}\n\
                 Install wasmtime ≥ 30 or use scripts/run/arukellt-selfhost.sh."
            );
            ExitCode::from(127)
        }
    }
}
