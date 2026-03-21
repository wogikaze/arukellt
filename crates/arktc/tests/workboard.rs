use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[derive(Debug, Deserialize, Clone)]
struct IndexEntry {
    id: String,
    title: String,
    status: String,
    priority: String,
    area: Vec<String>,
    depends_on: Vec<String>,
    #[serde(default)]
    blocked_on: Vec<String>,
    source: String,
    created_at: String,
    updated_at: String,
    file: String,
    summary: String,
}

#[derive(Debug, Deserialize, Clone)]
struct IssueMetadata {
    id: String,
    title: String,
    status: String,
    priority: String,
    area: Vec<String>,
    depends_on: Vec<String>,
    #[serde(default)]
    blocked_on: Vec<String>,
    source: String,
    created_at: String,
    updated_at: String,
}

fn extract_json_block(contents: &str) -> &str {
    let start = contents
        .find(
            "```json
",
        )
        .expect("missing opening json block")
        + "```json
"
        .len();
    let rest = &contents[start..];
    let end = rest
        .find(
            "
```",
        )
        .expect("missing closing code fence");
    &rest[..end]
}

fn load_index() -> Vec<IndexEntry> {
    let path = repo_root().join("issues/index.md");
    let contents = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(extract_json_block(&contents)).expect("parse issues/index.md json block")
}

fn load_issue_metadata(path: &Path) -> IssueMetadata {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(extract_json_block(&contents))
        .unwrap_or_else(|error| panic!("failed to parse metadata in {}: {error}", path.display()))
}

fn issue_paths(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn issues_queue_files_exist_and_docs_point_to_index() {
    let root = repo_root();
    let issues_dir = root.join("issues");
    let open_dir = issues_dir.join("open");
    let done_dir = issues_dir.join("done");
    let index_path = issues_dir.join("index.md");
    let template_path = issues_dir.join("templates/issue.md");

    assert!(index_path.exists(), "missing {}", index_path.display());
    assert!(open_dir.exists(), "missing {}", open_dir.display());
    assert!(done_dir.exists(), "missing {}", done_dir.display());
    assert!(
        template_path.exists(),
        "missing {}",
        template_path.display()
    );
    assert!(!issue_paths(&open_dir).is_empty(), "expected open issues");
    assert!(!issue_paths(&done_dir).is_empty(), "expected done issues");

    let agents = fs::read_to_string(root.join("AGENTS.md")).expect("read AGENTS.md");
    let readme = fs::read_to_string(root.join("README.md")).expect("read README.md");
    let workboard = fs::read_to_string(root.join("WORKBOARD.md")).expect("read WORKBOARD.md");

    assert!(
        agents.contains("issues/index.md"),
        "AGENTS.md should point contributors to issues/index.md"
    );
    assert!(
        readme.contains("issues/index.md"),
        "README.md should point readers to issues/index.md"
    );
    assert!(
        workboard.contains("issues/index.md"),
        "WORKBOARD.md should redirect to issues/index.md"
    );
}

#[test]
fn issue_index_and_files_are_consistent_and_semantically_valid() {
    let root = repo_root();
    let index = load_index();
    assert!(!index.is_empty(), "issue index should not be empty");

    let mut file_metadata = HashMap::new();
    for dir_name in ["open", "done"] {
        let dir = root.join("issues").join(dir_name);
        for path in issue_paths(&dir) {
            let metadata = load_issue_metadata(&path);
            let expected_name = format!("{}.md", metadata.id);
            assert_eq!(
                path.file_name().and_then(|name| name.to_str()),
                Some(expected_name.as_str()),
                "file name should match issue id for {}",
                path.display()
            );
            assert!(matches!(
                metadata.status.as_str(),
                "active" | "ready" | "blocked" | "done"
            ));
            assert!(matches!(
                metadata.priority.as_str(),
                "p0" | "p1" | "p2" | "p3"
            ));
            assert!(
                !metadata.title.is_empty(),
                "missing title in {}",
                path.display()
            );
            assert!(
                !metadata.area.is_empty(),
                "missing area in {}",
                path.display()
            );
            assert!(
                !metadata.source.is_empty(),
                "missing source in {}",
                path.display()
            );
            assert!(
                !metadata.created_at.is_empty(),
                "missing created_at in {}",
                path.display()
            );
            assert!(
                !metadata.updated_at.is_empty(),
                "missing updated_at in {}",
                path.display()
            );
            if dir_name == "open" {
                assert_ne!(
                    metadata.status,
                    "done",
                    "done issue should not live in open/: {}",
                    path.display()
                );
            } else {
                assert_eq!(
                    metadata.status,
                    "done",
                    "issues in done/ must have status done: {}",
                    path.display()
                );
            }

            let previous = file_metadata.insert(metadata.id.clone(), metadata.clone());
            assert!(
                previous.is_none(),
                "duplicate issue id {} across issue files",
                metadata.id
            );

            let contents = fs::read_to_string(&path).expect("read issue markdown");
            assert!(
                contents.contains("## Done When"),
                "missing Done When section in {}",
                path.display()
            );
            assert!(
                contents.contains("## Notes"),
                "missing Notes section in {}",
                path.display()
            );
        }
    }

    let mut index_ids = HashSet::new();
    let mut statuses = HashMap::new();
    for entry in &index {
        assert!(
            index_ids.insert(entry.id.clone()),
            "duplicate issue id {} in index",
            entry.id
        );
        assert!(matches!(
            entry.status.as_str(),
            "active" | "ready" | "blocked" | "done"
        ));
        assert!(matches!(entry.priority.as_str(), "p0" | "p1" | "p2" | "p3"));
        assert!(
            !entry.title.is_empty(),
            "missing title in index for {}",
            entry.id
        );
        assert!(
            !entry.area.is_empty(),
            "missing area in index for {}",
            entry.id
        );
        assert!(
            !entry.source.is_empty(),
            "missing source in index for {}",
            entry.id
        );
        assert!(
            !entry.created_at.is_empty(),
            "missing created_at in index for {}",
            entry.id
        );
        assert!(
            !entry.updated_at.is_empty(),
            "missing updated_at in index for {}",
            entry.id
        );
        assert!(
            !entry.summary.is_empty(),
            "missing summary in index for {}",
            entry.id
        );

        let metadata = file_metadata
            .get(&entry.id)
            .unwrap_or_else(|| panic!("index references missing issue file for {}", entry.id));
        assert_eq!(
            entry.title, metadata.title,
            "title mismatch for {}",
            entry.id
        );
        assert_eq!(
            entry.status, metadata.status,
            "status mismatch for {}",
            entry.id
        );
        assert_eq!(
            entry.priority, metadata.priority,
            "priority mismatch for {}",
            entry.id
        );
        assert_eq!(entry.area, metadata.area, "area mismatch for {}", entry.id);
        assert_eq!(
            entry.depends_on, metadata.depends_on,
            "depends_on mismatch for {}",
            entry.id
        );
        assert_eq!(
            entry.blocked_on, metadata.blocked_on,
            "blocked_on mismatch for {}",
            entry.id
        );
        assert_eq!(
            entry.source, metadata.source,
            "source mismatch for {}",
            entry.id
        );
        assert_eq!(
            entry.created_at, metadata.created_at,
            "created_at mismatch for {}",
            entry.id
        );
        assert_eq!(
            entry.updated_at, metadata.updated_at,
            "updated_at mismatch for {}",
            entry.id
        );

        let expected_file = if metadata.status == "done" {
            format!("issues/done/{}.md", entry.id)
        } else {
            format!("issues/open/{}.md", entry.id)
        };
        assert_eq!(
            entry.file, expected_file,
            "file path mismatch for {}",
            entry.id
        );
        statuses.insert(entry.id.clone(), entry.status.clone());
    }

    assert_eq!(
        index_ids.len(),
        file_metadata.len(),
        "index/file issue count mismatch"
    );

    for entry in &index {
        for dependency in &entry.depends_on {
            assert!(
                statuses.contains_key(dependency),
                "{} depends on missing {}",
                entry.id,
                dependency
            );
        }
    }

    let mut indegree: HashMap<String, usize> = index
        .iter()
        .map(|entry| (entry.id.clone(), entry.depends_on.len()))
        .collect();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for entry in &index {
        for dependency in &entry.depends_on {
            outgoing
                .entry(dependency.clone())
                .or_default()
                .push(entry.id.clone());
        }
    }

    let mut queue: VecDeque<String> = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = 0usize;
    while let Some(id) = queue.pop_front() {
        visited += 1;
        if let Some(children) = outgoing.get(&id) {
            for child in children {
                let count = indegree.get_mut(child).expect("child indegree");
                *count -= 1;
                if *count == 0 {
                    queue.push_back(child.clone());
                }
            }
        }
    }
    assert_eq!(
        visited,
        index.len(),
        "issue dependency graph must be acyclic"
    );

    for entry in &index {
        let unresolved: Vec<_> = entry
            .depends_on
            .iter()
            .filter(|dependency| {
                statuses
                    .get(*dependency)
                    .is_some_and(|status| status != "done")
            })
            .collect();
        if entry.status == "ready" {
            assert!(
                unresolved.is_empty(),
                "ready issue {} has unresolved dependencies",
                entry.id
            );
        }
        if entry.status == "active" {
            assert!(
                unresolved.is_empty(),
                "active issue {} has unresolved dependencies",
                entry.id
            );
        }
        if entry.status == "blocked" {
            assert!(
                !unresolved.is_empty()
                    || !entry.depends_on.is_empty()
                    || !entry.blocked_on.is_empty(),
                "blocked issue {} should explain its blocked state through dependencies",
                entry.id
            );
        }
    }

    let priority_rank = |priority: &str| match priority {
        "p0" => 0,
        "p1" => 1,
        "p2" => 2,
        "p3" => 3,
        other => panic!("unexpected priority {other}"),
    };
    let best_active_rank = index
        .iter()
        .filter(|entry| entry.status == "active")
        .map(|entry| priority_rank(&entry.priority))
        .min();
    let best_open_rank = index
        .iter()
        .filter(|entry| matches!(entry.status.as_str(), "active" | "ready"))
        .filter(|entry| {
            entry.depends_on.iter().all(|dependency| {
                statuses
                    .get(dependency)
                    .is_some_and(|status| status == "done")
            })
        })
        .map(|entry| priority_rank(&entry.priority))
        .min();
    if let (Some(active_rank), Some(open_rank)) = (best_active_rank, best_open_rank) {
        assert!(
            active_rank <= open_rank,
            "active set should include the highest-priority executable open work"
        );
    }

    assert!(
        file_metadata.contains_key("WB-051"),
        "missing historical WB-051 placeholder"
    );
    assert!(
        file_metadata.contains_key("WB-056"),
        "missing historical WB-056 placeholder"
    );
    assert!(
        file_metadata.contains_key("WB-062"),
        "missing normalized WB-062 issue"
    );
}
