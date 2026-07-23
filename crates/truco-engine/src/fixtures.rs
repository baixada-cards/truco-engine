use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Action, GameState, Player};

/// Top-level fixture document shared across engine implementations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineFixture {
    pub fixture_version: String,
    pub id: String,
    pub ruleset: String,
    pub description: String,
    pub initial_state: GameState,
    #[serde(default)]
    pub steps: Vec<FixtureStep>,
    #[serde(default)]
    pub expect_initial_state_error: Option<String>,
}

/// Supported fixture steps. This intentionally mirrors the JSON contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum FixtureStep {
    #[serde(rename = "assert_legal_actions")]
    AssertLegalActions {
        player: Player,
        #[serde(default)]
        must_include: Vec<Action>,
        #[serde(default)]
        must_exclude: Vec<Action>,
    },
    #[serde(rename = "apply_action")]
    ApplyAction { player: Player, action: Action },
    #[serde(rename = "assert_state")]
    AssertState { expect: Value },
    #[serde(rename = "assert_export_round_trip")]
    AssertExportRoundTrip { expect: Value },
    #[serde(rename = "assert_rejected_action")]
    AssertRejectedAction {
        player: Player,
        action: Action,
        error_code: String,
    },
}
