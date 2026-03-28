//! Integration test harness: manifest-driven `.ark` fixture tests.
//!
//! Reads `tests/fixtures/manifest.txt` for the fixture list.
//! For each entry:
//! - `run:path`               → compile + run, compare stdout against `.expected`
//! - `diag:path`              → compile-fail, check first line of `.diag` in output
//! - `module-run:path`        → same as run, for multi-file modules
//! - `module-diag:path`       → same as diag, for multi-file modules
//! - `t3-compile:path`        → T3 compile-only, verify exit code 0
//! - `component-compile:path` → T3 component compile, verify exit code 0
//! - `compile-error:path`     → compile with flags from `.flags`, expect failure + check `.diag`
//!
//! Self-check: verifies every `.ark` entry point on disk is listed in the manifest.
//!
//! Fixtures run in parallel using a work-stealing channel over all available CPU cores.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, mpsc};

/// A single manifest entry.
struct ManifestEntry {
    kind: String,
    path: String, // relative to tests/fixtures/
}

/// Outcome of running a single fixture.
enum FixtureOutcome {
    Pass,
    Fail(String),
    Skip(String),
}

/// Parse `manifest.txt`: each non-empty, non-comment line is `kind:path`.
fn load_manifest(manifest_path: &Path) -> Vec<ManifestEntry> {
    let content = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("Cannot read manifest at {:?}: {}", manifest_path, e));
    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((kind, path)) = line.split_once(':') {
            entries.push(ManifestEntry {
                kind: kind.to_string(),
                path: path.to_string(),
            });
        }
    }
    entries
}

/// Recursively discover all `.ark` files under `dir`.
fn discover_ark_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                files.extend(discover_ark_files(&path));
            } else if path.extension().is_some_and(|e| e == "ark") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Determine which `.ark` files are entry points (not module helpers).
fn entry_points(fixture_dir: &Path) -> HashSet<String> {
    let all_ark = discover_ark_files(fixture_dir);
    let mut result = HashSet::new();
    for path in &all_ark {
        let rel = path.strip_prefix(fixture_dir).unwrap();
        let dir = path.parent().unwrap();
        let is_main = path.file_name() == Some(std::ffi::OsStr::new("main.ark"));
        if !is_main && dir.join("main.ark").exists() {
            continue;
        }
        result.insert(rel.display().to_string());
    }
    result
}

fn arukellt_binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("arukellt");
    if !path.exists() {
        let mut release = path.clone();
        release.pop();
        release.pop();
        release.push("release");
        release.push("arukellt");
        if release.exists() {
            return release;
        }
    }
    path
}

/// Run a single manifest entry. Safe to call from multiple threads.
fn run_fixture_entry(bin: &Path, fixture_dir: &Path, entry: &ManifestEntry) -> FixtureOutcome {
    let fixture = fixture_dir.join(&entry.path);
    let name = &entry.path;
    let mir_debug = std::env::var_os("ARUKELLT_TEST_MIR_DEBUG").is_some();

    match entry.kind.as_str() {
        "run" | "module-run" => {
            let expected_path = fixture.with_extension("expected");
            if !expected_path.exists() {
                return FixtureOutcome::Skip(format!("{} (no .expected file)", name));
            }
            let expected = std::fs::read_to_string(&expected_path).unwrap();

            let flags_path = fixture.with_extension("flags");
            let extra_args: Vec<String> = if flags_path.exists() {
                std::fs::read_to_string(&flags_path)
                    .unwrap()
                    .split_whitespace()
                    .map(String::from)
                    .collect()
            } else {
                vec![]
            };

            let output = Command::new(bin)
                .arg("run")
                .args(&extra_args)
                .arg(&fixture)
                .output()
                .expect("failed to run arukellt");

            if mir_debug {
                let debug = Command::new(bin)
                    .arg("check")
                    .arg(&fixture)
                    .env(
                        "ARUKELLT_DUMP_PHASES",
                        "parse,resolve,mir,optimized-mir,backend-plan",
                    )
                    .env("ARUKELLT_DUMP_DIAGNOSTICS", "1")
                    .output()
                    .expect("failed to run arukellt debug check");
                let mut msg = format!("[mir-debug] fixture={}\n", name);
                if !debug.stdout.is_empty() {
                    msg.push_str(&String::from_utf8_lossy(&debug.stdout));
                }
                if !debug.stderr.is_empty() {
                    msg.push_str(&String::from_utf8_lossy(&debug.stderr));
                }
                eprint!("{}", msg);
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.as_ref() == expected {
                FixtureOutcome::Pass
            } else {
                FixtureOutcome::Fail(format!(
                    "FAIL [{}] {}\n  expected: {:?}\n  got:      {:?}",
                    entry.kind,
                    name,
                    expected.lines().next().unwrap_or(""),
                    stdout.lines().next().unwrap_or("")
                ))
            }
        }
        "diag" | "module-diag" => {
            let diag_path = fixture.with_extension("diag");
            if !diag_path.exists() {
                return FixtureOutcome::Skip(format!("{} (no .diag file)", name));
            }
            let diag_text = std::fs::read_to_string(&diag_path).unwrap();
            let first_line = diag_text.lines().next().unwrap_or("").trim().to_string();

            let output = Command::new(bin)
                .arg("run")
                .arg(&fixture)
                .output()
                .expect("failed to run arukellt");

            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            if stderr.contains(first_line.as_str()) || stdout.contains(first_line.as_str()) {
                FixtureOutcome::Pass
            } else {
                FixtureOutcome::Fail(format!(
                    "FAIL [{}] {}\n  expected to contain: {:?}\n  stderr: {:?}",
                    entry.kind,
                    name,
                    first_line,
                    stderr.lines().next().unwrap_or("")
                ))
            }
        }
        "t3-compile" => {
            let output = Command::new(bin)
                .arg("compile")
                .arg("--target")
                .arg("wasm32-wasi-p2")
                .arg(&fixture)
                .arg("-o")
                .arg("/dev/null")
                .output()
                .expect("failed to run arukellt");

            if output.status.success() {
                FixtureOutcome::Pass
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                FixtureOutcome::Fail(format!(
                    "FAIL [t3-compile] {}\n  stderr: {:?}",
                    name,
                    stderr.lines().next().unwrap_or("")
                ))
            }
        }
        "component-compile" => {
            let output = Command::new(bin)
                .arg("compile")
                .arg("--target")
                .arg("wasm32-wasi-p2")
                .arg("--emit")
                .arg("component")
                .arg(&fixture)
                .arg("-o")
                .arg("/dev/null")
                .output()
                .expect("failed to run arukellt");

            if output.status.success() {
                FixtureOutcome::Pass
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("wasm-tools not found")
                    || stderr.contains("ToolNotFound")
                    || stderr.contains("failed to resolve import")
                {
                    FixtureOutcome::Skip(format!(
                        "{} (wasm-tools or WASI adapter not available)",
                        name
                    ))
                } else {
                    FixtureOutcome::Fail(format!(
                        "FAIL [component-compile] {}\n  stderr: {:?}",
                        name,
                        stderr.lines().next().unwrap_or("")
                    ))
                }
            }
        }
        "compile-error" => {
            let diag_path = fixture.with_extension("diag");
            if !diag_path.exists() {
                return FixtureOutcome::Skip(format!("{} (no .diag file)", name));
            }
            let diag_text = std::fs::read_to_string(&diag_path).unwrap();
            let first_line = diag_text.lines().next().unwrap_or("").trim().to_string();

            let flags_path = fixture.with_extension("flags");
            let extra_args: Vec<String> = if flags_path.exists() {
                std::fs::read_to_string(&flags_path)
                    .unwrap()
                    .split_whitespace()
                    .map(String::from)
                    .collect()
            } else {
                vec!["compile".into(), "--target".into(), "wasm32-wasi-p2".into()]
            };

            let output = Command::new(bin)
                .args(&extra_args)
                .arg(&fixture)
                .arg("-o")
                .arg("/dev/null")
                .output()
                .expect("failed to run arukellt");

            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            if !output.status.success()
                && (stderr.contains(first_line.as_str())
                    || stdout.contains(first_line.as_str()))
            {
                FixtureOutcome::Pass
            } else if output.status.success() {
                FixtureOutcome::Fail(format!(
                    "FAIL [compile-error] {} — expected compile failure but succeeded",
                    name
                ))
            } else {
                FixtureOutcome::Fail(format!(
                    "FAIL [compile-error] {}\n  expected to contain: {:?}\n  stderr: {:?}",
                    name,
                    first_line,
                    stderr.lines().next().unwrap_or("")
                ))
            }
        }
        other => FixtureOutcome::Skip(format!("{} (unknown kind {:?})", name, other)),
    }
}

#[test]
fn fixture_harness() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let fixture_dir = workspace_root.join("tests").join("fixtures");

    if !fixture_dir.exists() {
        eprintln!("No fixtures directory at {:?}, skipping", fixture_dir);
        return;
    }

    // --- Load manifest ---
    let manifest_path = fixture_dir.join("manifest.txt");
    let entries = load_manifest(&manifest_path);
    eprintln!(
        "Manifest: {} entries from {:?}",
        entries.len(),
        manifest_path
    );

    // --- Self-check: manifest completeness ---
    let disk_entries = entry_points(&fixture_dir);
    let manifest_paths: HashSet<String> = entries.iter().map(|e| e.path.clone()).collect();

    let missing_from_manifest: Vec<&String> = disk_entries
        .iter()
        .filter(|p| !manifest_paths.contains(*p))
        .collect();
    let missing_from_disk: Vec<&String> = manifest_paths
        .iter()
        .filter(|p| !fixture_dir.join(p).exists())
        .collect();

    if !missing_from_manifest.is_empty() {
        let mut sorted: Vec<_> = missing_from_manifest;
        sorted.sort();
        panic!(
            "Fixture files on disk but NOT in manifest.txt:\n  {}",
            sorted
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }
    if !missing_from_disk.is_empty() {
        let mut sorted: Vec<_> = missing_from_disk;
        sorted.sort();
        panic!(
            "Manifest entries whose files do NOT exist on disk:\n  {}",
            sorted
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }

    // --- Run fixtures in parallel (work-stealing) ---
    let bin = arukellt_binary();
    eprintln!("Using binary: {:?}", bin);

    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(entries.len().max(1));

    // Feed tasks via a channel; workers pull until the sender is closed.
    let (task_tx, task_rx) = mpsc::channel::<(usize, String, String)>();
    let task_rx = Arc::new(Mutex::new(task_rx));
    let (res_tx, res_rx) = mpsc::channel::<(usize, FixtureOutcome)>();

    for (i, entry) in entries.iter().enumerate() {
        task_tx
            .send((i, entry.kind.clone(), entry.path.clone()))
            .unwrap();
    }
    drop(task_tx);

    std::thread::scope(|scope| {
        for _ in 0..num_threads {
            let task_rx = Arc::clone(&task_rx);
            let res_tx = res_tx.clone();
            let bin = &bin;
            let fixture_dir = &fixture_dir;
            scope.spawn(move || loop {
                let task = task_rx.lock().unwrap().recv();
                match task {
                    Err(_) => break,
                    Ok((idx, kind, path)) => {
                        let entry = ManifestEntry { kind, path };
                        let outcome = run_fixture_entry(bin, fixture_dir, &entry);
                        res_tx.send((idx, outcome)).unwrap();
                    }
                }
            });
        }
        drop(res_tx);
    });

    // Collect and sort by original manifest index for deterministic output.
    let mut outcomes: Vec<(usize, FixtureOutcome)> = res_rx.into_iter().collect();
    outcomes.sort_by_key(|(i, _)| *i);

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (_, outcome) in outcomes {
        match outcome {
            FixtureOutcome::Pass => passed += 1,
            FixtureOutcome::Fail(msg) => {
                failed += 1;
                failures.push(msg);
            }
            FixtureOutcome::Skip(reason) => {
                skipped += 1;
                eprintln!("  [skip] {}", reason);
            }
        }
    }

    eprintln!("\n--- Fixture Results ---");
    eprintln!("PASS: {} FAIL: {} SKIP: {}", passed, failed, skipped);

    if !failures.is_empty() {
        for f in &failures {
            eprintln!("{}", f);
        }
        panic!("{} fixture(s) failed", failures.len());
    }
}
