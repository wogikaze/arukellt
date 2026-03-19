use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn workboard_exists_with_stable_queue_sections() {
    let path = repo_root().join("WORKBOARD.md");
    let workboard = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    for required in [
        "# WORKBOARD",
        "## Operating Rules",
        "## Task Schema",
        "## Next",
        "## Ready",
        "## Blocked",
        "## Done",
        "status:",
        "priority:",
        "owner:",
        "depends_on:",
        "done_when:",
        "notes:",
    ] {
        assert!(
            workboard.contains(required),
            "missing required marker `{required}` in {}",
            path.display()
        );
    }

    let next_count = workboard
        .lines()
        .filter(|line| line.trim() == "status: NEXT")
        .count();
    assert!(
        next_count <= 1,
        "expected at most one NEXT item, found {next_count}"
    );
}

#[test]
fn contributor_docs_point_to_the_shared_workboard() {
    let agents = fs::read_to_string(repo_root().join("AGENTS.md")).expect("read AGENTS.md");
    let readme = fs::read_to_string(repo_root().join("README.md")).expect("read README.md");

    assert!(
        agents.contains("WORKBOARD.md"),
        "AGENTS.md should tell contributors to use WORKBOARD.md"
    );
    assert!(
        readme.contains("WORKBOARD.md"),
        "README.md should point readers to WORKBOARD.md"
    );
}
