use truco_engine::{Action, Card, EngineError, Hands, Match, Rank, Score, Suit, Turnup};

fn sample_turnup() -> Turnup {
    Turnup {
        rank: Rank::Ace,
        suit: Suit::Spades,
    }
}

fn sample_hands() -> Hands {
    Hands {
        zero: smallvec::smallvec![
            Card {
                id: "p0c0".into(),
                rank: Rank::Seven,
                suit: Suit::Diamonds,
            },
            Card {
                id: "p0c1".into(),
                rank: Rank::Six,
                suit: Suit::Clubs,
            },
            Card {
                id: "p0c2".into(),
                rank: Rank::Four,
                suit: Suit::Hearts,
            },
        ],
        one: smallvec::smallvec![
            Card {
                id: "p1c0".into(),
                rank: Rank::Three,
                suit: Suit::Clubs,
            },
            Card {
                id: "p1c1".into(),
                rank: Rank::Five,
                suit: Suit::Spades,
            },
            Card {
                id: "p1c2".into(),
                rank: Rank::Four,
                suit: Suit::Diamonds,
            },
        ],
    }
}

#[test]
fn match_can_start_first_hand_and_delegate_turn_queries() {
    let mut game = Match::new(1, Score { zero: 0, one: 0 }).expect("match should initialize");

    let hand = game
        .start_hand(sample_turnup(), sample_hands())
        .expect("first hand should start");

    assert_eq!(hand.current_player(), Some(0));
    assert_eq!(game.current_player(), Some(0));
    let legal_actions = game
        .legal_actions_for_current_player()
        .expect("legal actions should compute");
    let strategic_actions = game
        .strategic_legal_actions_for_current_player()
        .expect("strategic actions should compute");

    assert!(legal_actions.contains(&Action::ConcedeHand));
    assert!(!strategic_actions.contains(&Action::ConcedeHand));
}

#[test]
fn match_cannot_start_new_hand_while_current_one_is_active() {
    let mut game = Match::new(1, Score { zero: 0, one: 0 }).expect("match should initialize");
    game.start_hand(sample_turnup(), sample_hands())
        .expect("first hand should start");

    let error = game
        .start_hand(sample_turnup(), sample_hands())
        .expect_err("second hand should be blocked");

    assert_eq!(error, EngineError::HandStillInProgress);
}

#[test]
fn finishing_a_hand_updates_score_and_rotates_dealer() {
    let mut game = Match::new(1, Score { zero: 0, one: 0 }).expect("match should initialize");
    game.start_hand(
        Turnup {
            rank: Rank::Six,
            suit: Suit::Spades,
        },
        Hands {
            zero: smallvec::smallvec![
                Card {
                    id: "p0c0".into(),
                    rank: Rank::Seven,
                    suit: Suit::Diamonds,
                },
                Card {
                    id: "p0c1".into(),
                    rank: Rank::Six,
                    suit: Suit::Clubs,
                },
                Card {
                    id: "p0c2".into(),
                    rank: Rank::Four,
                    suit: Suit::Hearts,
                },
            ],
            one: smallvec::smallvec![
                Card {
                    id: "p1c0".into(),
                    rank: Rank::Five,
                    suit: Suit::Diamonds,
                },
                Card {
                    id: "p1c1".into(),
                    rank: Rank::Four,
                    suit: Suit::Clubs,
                },
                Card {
                    id: "p1c2".into(),
                    rank: Rank::Three,
                    suit: Suit::Spades,
                },
            ],
        },
    )
    .expect("hand should start");

    game.apply_action_for_current_player(&Action::Raise { to: 3 })
        .expect("raise should succeed");
    game.apply_action_for_current_player(&Action::Fold)
        .expect("fold should finish the hand");

    assert_eq!(game.score(), &Score { zero: 1, one: 0 });
    assert_eq!(game.winner(), None);
    assert_eq!(game.dealer_for_next_hand(), 0);
    assert!(game
        .current_hand()
        .expect("finished hand should still be inspectable")
        .is_hand_over());

    let next_hand = game
        .start_hand(sample_turnup(), sample_hands())
        .expect("next hand should start after the previous one ends");
    assert_eq!(next_hand.state().dealer, 0);
    assert_eq!(game.current_player(), Some(1));
}

#[test]
fn decided_match_cannot_start_new_hand() {
    let mut game = Match::new(1, Score { zero: 12, one: 0 }).expect("match should initialize");

    let error = game
        .start_hand(sample_turnup(), sample_hands())
        .expect_err("decided match should reject new hands");

    assert_eq!(error, EngineError::MatchAlreadyDecided);
}

#[test]
fn match_requires_active_hand_for_delegated_actions() {
    let mut game = Match::new(1, Score { zero: 0, one: 0 }).expect("match should initialize");

    let error = game
        .apply_action_for_current_player(&Action::Raise { to: 3 })
        .expect_err("no active hand should reject delegated action");

    assert_eq!(error, EngineError::NoActiveHand);
}
