"""Smoke tests for the Python-facing Rust embedding."""

from __future__ import annotations

import importlib
import sys
from pathlib import Path

import pytest
from truco_engine._native_loader import stage_native_module


@pytest.fixture(scope="session")
def staged_rust_python_extension() -> Path:
    return stage_native_module(build=True)


def test_native_module_imports_from_staged_artifact(
    staged_rust_python_extension: Path,
    monkeypatch: pytest.MonkeyPatch,
):
    sys.modules.pop("_truco_engine", None)
    monkeypatch.syspath_prepend(str(staged_rust_python_extension.parent))
    importlib.invalidate_caches()

    native_module = importlib.import_module("_truco_engine")

    assert hasattr(native_module, "ApiMatch")


def test_python_wrapper_can_drive_native_module(
    staged_rust_python_extension: Path,
    monkeypatch: pytest.MonkeyPatch,
):
    sys.modules.pop("_truco_engine", None)
    sys.modules.pop("truco_engine.rust_embed", None)
    monkeypatch.syspath_prepend(str(staged_rust_python_extension.parent))
    importlib.invalidate_caches()

    rust_embed = importlib.import_module("truco_engine.rust_embed")
    game = rust_embed.RustApiMatch(starting_dealer=1, score={"0": 0, "1": 0})

    snapshot = game.snapshot()
    assert snapshot["next_dealer"] == 1
    assert snapshot["hand_in_progress"] is False

    hand_snapshot = game.start_hand(
        turnup={"rank": "A", "suit": "SPADES"},
        hands={
            "0": [
                {"id": "p0c0", "rank": "7", "suit": "DIAMONDS"},
                {"id": "p0c1", "rank": "6", "suit": "CLUBS"},
                {"id": "p0c2", "rank": "4", "suit": "HEARTS"},
            ],
            "1": [
                {"id": "p1c0", "rank": "3", "suit": "CLUBS"},
                {"id": "p1c1", "rank": "5", "suit": "SPADES"},
                {"id": "p1c2", "rank": "4", "suit": "DIAMONDS"},
            ],
        },
    )
    assert hand_snapshot["current_player"] == 0

    legal_actions = game.legal_actions_for_current_player()
    assert any(action["type"] == "play_face_up" for action in legal_actions)
    assert any(action["type"] == "concede_hand" for action in legal_actions)

    result = game.apply_action_for_current_player(
        {"type": "play_face_up", "card_id": "p0c0"}
    )
    assert result["acted_by"] == 0
    assert result["snapshot"]["current_player"] == 1
