use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use truco_engine::{Action, Engine, EngineError, EngineState, Player};

#[derive(Debug, Deserialize)]
struct OracleRequest {
    id: String,
    op: OracleOp,
    state: EngineState,
    #[serde(default)]
    player: Option<Player>,
    #[serde(default)]
    action: Option<Action>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OracleOp {
    Eval,
    Apply,
    Expand,
}

#[derive(Debug, Serialize)]
struct OracleResponse {
    id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line in stdin.lock().lines() {
        let response = match line {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => handle_line(&line),
            Err(error) => OracleResponse {
                id: "<stdin>".into(),
                ok: false,
                value: None,
                error_code: None,
                message: Some(format!("failed to read request: {error}")),
            },
        };

        let encoded = serde_json::to_string(&response).unwrap_or_else(|error| {
            format!(
                r#"{{"id":"{}","ok":false,"message":"failed to serialize response: {}"}}"#,
                response.id, error
            )
        });
        if writeln!(stdout, "{encoded}").is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }
    }
}

fn handle_line(line: &str) -> OracleResponse {
    match serde_json::from_str::<OracleRequest>(line) {
        Ok(request) => handle_request(request),
        Err(error) => OracleResponse {
            id: "<parse>".into(),
            ok: false,
            value: None,
            error_code: None,
            message: Some(format!("failed to parse request: {error}")),
        },
    }
}

fn handle_request(request: OracleRequest) -> OracleResponse {
    let result = match request.op {
        OracleOp::Eval => eval_state(request.state),
        OracleOp::Apply => apply_action(request.state, request.player, request.action),
        OracleOp::Expand => expand_state(request.state),
    };

    match result {
        Ok(value) => OracleResponse {
            id: request.id,
            ok: true,
            value: Some(value),
            error_code: None,
            message: None,
        },
        Err(error) => OracleResponse {
            id: request.id,
            ok: false,
            value: None,
            error_code: Some(error.code()),
            message: Some(error.to_string()),
        },
    }
}

fn expand_state(state: EngineState) -> Result<Value, EngineError> {
    let engine = Engine::from_exported_state(state)?;
    let root = engine_value(&engine)?;
    let mut children = Vec::new();

    if let Some(player) = engine.current_player() {
        for action in engine.legal_actions(player)? {
            let mut child = engine.clone();
            child.apply_action(player, &action)?;
            children.push(json!({
                "action": action,
                "value": engine_value(&child)?,
            }));
        }
    }

    Ok(json!({
        "root": root,
        "children": children,
    }))
}

fn eval_state(state: EngineState) -> Result<Value, EngineError> {
    let engine = Engine::from_exported_state(state)?;
    engine_value(&engine)
}

fn apply_action(
    state: EngineState,
    player: Option<Player>,
    action: Option<Action>,
) -> Result<Value, EngineError> {
    let mut engine = Engine::from_exported_state(state)?;
    let player = player.ok_or(EngineError::InvalidInitialState)?;
    let action = action.ok_or(EngineError::InvalidInitialState)?;
    engine.apply_action(player, &action)?;
    engine_value(&engine)
}

fn engine_value(engine: &Engine) -> Result<Value, EngineError> {
    let legal_actions = match engine.current_player() {
        Some(player) => engine.legal_actions(player)?,
        None => Vec::new(),
    };

    Ok(json!({
        "export_state": engine.export_state(),
        "public_state": engine.public_state(),
        "current_player": engine.current_player(),
        "hand_winner": engine.hand_winner(),
        "match_winner": engine.match_winner(),
        "legal_actions": legal_actions,
    }))
}
