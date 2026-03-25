//! Integration test harness: auto-discover and run `.ark` fixture tests.
//!
//! For each `.ark` file in `tests/fixtures/`:
//! - If a `.expected` sibling exists → compile + run, compare stdout
//! - If a `.diag` sibling exists → compile-fail, check first line of stderr

use std::path::{Path, PathBuf};
use std::process::Command;

fn discover_fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut fixtures = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                fixtures.extend(discover_fixtures(&path));
            } else if path.extension().is_some_and(|e| e == "ark") {
                fixtures.push(path);
            }
        }
    }
    fixtures.sort();
    fixtures
}

fn arukellt_binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    // Go up from the test binary to the target dir
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("arukellt");
    if !path.exists() {
        // Try release
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

    let bin = arukellt_binary();
    eprintln!("Using binary: {:?}", bin);

    let fixtures = discover_fixtures(&fixture_dir);
    eprintln!("Discovered {} fixture(s)", fixtures.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut failures = Vec::new();

    for fixture in &fixtures {
        let name = fixture
            .strip_prefix(&fixture_dir)
            .unwrap_or(fixture.as_path())
            .display()
            .to_string();
        let expected_path = fixture.with_extension("expected");
        let diag_path = fixture.with_extension("diag");

        if expected_path.exists() {
            let expected = std::fs::read_to_string(&expected_path).unwrap();
            let output = Command::new(&bin)
                .arg("run")
                .arg(fixture)
                .output()
                .expect("failed to run arukellt");

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.as_ref() == expected {
                passed += 1;
            } else {
                failed += 1;
                failures.push(format!(
                    "FAIL [run] {}\n  expected: {:?}\n  got:      {:?}",
                    name,
                    expected.lines().next().unwrap_or(""),
                    stdout.lines().next().unwrap_or("")
                ));
            }
        } else if diag_path.exists() {
            let diag_text = std::fs::read_to_string(&diag_path).unwrap();
            let first_line = diag_text.lines().next().unwrap_or("").trim();

            let output = Command::new(&bin)
                .arg("run")
                .arg(fixture)
                .output()
                .expect("failed to run arukellt");

            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            if stderr.contains(first_line) || stdout.contains(first_line) {
                passed += 1;
            } else {
                failed += 1;
                failures.push(format!(
                    "FAIL [diag] {}\n  expected to contain: {:?}\n  stderr: {:?}",
                    name,
                    first_line,
                    stderr.lines().next().unwrap_or("")
                ));
            }
        } else {
            skipped += 1;
            eprintln!("  [skip] {} (no .expected or .diag)", name);
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
