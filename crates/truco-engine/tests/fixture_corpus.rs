use std::fs;
use std::path::{Path, PathBuf};

use truco_engine::{execute_fixture, EngineFixture, FixtureRunStatus};

mod support;

#[test]
fn json_fixture_corpus_passes_in_process() {
    let fixture_paths = collect_fixture_paths();
    assert_eq!(fixture_paths.len(), 67, "unexpected fixture corpus size");

    let mut failures = Vec::new();
    for path in fixture_paths {
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let fixture: EngineFixture = serde_json::from_str(&raw)
            .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));
        let report = execute_fixture(&fixture);
        if report.status != FixtureRunStatus::Pass {
            failures.push(format!(
                "{}: {}",
                path.strip_prefix(support::spec_root())
                    .unwrap_or(path.as_path())
                    .display(),
                report.message
            ));
        }
    }

    if !failures.is_empty() {
        panic!("fixture corpus failures:\n{}", failures.join("\n\n"));
    }
}

fn collect_fixture_paths() -> Vec<PathBuf> {
    let fixtures_dir = support::spec_root().join("engine").join("fixtures");
    let mut paths = Vec::new();
    collect_json_files(&fixtures_dir, &mut paths);
    paths.sort();
    paths
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read directory {}: {error}", dir.display()))
    {
        let entry = entry.unwrap_or_else(|error| panic!("failed to read directory entry: {error}"));
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            out.push(path);
        }
    }
}
