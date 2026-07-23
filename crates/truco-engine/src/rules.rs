use std::cmp::Ordering;

use crate::state::Visibility;
use crate::{
    Card, CompletedRound, PlayedCard, Player, Score, Turnup, INITIAL_HAND_VALUE, MATCH_TARGET,
    RAISE_LADDER,
};

pub(crate) const ELEVEN_AUTO_HAND_VALUE: u8 = 3;
pub(crate) const ELEVEN_FOLD_AWARD: u8 = INITIAL_HAND_VALUE;

pub(crate) fn compare_plays(
    turnup: &Turnup,
    first: &PlayedCard,
    second: &PlayedCard,
) -> Option<Player> {
    match (first.visibility, second.visibility) {
        (Visibility::Down, Visibility::Down) => None,
        (Visibility::Down, Visibility::Up) => Some(second.player),
        (Visibility::Up, Visibility::Down) => Some(first.player),
        (Visibility::Up, Visibility::Up) => {
            match compare_face_up_cards(turnup, &first.card, &second.card) {
                Ordering::Greater => Some(first.player),
                Ordering::Less => Some(second.player),
                Ordering::Equal => None,
            }
        }
    }
}

pub(crate) fn decide_hand_winner(
    rounds: &[CompletedRound],
    first_round_leader: Player,
) -> Option<Player> {
    let wins_zero = rounds
        .iter()
        .filter(|round| round.winner == Some(0))
        .count();
    let wins_one = rounds
        .iter()
        .filter(|round| round.winner == Some(1))
        .count();

    if wins_zero >= 2 {
        return Some(0);
    }
    if wins_one >= 2 {
        return Some(1);
    }

    if rounds.len() >= 2 {
        if wins_zero == 1 && wins_one == 0 && rounds.iter().any(|round| round.winner.is_none()) {
            return Some(0);
        }
        if wins_one == 1 && wins_zero == 0 && rounds.iter().any(|round| round.winner.is_none()) {
            return Some(1);
        }
    }

    if rounds.len() == 3 {
        if wins_zero == 1 && wins_one == 1 {
            return rounds.iter().find_map(|round| round.winner);
        }
        return Some(first_round_leader);
    }

    None
}

pub(crate) fn expected_round_leader(dealer: Player, completed_rounds: &[CompletedRound]) -> Player {
    if let Some(last_round) = completed_rounds.last() {
        last_round.winner.unwrap_or(last_round.leader)
    } else {
        other_player(dealer)
    }
}

pub(crate) fn first_round_leader(dealer: Player, completed_rounds: &[CompletedRound]) -> Player {
    completed_rounds
        .first()
        .map(|round| round.leader)
        .unwrap_or_else(|| other_player(dealer))
}

pub(crate) fn next_raise_after(value: u8) -> Option<u8> {
    RAISE_LADDER
        .iter()
        .position(|step| *step == value)
        .and_then(|index| RAISE_LADDER.get(index + 1).copied())
}

pub(crate) fn other_player(player: Player) -> Player {
    match player {
        0 => 1,
        _ => 0,
    }
}

pub(crate) fn score_triggers_eleven_hand(score: &Score) -> bool {
    score.zero == 11 || score.one == 11
}

pub(crate) fn both_players_on_eleven(score: &Score) -> bool {
    score.zero == 11 && score.one == 11
}

pub(crate) fn starting_hand_value(score: &Score) -> u8 {
    if both_players_on_eleven(score) {
        ELEVEN_AUTO_HAND_VALUE
    } else {
        INITIAL_HAND_VALUE
    }
}

pub(crate) fn eleven_hand_decision_player(score: &Score) -> Option<Player> {
    match (score.zero == 11, score.one == 11) {
        (true, false) => Some(0),
        (false, true) => Some(1),
        _ => None,
    }
}

pub(crate) fn score_winner(score: &Score) -> Option<Player> {
    if score.zero >= MATCH_TARGET {
        Some(0)
    } else if score.one >= MATCH_TARGET {
        Some(1)
    } else {
        None
    }
}

fn compare_face_up_cards(turnup: &Turnup, first: &Card, second: &Card) -> Ordering {
    let first_is_manilha = first.is_manilha(turnup);
    let second_is_manilha = second.is_manilha(turnup);

    match (first_is_manilha, second_is_manilha) {
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (true, true) => first
            .suit
            .manilha_strength()
            .cmp(&second.suit.manilha_strength()),
        (false, false) => first.rank.index().cmp(&second.rank.index()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{Rank, Suit};

    fn round(leader: Player, winner: Option<Player>) -> CompletedRound {
        CompletedRound {
            leader,
            winner,
            plays: Default::default(),
        }
    }

    #[test]
    fn hand_winner_covers_the_paulista_tie_ladder() {
        // Two straight wins.
        assert_eq!(
            decide_hand_winner(&[round(0, Some(1)), round(1, Some(1))], 0),
            Some(1)
        );
        // Tied first round: round 2 decides immediately.
        assert_eq!(
            decide_hand_winner(&[round(0, None), round(0, Some(1))], 0),
            Some(1)
        );
        // Win then tie: the first winner takes the hand.
        assert_eq!(
            decide_hand_winner(&[round(0, Some(0)), round(0, None)], 0),
            Some(0)
        );
        // 1-1 after two rounds: still open.
        assert_eq!(
            decide_hand_winner(&[round(0, Some(0)), round(0, Some(1))], 0),
            None
        );
        // 1-1 plus a tied third round: first round's winner.
        assert_eq!(
            decide_hand_winner(&[round(0, Some(1)), round(1, Some(0)), round(0, None)], 0),
            Some(1)
        );
        // Two ties then a win.
        assert_eq!(
            decide_hand_winner(&[round(0, None), round(0, None), round(0, Some(1))], 0),
            Some(1)
        );
        // All three tied: the first-round leader (the mao) wins.
        assert_eq!(
            decide_hand_winner(&[round(1, None), round(1, None), round(1, None)], 1),
            Some(1)
        );
        // Single completed round decides nothing.
        assert_eq!(decide_hand_winner(&[round(0, Some(0))], 0), None);
        assert_eq!(decide_hand_winner(&[round(0, None)], 0), None);
    }

    #[test]
    fn raise_ladder_steps() {
        assert_eq!(next_raise_after(1), Some(3));
        assert_eq!(next_raise_after(3), Some(6));
        assert_eq!(next_raise_after(6), Some(9));
        assert_eq!(next_raise_after(9), Some(12));
        assert_eq!(next_raise_after(12), None);
        assert_eq!(next_raise_after(2), None);
    }

    #[test]
    fn tied_round_keeps_leader_and_win_moves_it() {
        assert_eq!(expected_round_leader(1, &[]), 0);
        assert_eq!(expected_round_leader(1, &[round(0, Some(1))]), 1);
        assert_eq!(expected_round_leader(1, &[round(0, None)]), 0);
    }

    #[test]
    fn face_down_cards_never_win_and_plain_ranks_can_tie() {
        let turnup = Turnup {
            rank: Rank::Four,
            suit: Suit::Spades,
        };
        let up = |player: Player, rank: Rank, suit: Suit| PlayedCard {
            player,
            visibility: Visibility::Up,
            card: Card {
                id: format!("t{player}{rank:?}{suit:?}").into(),
                rank,
                suit,
            },
        };
        let down = |player: Player| PlayedCard {
            player,
            visibility: Visibility::Down,
            card: Card {
                id: format!("d{player}").into(),
                rank: Rank::Three,
                suit: Suit::Clubs,
            },
        };

        // Both hidden: tie. Hidden loses to any face-up card, even the weakest.
        assert_eq!(compare_plays(&turnup, &down(0), &down(1)), None);
        assert_eq!(
            compare_plays(&turnup, &down(0), &up(1, Rank::Four, Suit::Hearts)),
            Some(1)
        );
        // Equal plain ranks tie regardless of suit.
        assert_eq!(
            compare_plays(
                &turnup,
                &up(0, Rank::King, Suit::Clubs),
                &up(1, Rank::King, Suit::Hearts)
            ),
            None
        );
        // Manilha (turnup 4 -> manilha 5) beats the top plain rank; manilha
        // suits order diamonds < spades < hearts < clubs.
        assert_eq!(
            compare_plays(
                &turnup,
                &up(0, Rank::Five, Suit::Diamonds),
                &up(1, Rank::Three, Suit::Clubs)
            ),
            Some(0)
        );
        assert_eq!(
            compare_plays(
                &turnup,
                &up(0, Rank::Five, Suit::Diamonds),
                &up(1, Rank::Five, Suit::Clubs)
            ),
            Some(1)
        );
    }
}
