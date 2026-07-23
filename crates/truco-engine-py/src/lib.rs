#![allow(clippy::useless_conversion)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use truco_engine::{Action, ApiMatch, EngineError, HandStart, MatchConfig};

#[pyclass(name = "ApiMatch")]
pub struct PyApiMatch {
    inner: ApiMatch,
}

impl PyApiMatch {
    fn from_config_json(config_json: &str) -> PyResult<Self> {
        let config: MatchConfig = parse_json(config_json, "match config")?;
        let inner = ApiMatch::new(config).map_err(engine_error)?;
        Ok(Self { inner })
    }

    fn snapshot_json_impl(&self) -> PyResult<String> {
        serialize_json(&self.inner.public_view(), "snapshot")
    }

    fn start_hand_json_impl(&mut self, hand_json: &str) -> PyResult<String> {
        let hand: HandStart = parse_json(hand_json, "hand start")?;
        let snapshot = self.inner.start_hand(hand).map_err(engine_error)?;
        serialize_json(&snapshot, "hand snapshot")
    }

    fn legal_actions_for_current_player_json_impl(&self) -> PyResult<String> {
        let actions = self
            .inner
            .legal_actions_for_current_player()
            .map_err(engine_error)?;
        serialize_json(&actions, "legal actions")
    }

    fn apply_action_for_current_player_json_impl(&mut self, action_json: &str) -> PyResult<String> {
        let action: Action = parse_json(action_json, "action")?;
        let result = self
            .inner
            .apply_action_for_current_player(&action)
            .map_err(engine_error)?;
        serialize_json(&result, "action result")
    }
}

#[pymethods]
impl PyApiMatch {
    #[new]
    fn new(config_json: &str) -> PyResult<Self> {
        Self::from_config_json(config_json)
    }

    fn snapshot_json(&self) -> PyResult<String> {
        self.snapshot_json_impl()
    }

    fn start_hand_json(&mut self, hand_json: &str) -> PyResult<String> {
        self.start_hand_json_impl(hand_json)
    }

    fn legal_actions_for_current_player_json(&self) -> PyResult<String> {
        self.legal_actions_for_current_player_json_impl()
    }

    fn apply_action_for_current_player_json(&mut self, action_json: &str) -> PyResult<String> {
        self.apply_action_for_current_player_json_impl(action_json)
    }
}

#[pymodule]
fn _truco_engine(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyApiMatch>()?;
    Ok(())
}

fn parse_json<T: DeserializeOwned>(raw: &str, label: &str) -> PyResult<T> {
    serde_json::from_str(raw)
        .map_err(|error| PyValueError::new_err(format!("invalid {label} json: {error}")))
}

fn serialize_json<T: Serialize>(value: &T, label: &str) -> PyResult<String> {
    serde_json::to_string(value)
        .map_err(|error| PyValueError::new_err(format!("failed to serialize {label}: {error}")))
}

fn engine_error(error: EngineError) -> PyErr {
    PyValueError::new_err(format!("{}: {error}", error.code()))
}

#[cfg(test)]
mod tests {
    use super::PyApiMatch;

    #[test]
    fn py_api_match_round_trips_json() {
        let mut game =
            PyApiMatch::from_config_json(r#"{"starting_dealer":1,"score":{"0":0,"1":0}}"#)
                .expect("py api match should initialize");

        let snapshot = game
            .snapshot_json_impl()
            .expect("snapshot should serialize");
        assert_eq!(
            snapshot,
            r#"{"score":{"0":0,"1":0},"winner":null,"next_dealer":1,"current_player":null,"hand_in_progress":false,"hand":null}"#
        );

        let hand_snapshot = game
            .start_hand_json_impl(
                r#"{
                    "turnup":{"rank":"A","suit":"SPADES"},
                    "hands":{
                        "0":[
                            {"id":"p0c0","rank":"7","suit":"DIAMONDS"},
                            {"id":"p0c1","rank":"6","suit":"CLUBS"},
                            {"id":"p0c2","rank":"4","suit":"HEARTS"}
                        ],
                        "1":[
                            {"id":"p1c0","rank":"3","suit":"CLUBS"},
                            {"id":"p1c1","rank":"5","suit":"SPADES"},
                            {"id":"p1c2","rank":"4","suit":"DIAMONDS"}
                        ]
                    }
                }"#,
            )
            .expect("hand should start");
        assert!(hand_snapshot.contains(r#""current_player":0"#));

        let actions = game
            .legal_actions_for_current_player_json_impl()
            .expect("legal actions should serialize");
        assert!(actions.contains(r#""type":"play_face_up""#));

        let result = game
            .apply_action_for_current_player_json_impl(
                r#"{"type":"play_face_up","card_id":"p0c0"}"#,
            )
            .expect("action should succeed");
        assert!(result.contains(r#""acted_by":0"#));
        assert!(result.contains(r#""current_player":1"#));
    }
}
