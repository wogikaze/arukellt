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
    // Test/diagnostic hook: when set, emit the policy `[BUG]` ICE format
    // and exit with status 101 without launching the selfhost compiler.
    // Used by the panic/ICE policy smoke test (#615) so the format can be
    // verified end-to-end without having to actually crash the compiler.
    if env::var("ARUKELLT_ICE_SMOKE").is_ok_and(|v| !v.is_empty()) {
        return report_ice("ARUKELLT_ICE_SMOKE hook fired (synthetic ICE for policy smoke test)");
    }

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
