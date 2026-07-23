use std::{env, fs, path::PathBuf, process::ExitCode};

use truco_engine::{execute_fixture, EngineFixture, FixtureRunStatus};

const USAGE: &str = "Usage: fixture-runner <fixture-path>";
const EXIT_USAGE: u8 = 64;
const EXIT_DATAERR: u8 = 65;
const EXIT_FAILURE: u8 = 1;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn run() -> Result<u8, (u8, String)> {
    let mut args = env::args_os().skip(1);
    let first = args.next().ok_or((EXIT_USAGE, USAGE.to_string()))?;
    if args.next().is_some() {
        return Err((EXIT_USAGE, USAGE.to_string()));
    }

    let path = PathBuf::from(first);
    let raw = fs::read_to_string(&path).map_err(|err| {
        (
            EXIT_DATAERR,
            format!("failed to read {}: {err}", path.display()),
        )
    })?;
    let fixture: EngineFixture = serde_json::from_str(&raw).map_err(|err| {
        (
            EXIT_DATAERR,
            format!("failed to parse fixture {}: {err}", path.display()),
        )
    })?;

    let output = execute_fixture(&fixture);

    let json = serde_json::to_string_pretty(&output).map_err(|err| {
        (
            EXIT_DATAERR,
            format!("failed to serialize runner output: {err}"),
        )
    })?;
    println!("{json}");
    Ok(if output.status == FixtureRunStatus::Pass {
        0
    } else {
        EXIT_FAILURE
    })
}
