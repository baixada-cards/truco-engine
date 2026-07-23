"""Python wrapper for the Rust Truco engine embedding."""

from __future__ import annotations

import json
from typing import Any

from ._native_loader import load_api_match_class

_ApiMatch = load_api_match_class()


class RustApiMatch:
    """Small Python-friendly wrapper over the Rust serde-based embedding API."""

    def __init__(self, *, starting_dealer: int, score: dict[str, int]) -> None:
        self._inner = _ApiMatch(
            json.dumps(
                {
                    "starting_dealer": starting_dealer,
                    "score": score,
                }
            )
        )

    def snapshot(self) -> dict[str, Any]:
        return json.loads(self._inner.snapshot_json())

    def start_hand(
        self,
        *,
        turnup: dict[str, Any],
        hands: dict[str, list[dict[str, Any]]],
    ) -> dict[str, Any]:
        return json.loads(
            self._inner.start_hand_json(
                json.dumps(
                    {
                        "turnup": turnup,
                        "hands": hands,
                    }
                )
            )
        )

    def legal_actions_for_current_player(self) -> list[dict[str, Any]]:
        return json.loads(self._inner.legal_actions_for_current_player_json())

    def apply_action_for_current_player(
        self,
        action: dict[str, Any],
    ) -> dict[str, Any]:
        return json.loads(
            self._inner.apply_action_for_current_player_json(json.dumps(action))
        )
