use std::{fs, path::PathBuf};

use serde::Deserialize;
use serde_json::Value;
use truco_engine::{assert_expected_subset, resolve_match_spec, ExplorationMatchSpec};

mod support;

#[derive(Debug, Deserialize)]
struct ExplorationFixture {
    id: String,
    ruleset: String,
    spec: ExplorationMatchSpec,
    #[serde(default)]
    expect_resolved_state: Option<Value>,
    #[serde(default)]
    expect_error: Option<String>,
}

fn fixture_paths() -> Vec<PathBuf> {
    let fixture_root = support::spec_root().join("exploration").join("fixtures");
    let mut paths = Vec::new();
    for aspect_dir in fs::read_dir(&fixture_root).expect("fixture root should read") {
        let aspect_dir = aspect_dir.expect("aspect entry should read");
        if !aspect_dir
            .file_type()
            .expect("aspect type should read")
            .is_dir()
        {
            continue;
        }
        for entry in fs::read_dir(aspect_dir.path()).expect("aspect dir should read") {
            let entry = entry.expect("fixture entry should read");
            if entry.path().extension().and_then(|value| value.to_str()) == Some("json") {
                paths.push(entry.path());
            }
        }
    }
    paths.sort();
    paths
}

#[test]
fn exploration_fixture_corpus_passes_in_process() {
    for path in fixture_paths() {
        let raw = fs::read_to_string(&path).expect("fixture should read");
        let fixture: ExplorationFixture = serde_json::from_str(&raw).expect("fixture should parse");
        assert_eq!(fixture.ruleset, "truco-2p-v1", "{}", fixture.id);

        match (
            fixture.expect_resolved_state.as_ref(),
            fixture.expect_error.as_ref(),
        ) {
            (Some(expected), None) => {
                let resolved = resolve_match_spec(&fixture.spec)
                    .unwrap_or_else(|error| panic!("{}: unexpected error: {error}", fixture.id));
                let actual = serde_json::to_value(resolved).expect("state should serialize");
                assert_expected_subset(&actual, expected).unwrap_or_else(|message| {
                    panic!("{}: resolved state mismatch: {message}", fixture.id)
                });
            }
            (None, Some(expected_code)) => {
                let error = resolve_match_spec(&fixture.spec)
                    .expect_err("fixture should have produced an error");
                assert_eq!(error.code(), expected_code, "{}", fixture.id);
            }
            _ => panic!("{}: fixture must declare exactly one outcome", fixture.id),
        }
    }
}
