use serde::Serialize;
use serde_json::Value;

use crate::{assert_expected_subset, validate_state, Engine, EngineFixture, FixtureStep};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureRunReport {
    pub fixture_id: String,
    pub status: FixtureRunStatus,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureRunStatus {
    Pass,
    Fail,
}

pub fn execute_fixture(fixture: &EngineFixture) -> FixtureRunReport {
    match execute_fixture_inner(fixture) {
        Ok(message) => FixtureRunReport {
            fixture_id: fixture.id.clone(),
            status: FixtureRunStatus::Pass,
            message,
        },
        Err(message) => FixtureRunReport {
            fixture_id: fixture.id.clone(),
            status: FixtureRunStatus::Fail,
            message,
        },
    }
}

fn execute_fixture_inner(fixture: &EngineFixture) -> Result<String, String> {
    if let Some(expected_error) = &fixture.expect_initial_state_error {
        match validate_state(&fixture.initial_state) {
            Ok(()) => Err(format!(
                "expected initial state error {expected_error}, but state was accepted"
            )),
            Err(error) if error.code() == expected_error => {
                Ok(format!("initial state rejected with {expected_error}"))
            }
            Err(error) => Err(format!(
                "expected initial state error {expected_error}, got {}",
                error.code()
            )),
        }
    } else {
        let mut engine = Engine::from_state(fixture.initial_state.clone())
            .map_err(|error| format!("failed to load initial state: {}", error.code()))?;
        for (index, step) in fixture.steps.iter().enumerate() {
            execute_step(&mut engine, step)
                .map_err(|message| format!("step {} failed: {message}", index + 1))?;
        }
        Ok(format!("executed {} step(s)", fixture.steps.len()))
    }
}

fn execute_step(engine: &mut Engine, step: &FixtureStep) -> Result<(), String> {
    match step {
        FixtureStep::AssertLegalActions {
            player,
            must_include,
            must_exclude,
        } => {
            let actions = engine
                .legal_actions(*player)
                .map_err(|error| format!("legal_actions returned {}", error.code()))?;

            for action in must_include {
                if !actions.contains(action) {
                    return Err(format!(
                        "missing expected legal action {}",
                        action_json(action)
                    ));
                }
            }
            for action in must_exclude {
                if actions.contains(action) {
                    return Err(format!(
                        "unexpectedly allowed action {}",
                        action_json(action)
                    ));
                }
            }
            Ok(())
        }
        FixtureStep::ApplyAction { player, action } => engine
            .apply_action(*player, action)
            .map_err(|error| format!("apply_action returned {}", error.code())),
        FixtureStep::AssertState { expect } => {
            let actual = engine
                .public_state_value()
                .map_err(|error| format!("failed to serialize state: {}", error.code()))?;
            assert_expected_subset(&actual, expect).map_err(|message| {
                format!(
                    "state mismatch: {message}\nexpected subset: {}\nactual: {}",
                    pretty_json(expect),
                    pretty_json(&actual)
                )
            })
        }
        FixtureStep::AssertExportRoundTrip { expect } => {
            let before = engine.public_state_value().map_err(|error| {
                format!(
                    "failed to serialize state before round-trip: {}",
                    error.code()
                )
            })?;
            let exported = engine.export_state();
            let restored = Engine::from_exported_state(exported)
                .map_err(|error| format!("failed to restore exported state: {}", error.code()))?;
            let after = restored.public_state_value().map_err(|error| {
                format!(
                    "failed to serialize state after round-trip: {}",
                    error.code()
                )
            })?;

            if before != after {
                return Err(format!(
                    "public state changed after round-trip\nbefore: {}\nafter: {}",
                    pretty_json(&before),
                    pretty_json(&after)
                ));
            }

            assert_expected_subset(&after, expect).map_err(|message| {
                format!(
                    "round-trip state mismatch: {message}\nexpected subset: {}\nactual: {}",
                    pretty_json(expect),
                    pretty_json(&after)
                )
            })?;

            *engine = restored;
            Ok(())
        }
        FixtureStep::AssertRejectedAction {
            player,
            action,
            error_code,
        } => match engine.apply_action(*player, action) {
            Ok(()) => Err(format!(
                "expected rejected action {}, but it succeeded",
                action_json(action)
            )),
            Err(error) if error.code() == error_code => Ok(()),
            Err(error) => Err(format!(
                "expected rejected action {error_code}, got {}",
                error.code()
            )),
        },
    }
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn action_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<unserializable-action>".to_string())
}
