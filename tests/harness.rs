//! Test harness: auto-discover and run `.ark` fixture tests.
//!
//! For each `.ark` file in `tests/fixtures/`:
//! - If a `.expected` sibling exists → compile + run, compare stdout
//! - If a `.diag` sibling exists → compile-fail, check error codes

use std::path::{Path, PathBuf};

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

fn has_expected(ark_path: &Path) -> bool {
    ark_path.with_extension("expected").exists()
}

fn has_diag(ark_path: &Path) -> bool {
    ark_path.with_extension("diag").exists()
}

#[test]
fn discover_test_fixtures() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    if !fixture_dir.exists() {
        eprintln!("No fixtures directory found at {:?}, skipping", fixture_dir);
        return;
    }

    let fixtures = discover_fixtures(&fixture_dir);
    eprintln!("Discovered {} fixture(s)", fixtures.len());

    for fixture in &fixtures {
        let name = fixture.strip_prefix(&fixture_dir).unwrap().display();
        if has_expected(fixture) {
            eprintln!("  [run]  {}", name);
            // TODO: compile and run, compare stdout
        } else if has_diag(fixture) {
            eprintln!("  [diag] {}", name);
            // TODO: compile, expect failure, check diagnostic codes
        } else {
            eprintln!("  [skip] {} (no .expected or .diag)", name);
        }
    }

    // For now, just verify discovery works
    assert!(
        fixtures.len() > 0 || !fixture_dir.exists(),
        "Expected at least one fixture"
    );
}
