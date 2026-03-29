//! Integration test harness: manifest-driven `.ark` fixture tests.
//!
//! Reads `tests/fixtures/manifest.txt` for the fixture list.
//! For each entry:
//! - `run:path`        → compile + run, compare stdout against `.expected`
//! - `diag:path`       → compile-fail, check first line of `.diag` in output
//! - `module-run:path` → same as run, for multi-file modules
//! - `module-diag:path`→ same as diag, for multi-file modules
//! - `t3-run:path`     → compile + run on wasm32-wasi-p2, compare stdout against `.expected`
//!
//! Self-check: verifies every `.ark` entry point on disk is listed in the manifest.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A single manifest entry.
struct ManifestEntry {
    kind: String,
    path: String, // relative to tests/fixtures/
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
        // Skip helper files in module directories
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

    // --- Run fixtures ---
    let bin = arukellt_binary();
    eprintln!("Using binary: {:?}", bin);

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut failures = Vec::new();

    for entry in &entries {
        let fixture = fixture_dir.join(&entry.path);
        let name = &entry.path;

        match entry.kind.as_str() {
            "run" | "module-run" => {
                let expected_path = fixture.with_extension("expected");
                if !expected_path.exists() {
                    skipped += 1;
                    eprintln!("  [skip] {} (no .expected file)", name);
                    continue;
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

                let output = Command::new(&bin)
                    .arg("run")
                    .args(&extra_args)
                    .arg(&fixture)
                    .output()
                    .expect("failed to run arukellt");

                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.as_ref() == expected {
                    passed += 1;
                } else {
                    failed += 1;
                    failures.push(format!(
                        "FAIL [{}] {}\n  expected: {:?}\n  got:      {:?}",
                        entry.kind,
                        name,
                        expected.lines().next().unwrap_or(""),
                        stdout.lines().next().unwrap_or("")
                    ));
                }
            }
            "diag" | "module-diag" => {
                let diag_path = fixture.with_extension("diag");
                if !diag_path.exists() {
                    skipped += 1;
                    eprintln!("  [skip] {} (no .diag file)", name);
                    continue;
                }
                let diag_text = std::fs::read_to_string(&diag_path).unwrap();
                let first_line = diag_text.lines().next().unwrap_or("").trim();

                let output = Command::new(&bin)
                    .arg("run")
                    .arg(&fixture)
                    .output()
                    .expect("failed to run arukellt");

                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);

                if stderr.contains(first_line) || stdout.contains(first_line) {
                    passed += 1;
                } else {
                    failed += 1;
                    failures.push(format!(
                        "FAIL [{}] {}\n  expected to contain: {:?}\n  stderr: {:?}",
                        entry.kind,
                        name,
                        first_line,
                        stderr.lines().next().unwrap_or("")
                    ));
                }
            }
            "t3-run" => {
                let expected_path = fixture.with_extension("expected");
                if !expected_path.exists() {
                    skipped += 1;
                    eprintln!("  [skip] {} (no .expected file)", name);
                    continue;
                }
                let expected = std::fs::read_to_string(&expected_path).unwrap();

                let output = Command::new(&bin)
                    .arg("run")
                    .arg("--target")
                    .arg("wasm32-wasi-p2")
                    .arg(&fixture)
                    .output()
                    .expect("failed to run arukellt");

                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.as_ref() == expected {
                    passed += 1;
                } else {
                    failed += 1;
                    failures.push(format!(
                        "FAIL [{}] {}\n  expected: {:?}\n  got:      {:?}",
                        entry.kind,
                        name,
                        expected.lines().next().unwrap_or(""),
                        stdout.lines().next().unwrap_or("")
                    ));
                }
            }
            other => {
                skipped += 1;
                eprintln!("  [skip] {} (unknown kind {:?})", name, other);
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
