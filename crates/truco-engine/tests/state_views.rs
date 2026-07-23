use serde_json::json;
use truco_engine::{
    Action, ApiMatch, Engine, Hands, Match, MatchConfig, Rank, Score, Suit, Turnup,
};

fn sample_turnup() -> Turnup {
    Turnup {
        rank: Rank::Ace,
        suit: Suit::Spades,
    }
}

fn sample_hands() -> Hands {
    Hands {
        zero: smallvec::smallvec![
            truco_engine::Card {
                id: "p0c0".into(),
                rank: Rank::Seven,
                suit: Suit::Diamonds,
            },
            truco_engine::Card {
                id: "p0c1".into(),
                rank: Rank::Six,
                suit: Suit::Clubs,
            },
            truco_engine::Card {
                id: "p0c2".into(),
                rank: Rank::Four,
                suit: Suit::Hearts,
            },
        ],
        one: smallvec::smallvec![
            truco_engine::Card {
                id: "p1c0".into(),
                rank: Rank::Three,
                suit: Suit::Clubs,
            },
            truco_engine::Card {
                id: "p1c1".into(),
                rank: Rank::Five,
                suit: Suit::Spades,
            },
            truco_engine::Card {
                id: "p1c2".into(),
                rank: Rank::Four,
                suit: Suit::Diamonds,
            },
        ],
    }
}

#[test]
fn engine_state_round_trips_after_hand_is_decided() {
    let mut engine = Engine::new_hand(
        1,
        Score { zero: 0, one: 0 },
        sample_turnup(),
        sample_hands(),
    )
    .expect("engine should initialize");
    engine
        .apply_action_for_current_player(&Action::Raise { to: 3 })
        .expect("raise should succeed");
    engine
        .apply_action_for_current_player(&Action::Fold)
        .expect("fold should end the hand");

    let exported = engine.export_state();
    let restored =
        Engine::from_exported_state(exported).expect("exported engine state should restore");

    assert!(restored.is_hand_over());
    assert_eq!(restored.hand_winner(), Some(0));
    assert_eq!(restored.match_winner(), None);
    assert_eq!(restored.state().score, Score { zero: 1, one: 0 });
}

#[test]
fn match_state_round_trips_through_api_boundary() {
    let mut game = ApiMatch::new(MatchConfig {
        starting_dealer: 1,
        score: Score { zero: 0, one: 0 },
    })
    .expect("api match should initialize");
    game.start_hand(truco_engine::HandStart {
        turnup: sample_turnup(),
        hands: sample_hands(),
    })
    .expect("hand should start");
    game.apply_action_for_current_player(&Action::PlayFaceUp {
        card_id: "p0c0".into(),
    })
    .expect("action should succeed");

    let exported = game.export_state();
    let restored = ApiMatch::from_state(exported).expect("match state should restore");

    assert_eq!(restored.public_view().current_player, Some(1));
    assert_eq!(
        restored
            .public_view()
            .hand
            .expect("hand should be present")
            .current_round
            .plays
            .len(),
        1
    );
}

#[test]
fn player_view_includes_private_hand_but_not_opponent_hand() {
    let mut game = Match::new(1, Score { zero: 0, one: 0 }).expect("match should initialize");
    game.start_hand(sample_turnup(), sample_hands())
        .expect("hand should start");
    game.apply_action_for_current_player(&Action::PlayFaceUp {
        card_id: "p0c0".into(),
    })
    .expect("first play should succeed");

    let player_one_view = game.player_view(1).expect("player view should succeed");

    assert_eq!(player_one_view.player, 1);
    assert_eq!(player_one_view.current_player, Some(1));
    assert_eq!(
        player_one_view
            .hand
            .as_ref()
            .expect("hand should be visible to the player")
            .hand
            .len(),
        3
    );
    let visible_card = player_one_view
        .hand
        .as_ref()
        .expect("hand should be visible to the player")
        .public_state
        .current_round
        .plays[0]
        .card
        .as_ref()
        .expect("face-up card should stay visible");
    assert_eq!(visible_card.rank, Rank::Seven);
    assert_eq!(visible_card.suit, Suit::Diamonds);
}

#[test]
fn api_match_player_view_serializes_cleanly() {
    let mut game = ApiMatch::new(MatchConfig {
        starting_dealer: 1,
        score: Score { zero: 0, one: 0 },
    })
    .expect("api match should initialize");
    game.start_hand(truco_engine::HandStart {
        turnup: sample_turnup(),
        hands: sample_hands(),
    })
    .expect("hand should start");

    let view = game
        .player_view_value(0)
        .expect("player view should serialize");

    assert_eq!(view["player"], json!(0));
    assert_eq!(view["score"], json!({ "0": 0, "1": 0 }));
    assert_eq!(view["hand"]["hand"].as_array().expect("hand list").len(), 3);
}

#[test]
fn completed_rounds_keep_private_plays_while_public_state_stays_summary_only() {
    let mut engine = Engine::new_hand(
        1,
        Score { zero: 0, one: 0 },
        Turnup {
            rank: Rank::Seven,
            suit: Suit::Spades,
        },
        Hands {
            zero: smallvec::smallvec![
                truco_engine::Card {
                    id: "p0c0".into(),
                    rank: Rank::King,
                    suit: Suit::Hearts,
                },
                truco_engine::Card {
                    id: "p0c1".into(),
                    rank: Rank::Four,
                    suit: Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p0c2".into(),
                    rank: Rank::Queen,
                    suit: Suit::Diamonds,
                },
            ],
            one: smallvec::smallvec![
                truco_engine::Card {
                    id: "p1c0".into(),
                    rank: Rank::Four,
                    suit: Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p1c1".into(),
                    rank: Rank::Five,
                    suit: Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p1c2".into(),
                    rank: Rank::Jack,
                    suit: Suit::Clubs,
                },
            ],
        },
    )
    .expect("engine should initialize");

    engine
        .apply_action_for_current_player(&Action::PlayFaceUp {
            card_id: "p0c0".into(),
        })
        .expect("first round play should succeed");
    engine
        .apply_action_for_current_player(&Action::PlayFaceUp {
            card_id: "p1c0".into(),
        })
        .expect("round should resolve");
    engine
        .apply_action_for_current_player(&Action::PlayFaceDown {
            card_id: "p0c1".into(),
        })
        .expect("second-round hidden play should succeed");
    engine
        .apply_action_for_current_player(&Action::PlayFaceUp {
            card_id: "p1c1".into(),
        })
        .expect("second round should resolve");

    let exported = engine.export_state();
    assert_eq!(exported.state.completed_rounds.len(), 2);
    assert_eq!(exported.state.completed_rounds[1].plays.len(), 2);
    assert_eq!(
        exported.state.completed_rounds[1].plays[0].visibility,
        truco_engine::state::Visibility::Down
    );
    assert_eq!(
        exported.state.completed_rounds[1].plays[0].card.rank,
        Rank::Four
    );
    assert_eq!(
        exported.state.completed_rounds[1].plays[0].card.suit,
        Suit::Clubs
    );

    let public_state = engine
        .public_state_value()
        .expect("public state should serialize");
    assert_eq!(
        public_state["completed_rounds"],
        json!([
            { "leader": 0, "winner": 0 },
            { "leader": 0, "winner": 1 }
        ])
    );
}

#[test]
fn exported_state_includes_last_raised_by_after_accepted_raise() {
    let mut engine = Engine::new_hand(
        1,
        Score { zero: 0, one: 0 },
        sample_turnup(),
        sample_hands(),
    )
    .expect("engine should initialize");

    engine
        .apply_action_for_current_player(&Action::Raise { to: 3 })
        .expect("raise should succeed");
    engine
        .apply_action_for_current_player(&Action::AcceptRaise)
        .expect("accept should succeed");

    let exported = engine.export_state();
    assert_eq!(exported.state.last_raised_by, Some(0));
}

#[test]
fn pending_raise_state_without_last_raised_by_normalizes_on_load() {
    let engine = Engine::from_state(
        serde_json::from_value(json!({
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
        }))
        .expect("state should deserialize"),
    )
    .expect("legacy pending-raise state should load");

    assert_eq!(engine.state().last_raised_by, Some(0));
}

#[test]
fn fresh_hand_state_without_last_raised_by_normalizes_to_none() {
    let engine = Engine::from_state(
        serde_json::from_value(json!({
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
            "pending_raise": null
        }))
        .expect("state should deserialize"),
    )
    .expect("legacy fresh state should load");

    assert_eq!(engine.state().last_raised_by, None);
}

#[test]
fn accepted_raise_state_without_last_raised_by_is_rejected_as_ambiguous() {
    let error = Engine::from_state(
        serde_json::from_value(json!({
            "dealer": 1,
            "next_player": 0,
            "score": { "0": 0, "1": 0 },
            "hand_value": 3,
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
        }))
        .expect("state should deserialize"),
    )
    .expect_err("accepted-raised legacy state should be rejected");

    assert_eq!(error, truco_engine::EngineError::InvalidInitialState);
}

#[test]
fn pending_reraise_state_round_trips_after_villain_raises_over_a_hero_play() {
    let mut engine = Engine::new_hand(
        1,
        Score { zero: 0, one: 0 },
        sample_turnup(),
        sample_hands(),
    )
    .expect("engine should initialize");

    engine
        .apply_action_for_current_player(&Action::PlayFaceUp {
            card_id: "p0c0".into(),
        })
        .expect("hero should open the round");
    engine
        .apply_action_for_current_player(&Action::Raise { to: 3 })
        .expect("villain raise should succeed");
    engine
        .apply_action_for_current_player(&Action::Raise { to: 6 })
        .expect("hero re-raise should succeed");

    let exported = engine.export_state();
    assert_eq!(exported.state.last_raised_by, Some(0));
    assert_eq!(
        exported
            .state
            .pending_raise
            .as_ref()
            .map(|raise| raise.raised_by),
        Some(0)
    );
    assert_eq!(
        exported.state.pending_raise.as_ref().map(|raise| raise.to),
        Some(6)
    );
    assert_eq!(
        exported
            .state
            .pending_raise
            .as_ref()
            .map(|raise| raise.previous_value),
        Some(3)
    );
    let restored =
        Engine::from_exported_state(exported).expect("pending re-raise state should restore");

    assert_eq!(restored.state().hand_value, 1);
    assert_eq!(restored.state().last_raised_by, Some(0));
    assert_eq!(
        restored
            .state()
            .pending_raise
            .as_ref()
            .map(|raise| raise.to),
        Some(6)
    );
    assert_eq!(
        restored
            .state()
            .pending_raise
            .as_ref()
            .map(|raise| raise.previous_value),
        Some(3)
    );
    assert_eq!(restored.state().current_round.plays.len(), 1);
    assert_eq!(restored.state().next_player, Some(1));
}

#[test]
fn accepted_reraise_state_round_trips_after_villain_raises_over_a_hero_play() {
    let mut engine = Engine::new_hand(
        1,
        Score { zero: 0, one: 0 },
        sample_turnup(),
        sample_hands(),
    )
    .expect("engine should initialize");

    engine
        .apply_action_for_current_player(&Action::PlayFaceUp {
            card_id: "p0c0".into(),
        })
        .expect("hero should open the round");
    engine
        .apply_action_for_current_player(&Action::Raise { to: 3 })
        .expect("villain raise should succeed");
    engine
        .apply_action_for_current_player(&Action::Raise { to: 6 })
        .expect("hero re-raise should succeed");
    engine
        .apply_action_for_current_player(&Action::AcceptRaise)
        .expect("villain should be able to accept the re-raise");

    let exported = engine.export_state();
    assert_eq!(exported.state.last_raised_by, Some(0));
    assert!(exported.state.pending_raise.is_none());
    let restored =
        Engine::from_exported_state(exported).expect("accepted re-raise state should restore");

    assert_eq!(restored.state().hand_value, 6);
    assert_eq!(restored.state().last_raised_by, Some(0));
    assert!(restored.state().pending_raise.is_none());
    assert_eq!(restored.state().current_round.plays.len(), 1);
    assert_eq!(restored.state().next_player, Some(1));
}

#[test]
fn accepted_reraise_state_round_trips_when_villain_raises_before_leading_round_two() {
    let mut game = Match::new(1, Score { zero: 3, one: 1 }).expect("match should initialize");
    game.start_hand(
        Turnup {
            rank: Rank::Jack,
            suit: Suit::Spades,
        },
        Hands {
            zero: smallvec::smallvec![
                truco_engine::Card {
                    id: "p0c0".into(),
                    rank: Rank::Five,
                    suit: Suit::Diamonds,
                },
                truco_engine::Card {
                    id: "p0c1".into(),
                    rank: Rank::Seven,
                    suit: Suit::Clubs,
                },
                truco_engine::Card {
                    id: "p0c2".into(),
                    rank: Rank::Three,
                    suit: Suit::Hearts,
                },
            ],
            one: smallvec::smallvec![
                truco_engine::Card {
                    id: "p1c0".into(),
                    rank: Rank::Six,
                    suit: Suit::Spades,
                },
                truco_engine::Card {
                    id: "p1c1".into(),
                    rank: Rank::Two,
                    suit: Suit::Spades,
                },
                truco_engine::Card {
                    id: "p1c2".into(),
                    rank: Rank::Three,
                    suit: Suit::Clubs,
                },
            ],
        },
    )
    .expect("hand should start");

    game.apply_action_for_current_player(&Action::PlayFaceUp {
        card_id: "p0c0".into(),
    })
    .expect("hero should open the first round");
    game.apply_action_for_current_player(&Action::PlayFaceUp {
        card_id: "p1c0".into(),
    })
    .expect("villain should win the first round");
    game.apply_action_for_current_player(&Action::Raise { to: 3 })
        .expect("villain should be able to raise before leading round two");
    game.apply_action_for_current_player(&Action::Raise { to: 6 })
        .expect("hero should be able to re-raise");
    game.apply_action_for_current_player(&Action::AcceptRaise)
        .expect("villain should be able to accept the re-raise");

    let exported = game.export_state();
    let restored =
        Match::from_state(exported).expect("accepted re-raise state should restore cleanly");
    let restored_hand = restored
        .current_hand()
        .expect("hand should still be active");

    assert_eq!(restored_hand.state().hand_value, 6);
    assert_eq!(restored_hand.state().current_round.leader, 1);
    assert_eq!(restored.current_player(), Some(1));
    assert!(restored_hand.state().pending_raise.is_none());
}

#[test]
fn accepted_raise_state_round_trips_after_split_rounds_reach_the_third_trick() {
    let restored = Match::from_state(
        serde_json::from_value(json!({
            "next_dealer": 1,
            "score": { "0": 0, "1": 0 },
            "winner": null,
            "current_hand": {
                "state": {
                    "dealer": 1,
                    "next_player": 0,
                    "score": { "0": 0, "1": 0 },
                    "hand_value": 3,
                    "turnup": { "rank": "5", "suit": "CLUBS" },
                    "hands": {
                        "0": [
                            { "id": "p0c1", "rank": "7", "suit": "CLUBS" }
                        ],
                        "1": [
                            { "id": "p1c2", "rank": "6", "suit": "SPADES" }
                        ]
                    },
                    "completed_rounds": [
                        {
                            "leader": 0,
                            "winner": 1,
                            "plays": [
                                {
                                    "player": 0,
                                    "visibility": "up",
                                    "card": { "id": "p0c0", "rank": "5", "suit": "HEARTS" }
                                },
                                {
                                    "player": 1,
                                    "visibility": "up",
                                    "card": { "id": "p1c0", "rank": "A", "suit": "SPADES" }
                                }
                            ]
                        },
                        {
                            "leader": 1,
                            "winner": 0,
                            "plays": [
                                {
                                    "player": 1,
                                    "visibility": "up",
                                    "card": { "id": "p1c1", "rank": "2", "suit": "SPADES" }
                                },
                                {
                                    "player": 0,
                                    "visibility": "up",
                                    "card": { "id": "p0c2", "rank": "3", "suit": "HEARTS" }
                                }
                            ]
                        }
                    ],
                    "current_round": {
                        "leader": 0,
                        "plays": []
                    },
                    "last_raised_by": 0,
                    "pending_raise": null,
                    "pending_decision": null
                },
                "hand_winner": null,
                "match_winner": null
            }
        }))
        .expect("state should deserialize"),
    )
    .expect("split-round raised state should restore");

    let exported = restored.export_state();
    let mut replayed = Match::from_state(exported).expect("round-tripped match should restore");
    replayed
        .apply_action_for_current_player(&Action::PlayFaceUp {
            card_id: "p0c1".into(),
        })
        .expect("hero should be able to lead the third round");
    replayed
        .apply_action_for_current_player(&Action::PlayFaceUp {
            card_id: "p1c2".into(),
        })
        .expect("villain should answer the third round");

    let finished = replayed.export_state();
    let restored_finished =
        Match::from_state(finished).expect("finished third-round raised hand should restore");

    assert_eq!(restored_finished.score(), &Score { zero: 0, one: 3 });
    assert_eq!(restored_finished.winner(), None);
    assert!(restored_finished
        .current_hand()
        .expect("hand")
        .is_hand_over());
    assert_eq!(
        restored_finished
            .current_hand()
            .expect("hand")
            .state()
            .current_round
            .leader,
        1
    );
}
