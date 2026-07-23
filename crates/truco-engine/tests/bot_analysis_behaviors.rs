use rand::{rngs::StdRng, SeedableRng};
use truco_engine::{
    bot_analysis::{analyze_tactical_turn, analyze_tactical_turn_sampled},
    Action, Card, Hands, Match, Rank, Score, Suit, Turnup,
};

fn sample_turnup() -> Turnup {
    Turnup {
        rank: Rank::Ace,
        suit: Suit::Spades,
    }
}

#[test]
fn tactical_analysis_counts_completed_hidden_opponent_information_from_players_view() {
    let mut game = Match::new(1, Score { zero: 0, one: 0 }).expect("match should initialize");
    game.start_hand(
        sample_turnup(),
        Hands {
            zero: smallvec::smallvec![
                Card {
                    id: "p0c0".into(),
                    rank: Rank::Four,
                    suit: Suit::Hearts,
                },
                Card {
                    id: "p0c1".into(),
                    rank: Rank::Seven,
                    suit: Suit::Diamonds,
                },
                Card {
                    id: "p0c2".into(),
                    rank: Rank::Six,
                    suit: Suit::Clubs,
                },
            ],
            one: smallvec::smallvec![
                Card {
                    id: "p1c0".into(),
                    rank: Rank::Four,
                    suit: Suit::Diamonds,
                },
                Card {
                    id: "p1c1".into(),
                    rank: Rank::Seven,
                    suit: Suit::Hearts,
                },
                Card {
                    id: "p1c2".into(),
                    rank: Rank::Five,
                    suit: Suit::Spades,
                },
            ],
        },
    )
    .expect("hand should start");

    for action in [
        Action::PlayFaceUp {
            card_id: "p0c0".into(),
        },
        Action::PlayFaceUp {
            card_id: "p1c0".into(),
        },
        Action::PlayFaceDown {
            card_id: "p0c1".into(),
        },
        Action::PlayFaceDown {
            card_id: "p1c1".into(),
        },
    ] {
        game.apply_action_for_current_player(&action)
            .expect("scripted action should apply");
    }

    let analysis = analyze_tactical_turn(&game, 0).expect("analysis should succeed");
    assert_eq!(analysis.action_summaries.len(), 3);
    assert!(analysis
        .action_summaries
        .iter()
        .all(|summary| summary.total_determinizations == 1190));
}

#[test]
fn tactical_analysis_counts_current_hidden_opponent_information_from_players_view() {
    let mut game = Match::new(1, Score { zero: 0, one: 0 }).expect("match should initialize");
    game.start_hand(
        sample_turnup(),
        Hands {
            zero: smallvec::smallvec![
                Card {
                    id: "p0c0".into(),
                    rank: Rank::Four,
                    suit: Suit::Hearts,
                },
                Card {
                    id: "p0c1".into(),
                    rank: Rank::Seven,
                    suit: Suit::Diamonds,
                },
                Card {
                    id: "p0c2".into(),
                    rank: Rank::Six,
                    suit: Suit::Clubs,
                },
            ],
            one: smallvec::smallvec![
                Card {
                    id: "p1c0".into(),
                    rank: Rank::Four,
                    suit: Suit::Diamonds,
                },
                Card {
                    id: "p1c1".into(),
                    rank: Rank::Seven,
                    suit: Suit::Hearts,
                },
                Card {
                    id: "p1c2".into(),
                    rank: Rank::Five,
                    suit: Suit::Spades,
                },
            ],
        },
    )
    .expect("hand should start");
    game.apply_action_for_current_player(&Action::PlayFaceUp {
        card_id: "p0c0".into(),
    })
    .expect("first play should apply");
    game.apply_action_for_current_player(&Action::PlayFaceUp {
        card_id: "p1c0".into(),
    })
    .expect("round should resolve");
    game.apply_action_for_current_player(&Action::PlayFaceDown {
        card_id: "p0c1".into(),
    })
    .expect("hidden play should apply");

    let analysis = analyze_tactical_turn(&game, 1).expect("analysis should succeed");
    assert!(analysis
        .action_summaries
        .iter()
        .all(|summary| summary.total_determinizations == 1190));
}

#[test]
fn sampled_tactical_analysis_uses_budget_for_wide_first_response_range() {
    let mut game = Match::new(1, Score { zero: 0, one: 0 }).expect("match should initialize");
    game.start_hand(
        sample_turnup(),
        Hands {
            zero: smallvec::smallvec![
                Card {
                    id: "p0c0".into(),
                    rank: Rank::Four,
                    suit: Suit::Hearts,
                },
                Card {
                    id: "p0c1".into(),
                    rank: Rank::Seven,
                    suit: Suit::Diamonds,
                },
                Card {
                    id: "p0c2".into(),
                    rank: Rank::Six,
                    suit: Suit::Clubs,
                },
            ],
            one: smallvec::smallvec![
                Card {
                    id: "p1c0".into(),
                    rank: Rank::Four,
                    suit: Suit::Diamonds,
                },
                Card {
                    id: "p1c1".into(),
                    rank: Rank::Seven,
                    suit: Suit::Hearts,
                },
                Card {
                    id: "p1c2".into(),
                    rank: Rank::Five,
                    suit: Suit::Spades,
                },
            ],
        },
    )
    .expect("hand should start");
    game.apply_action_for_current_player(&Action::PlayFaceUp {
        card_id: "p0c0".into(),
    })
    .expect("first play should apply");

    let mut rng = StdRng::seed_from_u64(42);
    let analysis =
        analyze_tactical_turn_sampled(&game, 1, 8, &mut rng).expect("analysis should succeed");

    assert_eq!(analysis.action_summaries.len(), 4);
    assert!(analysis
        .action_summaries
        .iter()
        .all(|summary| summary.total_determinizations == 8 && !summary.force_hand_win));
}
