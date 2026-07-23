//! Seeded randomized playouts: whole matches driven by uniformly random
//! legal actions. Guards three properties the example-based tests cannot:
//!
//! 1. no reachable state panics or returns an unexpected error;
//! 2. every hand and match terminates within its structural bounds;
//! 3. every reachable state survives an export -> import round-trip — which
//!    also proves the state validator never rejects a reachable state.

use truco_engine::{Card, Engine, Hands, Match, Rank, Suit, Turnup};

/// Deterministic xorshift64* — no external RNG dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

fn full_deck() -> Vec<(Rank, Suit)> {
    let ranks = [
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Queen,
        Rank::Jack,
        Rank::King,
        Rank::Ace,
        Rank::Two,
        Rank::Three,
    ];
    let suits = [Suit::Diamonds, Suit::Spades, Suit::Hearts, Suit::Clubs];
    ranks
        .into_iter()
        .flat_map(|rank| suits.into_iter().map(move |suit| (rank, suit)))
        .collect()
}

fn random_deal(rng: &mut Rng) -> (Turnup, Hands) {
    let mut deck = full_deck();
    for i in (1..deck.len()).rev() {
        deck.swap(i, rng.below(i + 1));
    }
    let card = |player: usize, index: usize| {
        let (rank, suit) = deck[player * 3 + index];
        Card {
            id: format!("p{player}c{index}").into(),
            rank,
            suit,
        }
    };
    let turnup = Turnup {
        rank: deck[6].0,
        suit: deck[6].1,
    };
    let hands = Hands {
        zero: (0..3).map(|i| card(0, i)).collect(),
        one: (0..3).map(|i| card(1, i)).collect(),
    };
    (turnup, hands)
}

#[test]
fn random_matches_never_panic_terminate_and_round_trip() {
    let mut rng = Rng(0x5EED_CAFE_D00D_0001);

    for match_idx in 0..200 {
        let mut game = Match::new(
            (match_idx % 2) as u8,
            truco_engine::Score { zero: 0, one: 0 },
        )
        .expect("fresh match");
        let mut hands_played = 0usize;

        while game.winner().is_none() {
            hands_played += 1;
            assert!(
                hands_played <= 23,
                "match {match_idx}: every hand awards at least 1 point, so 23 hands must decide 12"
            );

            let (turnup, hands) = random_deal(&mut rng);
            game.start_hand(turnup, hands).expect("hand should start");

            let mut actions_taken = 0usize;
            while game.current_hand().is_some_and(|hand| !hand.is_hand_over()) {
                actions_taken += 1;
                assert!(
                    actions_taken <= 32,
                    "match {match_idx}: a hand admits at most ~18 actions, got stuck"
                );

                let actions = game
                    .legal_actions_for_current_player()
                    .expect("live hand must offer actions");
                assert!(!actions.is_empty(), "live hand offered no actions");
                let action = actions[rng.below(actions.len())].clone();
                game.apply_action_for_current_player(&action)
                    .expect("a legal action must apply cleanly");

                // Round-trip every intermediate state through both API layers.
                // `last_raised_by` is normalization metadata (cleared on
                // finished hands whose value fell back to 1), so compare it
                // via the normalized form; everything else must be identical.
                let engine = game.current_hand().expect("hand exists");
                let restored = Engine::from_exported_state(engine.export_state())
                    .expect("every reachable hand state must re-import");
                let mut expected = engine.state().clone();
                expected.last_raised_by = restored.state().last_raised_by;
                assert_eq!(restored.state(), &expected);
                assert_eq!(restored.hand_winner(), engine.hand_winner());
                // Normalization must be idempotent: a second round-trip is
                // exact.
                let again = Engine::from_exported_state(restored.export_state())
                    .expect("normalized state must re-import");
                assert_eq!(again.state(), restored.state());

                let match_restored =
                    Match::from_state(game.export_state()).expect("match state must re-import");
                assert_eq!(match_restored.score(), game.score());
                assert_eq!(match_restored.winner(), game.winner());
            }
        }

        let score = game.score();
        assert!(score.zero == 12 || score.one == 12);
    }
}
