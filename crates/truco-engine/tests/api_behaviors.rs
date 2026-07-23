use serde_json::json;
use truco_engine::{Action, ApiMatch, HandStart, MatchConfig, Rank, Score, Suit, Turnup};

fn sample_hand_start() -> HandStart {
    HandStart {
        turnup: Turnup {
            rank: Rank::Ace,
            suit: Suit::Spades,
        },
        hands: truco_engine::Hands {
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
        },
    }
}

#[test]
fn api_match_snapshot_has_stable_shape_before_hand_starts() {
    let game = ApiMatch::new(MatchConfig {
        starting_dealer: 1,
        score: Score { zero: 0, one: 0 },
    })
    .expect("api match should initialize");

    let snapshot = serde_json::to_value(game.public_view()).expect("snapshot should serialize");
    assert_eq!(
        snapshot,
        json!({
            "score": { "0": 0, "1": 0 },
            "winner": null,
            "next_dealer": 1,
            "current_player": null,
            "hand_in_progress": false,
            "hand": null
        })
    );
}

#[test]
fn api_match_can_start_hand_and_return_snapshot() {
    let mut game = ApiMatch::new(MatchConfig {
        starting_dealer: 1,
        score: Score { zero: 0, one: 0 },
    })
    .expect("api match should initialize");

    let snapshot = game
        .start_hand(sample_hand_start())
        .expect("hand should start");

    assert_eq!(snapshot.next_dealer, 1);
    assert_eq!(snapshot.current_player, Some(0));
    assert!(snapshot.hand_in_progress);
    assert_eq!(
        snapshot
            .hand
            .as_ref()
            .expect("hand state should be present")
            .current_round
            .leader,
        0
    );
}

#[test]
fn api_match_can_apply_action_and_return_json_result() {
    let mut game = ApiMatch::new(MatchConfig {
        starting_dealer: 1,
        score: Score { zero: 0, one: 0 },
    })
    .expect("api match should initialize");
    game.start_hand(sample_hand_start())
        .expect("hand should start");

    let result = game
        .apply_action_for_current_player_value(&Action::PlayFaceUp {
            card_id: "p0c0".into(),
        })
        .expect("action should succeed");

    assert_eq!(result["acted_by"], json!(0));
    assert_eq!(result["snapshot"]["current_player"], json!(1));
    assert_eq!(
        result["snapshot"]["hand"]["current_round"]["plays"][0]["card"],
        json!({
            "rank": "7",
            "suit": "DIAMONDS"
        })
    );
}

#[test]
fn api_match_legal_actions_include_concede_hand() {
    let mut game = ApiMatch::new(MatchConfig {
        starting_dealer: 1,
        score: Score { zero: 0, one: 0 },
    })
    .expect("api match should initialize");
    game.start_hand(sample_hand_start())
        .expect("hand should start");

    let legal_actions = game
        .legal_actions_for_current_player_value()
        .expect("legal actions should serialize");
    assert!(legal_actions
        .as_array()
        .expect("actions list")
        .iter()
        .any(|action| action["type"] == json!("concede_hand")));
}
