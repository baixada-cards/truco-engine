use serde_json::{json, Value};
use truco_engine::{Action, Engine, EngineError, GameState, Hands, Score, Turnup};

fn state_from_json(value: Value) -> GameState {
    serde_json::from_value(value).expect("test state should deserialize")
}

fn public_state_json(engine: &Engine) -> Value {
    engine
        .public_state_value()
        .expect("public state should serialize")
}

#[test]
fn pending_raise_limits_legal_actions() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 1,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "A", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "7", "suit": "CLUBS" },
                { "id": "p1c1", "rank": "5", "suit": "SPADES" },
                { "id": "p1c2", "rank": "4", "suit": "DIAMONDS" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": {
            "raised_by": 0,
            "to": 3,
            "previous_value": 1
        }
    }));

    let engine = Engine::from_state(state).expect("state should load");
    let actions = engine
        .legal_actions(1)
        .expect("legal actions should compute");

    assert!(actions.contains(&Action::AcceptRaise));
    assert!(actions.contains(&Action::Fold));
    assert!(actions.contains(&Action::ConcedeHand));
    assert!(actions.contains(&Action::Raise { to: 6 }));
    assert!(!actions.contains(&Action::PlayFaceUp {
        card_id: "p1c0".into()
    }));
}

#[test]
fn strategic_legal_actions_exclude_concede_hand() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 1,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "A", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "7", "suit": "CLUBS" },
                { "id": "p1c1", "rank": "5", "suit": "SPADES" },
                { "id": "p1c2", "rank": "4", "suit": "DIAMONDS" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": {
            "raised_by": 0,
            "to": 3,
            "previous_value": 1
        }
    }));

    let engine = Engine::from_state(state).expect("state should load");
    let legal_actions = engine
        .legal_actions(1)
        .expect("legal actions should compute");
    let strategic_actions = engine
        .strategic_legal_actions(1)
        .expect("strategic actions should compute");

    assert!(legal_actions.contains(&Action::ConcedeHand));
    assert!(!strategic_actions.contains(&Action::ConcedeHand));
    assert!(strategic_actions.contains(&Action::AcceptRaise));
    assert!(strategic_actions.contains(&Action::Fold));
    assert!(strategic_actions.contains(&Action::Raise { to: 6 }));
}

#[test]
fn hidden_card_is_omitted_from_public_state() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "7", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "K", "suit": "HEARTS" },
                { "id": "p0c1", "rank": "5", "suit": "CLUBS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "4", "suit": "DIAMONDS" },
                { "id": "p1c1", "rank": "6", "suit": "HEARTS" }
            ]
        },
        "completed_rounds": [
            { "leader": 0, "winner": 0 }
        ],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(
            0,
            &Action::PlayFaceDown {
                card_id: "p0c0".into(),
            },
        )
        .expect("hidden play should be allowed after the first round");

    let public_state = public_state_json(&engine);
    assert_eq!(
        public_state["current_round"]["plays"][0],
        json!({
            "player": 0,
            "visibility": "down"
        })
    );
}

#[test]
fn fold_awards_previous_hand_value_to_raiser() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "6", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "5", "suit": "DIAMONDS" },
                { "id": "p1c1", "rank": "4", "suit": "CLUBS" },
                { "id": "p1c2", "rank": "3", "suit": "SPADES" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(0, &Action::Raise { to: 3 })
        .expect("opening raise should be legal");
    engine
        .apply_action(1, &Action::Fold)
        .expect("fold should resolve the hand");

    let public_state = public_state_json(&engine);
    assert_eq!(public_state["hand_winner"], json!(0));
    assert_eq!(public_state["score"], json!({ "0": 1, "1": 0 }));
    assert_eq!(public_state["next_player"], Value::Null);
}

#[test]
fn concede_hand_awards_current_hand_value_to_opponent() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 3,
        "turnup": { "rank": "6", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "5", "suit": "DIAMONDS" },
                { "id": "p1c1", "rank": "4", "suit": "CLUBS" },
                { "id": "p1c2", "rank": "3", "suit": "SPADES" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "last_raised_by": 1,
        "pending_raise": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(0, &Action::ConcedeHand)
        .expect("conceding the hand should be legal");

    let public_state = public_state_json(&engine);
    assert_eq!(public_state["hand_winner"], json!(1));
    assert_eq!(public_state["score"], json!({ "0": 0, "1": 3 }));
    assert_eq!(public_state["next_player"], Value::Null);
}

#[test]
fn all_three_tied_rounds_award_hand_to_first_leader() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "4", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "3", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "7", "suit": "CLUBS" },
                { "id": "p1c1", "rank": "6", "suit": "SPADES" },
                { "id": "p1c2", "rank": "3", "suit": "DIAMONDS" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    for (player, action) in [
        (
            0,
            Action::PlayFaceUp {
                card_id: "p0c0".into(),
            },
        ),
        (
            1,
            Action::PlayFaceUp {
                card_id: "p1c0".into(),
            },
        ),
        (
            0,
            Action::PlayFaceUp {
                card_id: "p0c1".into(),
            },
        ),
        (
            1,
            Action::PlayFaceUp {
                card_id: "p1c1".into(),
            },
        ),
        (
            0,
            Action::PlayFaceUp {
                card_id: "p0c2".into(),
            },
        ),
        (
            1,
            Action::PlayFaceUp {
                card_id: "p1c2".into(),
            },
        ),
    ] {
        engine
            .apply_action(player, &action)
            .expect("play should succeed");
    }

    let public_state = public_state_json(&engine);
    assert_eq!(public_state["hand_winner"], json!(0));
    assert_eq!(public_state["score"], json!({ "0": 1, "1": 0 }));
    assert_eq!(
        public_state["completed_rounds"],
        json!([
            { "leader": 0, "winner": null },
            { "leader": 0, "winner": null },
            { "leader": 0, "winner": null }
        ])
    );
}

#[test]
fn accepts_eleven_without_changing_turn_order() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 11, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "6", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "5", "suit": "DIAMONDS" },
                { "id": "p1c1", "rank": "4", "suit": "CLUBS" },
                { "id": "p1c2", "rank": "3", "suit": "SPADES" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null,
        "pending_decision": {
            "type": "mao_de_onze",
            "player": 0
        }
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(0, &Action::AcceptEleven)
        .expect("accepting eleven should be legal");

    let public_state = public_state_json(&engine);
    assert_eq!(public_state["hand_value"], json!(3));
    assert_eq!(public_state["pending_decision"], Value::Null);
    assert_eq!(public_state["next_player"], json!(0));
}

#[test]
fn accepts_eleven_and_returns_play_to_the_round_leader() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 1,
        "score": { "0": 4, "1": 11 },
        "hand_value": 1,
        "turnup": { "rank": "6", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "5", "suit": "DIAMONDS" },
                { "id": "p1c1", "rank": "4", "suit": "CLUBS" },
                { "id": "p1c2", "rank": "3", "suit": "SPADES" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null,
        "pending_decision": {
            "type": "mao_de_onze",
            "player": 1
        }
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(1, &Action::AcceptEleven)
        .expect("accepting eleven should be legal");

    let public_state = public_state_json(&engine);
    assert_eq!(public_state["hand_value"], json!(3));
    assert_eq!(public_state["pending_decision"], Value::Null);
    assert_eq!(public_state["next_player"], json!(0));
}

#[test]
fn rejects_mao_de_onze_owned_by_the_non_eleven_player() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 4, "1": 11 },
        "hand_value": 1,
        "turnup": { "rank": "6", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "5", "suit": "DIAMONDS" },
                { "id": "p1c1", "rank": "4", "suit": "CLUBS" },
                { "id": "p1c2", "rank": "3", "suit": "SPADES" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null,
        "pending_decision": {
            "type": "mao_de_onze",
            "player": 0
        }
    }));

    let error =
        Engine::from_state(state).expect_err("non-eleven player should not own mao de onze");
    assert_eq!(error, EngineError::InvalidInitialState);
}

#[test]
fn accepted_raise_blocks_immediate_reraise_in_legal_actions() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "A", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "7", "suit": "CLUBS" },
                { "id": "p1c1", "rank": "5", "suit": "SPADES" },
                { "id": "p1c2", "rank": "4", "suit": "DIAMONDS" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null,
        "pending_decision": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(0, &Action::Raise { to: 3 })
        .expect("opening raise should be legal");
    engine
        .apply_action(1, &Action::AcceptRaise)
        .expect("accepting raise should be legal");

    let actions = engine.legal_actions(0).expect("actions should compute");
    assert!(!actions.contains(&Action::Raise { to: 6 }));
    assert!(actions.contains(&Action::PlayFaceUp {
        card_id: "p0c0".into()
    }));
}

#[test]
fn accepted_raise_rejects_immediate_reraise_attempt() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "A", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "7", "suit": "CLUBS" },
                { "id": "p1c1", "rank": "5", "suit": "SPADES" },
                { "id": "p1c2", "rank": "4", "suit": "DIAMONDS" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null,
        "pending_decision": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(0, &Action::Raise { to: 3 })
        .expect("opening raise should be legal");
    engine
        .apply_action(1, &Action::AcceptRaise)
        .expect("accepting raise should be legal");

    let error = engine
        .apply_action(0, &Action::Raise { to: 6 })
        .expect_err("same player should not re-raise immediately");
    assert_eq!(error, EngineError::RaiseNotAllowed);
}

#[test]
fn accepted_raise_keeps_card_play_legal_for_original_raiser() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "A", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "7", "suit": "CLUBS" },
                { "id": "p1c1", "rank": "5", "suit": "SPADES" },
                { "id": "p1c2", "rank": "4", "suit": "DIAMONDS" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null,
        "pending_decision": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(0, &Action::Raise { to: 3 })
        .expect("opening raise should be legal");
    engine
        .apply_action(1, &Action::AcceptRaise)
        .expect("accepting raise should be legal");
    engine
        .apply_action(
            0,
            &Action::PlayFaceUp {
                card_id: "p0c0".into(),
            },
        )
        .expect("raiser should still be able to play a card");

    let public_state = public_state_json(&engine);
    assert_eq!(public_state["hand_value"], json!(3));
    assert_eq!(
        public_state["current_round"]["plays"][0]["player"],
        json!(0)
    );
    assert_eq!(public_state["next_player"], json!(1));
}

#[test]
fn responder_can_still_reraise_while_raise_is_pending() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "A", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "3", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "7", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "K", "suit": "CLUBS" },
                { "id": "p1c1", "rank": "6", "suit": "SPADES" },
                { "id": "p1c2", "rank": "5", "suit": "DIAMONDS" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null,
        "pending_decision": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(0, &Action::Raise { to: 3 })
        .expect("opening raise should be legal");
    engine
        .apply_action(1, &Action::Raise { to: 6 })
        .expect("re-raise should remain legal for the responder");

    let public_state = public_state_json(&engine);
    assert_eq!(public_state["pending_raise"]["raised_by"], json!(1));
    assert_eq!(public_state["pending_raise"]["to"], json!(6));
    assert_eq!(public_state["next_player"], json!(0));
}

#[test]
fn opponent_can_raise_after_original_raiser_plays() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "A", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "7", "suit": "CLUBS" },
                { "id": "p1c1", "rank": "5", "suit": "SPADES" },
                { "id": "p1c2", "rank": "4", "suit": "DIAMONDS" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null,
        "pending_decision": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    engine
        .apply_action(0, &Action::Raise { to: 3 })
        .expect("opening raise should be legal");
    engine
        .apply_action(1, &Action::AcceptRaise)
        .expect("accepting raise should be legal");
    engine
        .apply_action(
            0,
            &Action::PlayFaceUp {
                card_id: "p0c0".into(),
            },
        )
        .expect("raiser should be able to play");

    let actions = engine.legal_actions(1).expect("actions should compute");
    assert!(actions.contains(&Action::Raise { to: 6 }));
}

#[test]
fn rejects_hidden_card_in_first_round() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 0, "1": 0 },
        "hand_value": 1,
        "turnup": { "rank": "A", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "3", "suit": "CLUBS" },
                { "id": "p1c1", "rank": "5", "suit": "SPADES" },
                { "id": "p1c2", "rank": "4", "suit": "DIAMONDS" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null
    }));

    let mut engine = Engine::from_state(state).expect("state should load");
    let error = engine
        .apply_action(
            0,
            &Action::PlayFaceDown {
                card_id: "p0c0".into(),
            },
        )
        .expect_err("first-round hidden play should be rejected");

    assert_eq!(error, EngineError::HideNotAllowedInFirstRound);
}

#[test]
fn can_start_a_fresh_hand_without_manual_state_plumbing() {
    let engine = Engine::new_hand(
        1,
        Score { zero: 0, one: 0 },
        Turnup {
            rank: truco_engine::Rank::Ace,
            suit: truco_engine::Suit::Spades,
        },
        Hands {
            zero: smallvec::smallvec![
                truco_engine::Card {
                    id: "p0c0".into(),
                    rank: truco_engine::Rank::Seven,
                    suit: truco_engine::Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p0c1".into(),
                    rank: truco_engine::Rank::Six,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p0c2".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Hearts,
                },
            ],
            one: smallvec::smallvec![
                truco_engine::Card {
                    id: "p1c0".into(),
                    rank: truco_engine::Rank::Three,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p1c1".into(),
                    rank: truco_engine::Rank::Five,
                    suit: truco_engine::Suit::Spades,
                },
                truco_engine::Card {
                    id: "p1c2".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Diamonds,
                },
            ],
        },
    )
    .expect("fresh hand should initialize");

    assert_eq!(engine.current_player(), Some(0));
    assert!(!engine.is_hand_over());
    assert!(!engine.is_match_over());
    assert_eq!(engine.state().current_round.leader, 0);
    assert!(engine.legal_actions_for_current_player().is_ok());
}

#[test]
fn fresh_hand_on_eleven_opens_with_pending_decision() {
    let engine = Engine::new_hand(
        1,
        Score { zero: 11, one: 0 },
        Turnup {
            rank: truco_engine::Rank::Six,
            suit: truco_engine::Suit::Spades,
        },
        Hands {
            zero: smallvec::smallvec![
                truco_engine::Card {
                    id: "p0c0".into(),
                    rank: truco_engine::Rank::Seven,
                    suit: truco_engine::Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p0c1".into(),
                    rank: truco_engine::Rank::Six,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p0c2".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Hearts,
                },
            ],
            one: smallvec::smallvec![
                truco_engine::Card {
                    id: "p1c0".into(),
                    rank: truco_engine::Rank::Five,
                    suit: truco_engine::Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p1c1".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p1c2".into(),
                    rank: truco_engine::Rank::Three,
                    suit: truco_engine::Suit::Spades,
                },
            ],
        },
    )
    .expect("fresh hand on eleven should initialize");

    let actions = engine
        .legal_actions_for_current_player()
        .expect("current player should have legal actions");
    assert_eq!(engine.current_player(), Some(0));
    assert!(actions.contains(&Action::AcceptEleven));
    assert!(actions.contains(&Action::FoldEleven));
    assert!(actions.contains(&Action::ConcedeHand));
}

#[test]
fn fresh_hand_on_eleven_assigns_the_decision_to_the_player_on_eleven() {
    let engine = Engine::new_hand(
        1,
        Score { zero: 4, one: 11 },
        Turnup {
            rank: truco_engine::Rank::Six,
            suit: truco_engine::Suit::Spades,
        },
        Hands {
            zero: smallvec::smallvec![
                truco_engine::Card {
                    id: "p0c0".into(),
                    rank: truco_engine::Rank::Seven,
                    suit: truco_engine::Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p0c1".into(),
                    rank: truco_engine::Rank::Six,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p0c2".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Hearts,
                },
            ],
            one: smallvec::smallvec![
                truco_engine::Card {
                    id: "p1c0".into(),
                    rank: truco_engine::Rank::Five,
                    suit: truco_engine::Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p1c1".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p1c2".into(),
                    rank: truco_engine::Rank::Three,
                    suit: truco_engine::Suit::Spades,
                },
            ],
        },
    )
    .expect("fresh hand on eleven should initialize");

    let actions = engine
        .legal_actions_for_current_player()
        .expect("the player on eleven should receive the decision");
    assert_eq!(engine.current_player(), Some(1));
    assert!(actions.contains(&Action::AcceptEleven));
    assert!(actions.contains(&Action::FoldEleven));
    assert!(actions.contains(&Action::ConcedeHand));
}

#[test]
fn fresh_hand_with_both_players_on_eleven_starts_playing_for_three() {
    let engine = Engine::new_hand(
        1,
        Score { zero: 11, one: 11 },
        Turnup {
            rank: truco_engine::Rank::Six,
            suit: truco_engine::Suit::Spades,
        },
        Hands {
            zero: smallvec::smallvec![
                truco_engine::Card {
                    id: "p0c0".into(),
                    rank: truco_engine::Rank::Seven,
                    suit: truco_engine::Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p0c1".into(),
                    rank: truco_engine::Rank::Six,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p0c2".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Hearts,
                },
            ],
            one: smallvec::smallvec![
                truco_engine::Card {
                    id: "p1c0".into(),
                    rank: truco_engine::Rank::Five,
                    suit: truco_engine::Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p1c1".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p1c2".into(),
                    rank: truco_engine::Rank::Three,
                    suit: truco_engine::Suit::Spades,
                },
            ],
        },
    )
    .expect("fresh hand with both players on eleven should initialize");

    let actions = engine
        .legal_actions_for_current_player()
        .expect("the round leader should be able to play immediately");
    assert_eq!(engine.current_player(), Some(0));
    assert_eq!(engine.state().hand_value, 3);
    assert!(engine.state().pending_decision.is_none());
    assert!(!actions.contains(&Action::AcceptEleven));
    assert!(!actions.contains(&Action::FoldEleven));
    assert!(actions.contains(&Action::ConcedeHand));
    assert!(actions.contains(&Action::PlayFaceUp {
        card_id: "p0c0".into()
    }));
}

#[test]
fn rejects_pending_mao_de_onze_when_both_players_are_on_eleven() {
    let state = state_from_json(json!({
        "dealer": 1,
        "next_player": 0,
        "score": { "0": 11, "1": 11 },
        "hand_value": 3,
        "turnup": { "rank": "6", "suit": "SPADES" },
        "hands": {
            "0": [
                { "id": "p0c0", "rank": "7", "suit": "DIAMONDS" },
                { "id": "p0c1", "rank": "6", "suit": "CLUBS" },
                { "id": "p0c2", "rank": "4", "suit": "HEARTS" }
            ],
            "1": [
                { "id": "p1c0", "rank": "5", "suit": "DIAMONDS" },
                { "id": "p1c1", "rank": "4", "suit": "CLUBS" },
                { "id": "p1c2", "rank": "3", "suit": "SPADES" }
            ]
        },
        "completed_rounds": [],
        "current_round": { "leader": 0, "plays": [] },
        "pending_raise": null,
        "pending_decision": {
            "type": "mao_de_onze",
            "player": 0
        }
    }));

    let error =
        Engine::from_state(state).expect_err("11-11 hands should not create a pending decision");
    assert_eq!(error, EngineError::InvalidInitialState);
}

#[test]
fn can_apply_action_for_current_player_without_repeating_turn_owner() {
    let mut engine = Engine::new_hand(
        1,
        Score { zero: 0, one: 0 },
        Turnup {
            rank: truco_engine::Rank::Ace,
            suit: truco_engine::Suit::Spades,
        },
        Hands {
            zero: smallvec::smallvec![
                truco_engine::Card {
                    id: "p0c0".into(),
                    rank: truco_engine::Rank::Seven,
                    suit: truco_engine::Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p0c1".into(),
                    rank: truco_engine::Rank::Six,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p0c2".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Hearts,
                },
            ],
            one: smallvec::smallvec![
                truco_engine::Card {
                    id: "p1c0".into(),
                    rank: truco_engine::Rank::Three,
                    suit: truco_engine::Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p1c1".into(),
                    rank: truco_engine::Rank::Five,
                    suit: truco_engine::Suit::Spades,
                },
                truco_engine::Card {
                    id: "p1c2".into(),
                    rank: truco_engine::Rank::Four,
                    suit: truco_engine::Suit::Diamonds,
                },
            ],
        },
    )
    .expect("fresh hand should initialize");

    let acted = engine
        .apply_action_for_current_player(&Action::PlayFaceUp {
            card_id: "p0c0".into(),
        })
        .expect("current player action should succeed");

    assert_eq!(acted, 0);
    assert_eq!(engine.current_player(), Some(1));
}
