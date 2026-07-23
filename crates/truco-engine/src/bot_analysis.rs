use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    rules::other_player, state::Visibility, Action, Card, EngineError, Match, MatchState, Player,
    Rank, Suit, Turnup,
};

#[derive(Debug, Error)]
pub enum BotAnalysisError {
    #[error("{0}")]
    Engine(#[from] EngineError),
    #[error("bot received no legal actions")]
    NoLegalActions,
    #[error("{0}")]
    InvalidAnalysis(String),
}

type BotError = BotAnalysisError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalTurnAnalysis {
    pub player: Player,
    pub action_summaries: Vec<TacticalActionSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalActionSummary {
    pub action: Action,
    pub force_hand_win: bool,
    pub worst_case_signed_points: i32,
    pub average_signed_points: f32,
    pub winning_determinizations: usize,
    pub total_determinizations: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CardFace {
    rank: Rank,
    suit: Suit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HiddenSlot {
    Completed {
        round_index: usize,
        play_index: usize,
    },
    Current {
        play_index: usize,
    },
}

#[derive(Debug, Clone)]
struct PerspectiveRequest {
    opponent: Player,
    turnup: Turnup,
    opponent_hand_ids: Vec<std::sync::Arc<str>>,
    hidden_slots: Vec<HiddenSlot>,
    available_faces: Vec<CardFace>,
}

#[derive(Debug, Clone, Copy)]
struct AggregateScore {
    worst_case_signed_points: i32,
    total_signed_points: i32,
    winning_determinizations: usize,
}

pub fn analyze_tactical_turn(
    game: &Match,
    player: Player,
) -> Result<TacticalTurnAnalysis, BotError> {
    let legal_actions = game.strategic_legal_actions(player)?;
    if legal_actions.is_empty() {
        return Err(BotError::NoLegalActions);
    }

    let perspective = build_perspective_request(&game.export_state(), player)?;
    let mut aggregates = fresh_aggregates(legal_actions.len());
    let mut total_determinizations = 0_usize;

    enumerate_determinizations(&perspective, |hidden_faces, hand_faces| {
        accumulate_determinization(
            game,
            &perspective,
            player,
            &legal_actions,
            hidden_faces,
            hand_faces,
            &mut aggregates,
            &mut total_determinizations,
        )
    })?;

    finalize_analysis(
        player,
        legal_actions,
        aggregates,
        total_determinizations,
        false,
    )
}

/// Same analysis as [`analyze_tactical_turn`], but falls back to Monte Carlo
/// sampling when enumerating every determinization would exceed `max_samples`.
///
/// Sampling is used only when the exact count of compatible determinizations
/// exceeds the budget; otherwise we fall through to the exhaustive path and
/// return identical results. When sampling kicks in, `force_hand_win` is
/// suppressed (set to `false`) for every action summary because proving a
/// forced win requires observing *all* determinizations, not a subset.
pub fn analyze_tactical_turn_sampled<R: Rng>(
    game: &Match,
    player: Player,
    max_samples: usize,
    rng: &mut R,
) -> Result<TacticalTurnAnalysis, BotError> {
    let legal_actions = game.strategic_legal_actions(player)?;
    if legal_actions.is_empty() {
        return Err(BotError::NoLegalActions);
    }

    let perspective = build_perspective_request(&game.export_state(), player)?;
    let total = count_determinizations(&perspective);
    let use_sampling = max_samples > 0 && total > max_samples;

    let mut aggregates = fresh_aggregates(legal_actions.len());
    let mut total_determinizations = 0_usize;

    if use_sampling {
        let samples = draw_sampled_determinizations(&perspective, max_samples, rng);
        let sample_values: Result<Vec<Vec<i32>>, BotError> = samples
            .par_iter()
            .map(|(hidden_faces, hand_faces)| {
                let determinized = determinize_match(game, &perspective, hidden_faces, hand_faces)?;
                evaluate_root_actions(&determinized, player, &legal_actions)
            })
            .collect();
        for values in sample_values? {
            total_determinizations += 1;
            accumulate_values(&mut aggregates, values);
        }
    } else {
        enumerate_determinizations(&perspective, |hidden_faces, hand_faces| {
            accumulate_determinization(
                game,
                &perspective,
                player,
                &legal_actions,
                hidden_faces,
                hand_faces,
                &mut aggregates,
                &mut total_determinizations,
            )
        })?;
    }

    finalize_analysis(
        player,
        legal_actions,
        aggregates,
        total_determinizations,
        use_sampling,
    )
}

fn fresh_aggregates(len: usize) -> Vec<AggregateScore> {
    vec![
        AggregateScore {
            worst_case_signed_points: i32::MAX,
            total_signed_points: 0,
            winning_determinizations: 0,
        };
        len
    ]
}

#[allow(clippy::too_many_arguments)]
fn accumulate_determinization(
    game: &Match,
    perspective: &PerspectiveRequest,
    player: Player,
    legal_actions: &[Action],
    hidden_faces: &[CardFace],
    hand_faces: &[CardFace],
    aggregates: &mut [AggregateScore],
    total_determinizations: &mut usize,
) -> Result<(), BotError> {
    *total_determinizations += 1;
    let determinized = determinize_match(game, perspective, hidden_faces, hand_faces)?;
    let values = evaluate_root_actions(&determinized, player, legal_actions)?;

    accumulate_values(aggregates, values);

    Ok(())
}

fn accumulate_values(aggregates: &mut [AggregateScore], values: Vec<i32>) {
    for (aggregate, value) in aggregates.iter_mut().zip(values) {
        aggregate.worst_case_signed_points = aggregate.worst_case_signed_points.min(value);
        aggregate.total_signed_points += value;
        if value > 0 {
            aggregate.winning_determinizations += 1;
        }
    }
}

fn finalize_analysis(
    player: Player,
    legal_actions: Vec<Action>,
    aggregates: Vec<AggregateScore>,
    total_determinizations: usize,
    sampled: bool,
) -> Result<TacticalTurnAnalysis, BotError> {
    if total_determinizations == 0 {
        return Err(BotError::InvalidAnalysis(
            "tactical analysis found no compatible determinizations".to_string(),
        ));
    }

    let action_summaries = legal_actions
        .into_iter()
        .zip(aggregates)
        .map(|(action, aggregate)| TacticalActionSummary {
            force_hand_win: !sampled
                && aggregate.winning_determinizations == total_determinizations,
            worst_case_signed_points: aggregate.worst_case_signed_points,
            average_signed_points: aggregate.total_signed_points as f32
                / total_determinizations as f32,
            winning_determinizations: aggregate.winning_determinizations,
            total_determinizations,
            action,
        })
        .collect();

    Ok(TacticalTurnAnalysis {
        player,
        action_summaries,
    })
}

fn count_determinizations(perspective: &PerspectiveRequest) -> usize {
    let available = perspective.available_faces.len();
    let hidden = perspective.hidden_slots.len();
    let hand = perspective.opponent_hand_ids.len();
    if available < hidden + hand {
        return 0;
    }

    // Permutations for the ordered hidden-slot assignments.
    let mut total: usize = 1;
    for i in 0..hidden {
        total = total.saturating_mul(available - i);
    }

    // Combinations for the unordered opponent-hand assignment.
    let remaining = available - hidden;
    let mut num: usize = 1;
    let mut den: usize = 1;
    for i in 0..hand {
        num = num.saturating_mul(remaining - i);
        den = den.saturating_mul(i + 1);
    }
    total = total.saturating_mul(num / den);
    total
}

fn draw_sampled_determinizations<R>(
    perspective: &PerspectiveRequest,
    samples: usize,
    rng: &mut R,
) -> Vec<(Vec<CardFace>, Vec<CardFace>)>
where
    R: Rng,
{
    let hidden = perspective.hidden_slots.len();
    let hand = perspective.opponent_hand_ids.len();
    let needed = hidden + hand;
    if perspective.available_faces.len() < needed {
        return Vec::new();
    }

    let mut pool = perspective.available_faces.clone();
    let mut hand_faces = Vec::with_capacity(hand);
    let mut determinizations = Vec::with_capacity(samples);

    for _ in 0..samples {
        // Partial Fisher-Yates: shuffle the first `needed` slots of the pool.
        for i in 0..needed {
            let j = rng.gen_range(i..pool.len());
            pool.swap(i, j);
        }

        hand_faces.clear();
        hand_faces.extend_from_slice(&pool[hidden..hidden + hand]);
        hand_faces.sort_by(|left, right| compare_faces(*left, *right, &perspective.turnup));

        determinizations.push((pool[..hidden].to_vec(), hand_faces.clone()));
    }

    determinizations
}

fn build_perspective_request(
    match_state: &MatchState,
    actor: Player,
) -> Result<PerspectiveRequest, BotError> {
    let current_hand = match_state
        .current_hand
        .as_ref()
        .ok_or(BotError::Engine(EngineError::NoActiveHand))?;
    let opponent = other_player(actor);
    let mut known_faces: Vec<CardFace> = vec![CardFace {
        rank: current_hand.state.turnup.rank,
        suit: current_hand.state.turnup.suit,
    }];
    let mut hidden_slots = Vec::new();

    for card in current_hand.state.hands.player(actor) {
        known_faces.push(CardFace::from(card));
    }

    for (round_index, round) in current_hand.state.completed_rounds.iter().enumerate() {
        for (play_index, play) in round.plays.iter().enumerate() {
            if play.player == actor || matches!(play.visibility, Visibility::Up) {
                known_faces.push(CardFace::from(&play.card));
            } else {
                hidden_slots.push(HiddenSlot::Completed {
                    round_index,
                    play_index,
                });
            }
        }
    }

    for (play_index, play) in current_hand.state.current_round.plays.iter().enumerate() {
        if play.player == actor || matches!(play.visibility, Visibility::Up) {
            known_faces.push(CardFace::from(&play.card));
        } else {
            hidden_slots.push(HiddenSlot::Current { play_index });
        }
    }

    let available_faces = full_deck()
        .into_iter()
        .filter(|face| !known_faces.contains(face))
        .collect();
    let opponent_hand_ids = current_hand
        .state
        .hands
        .player(opponent)
        .iter()
        .map(|card| card.id.clone())
        .collect();

    Ok(PerspectiveRequest {
        opponent,
        turnup: current_hand.state.turnup.clone(),
        opponent_hand_ids,
        hidden_slots,
        available_faces,
    })
}

fn enumerate_determinizations<F>(
    perspective: &PerspectiveRequest,
    mut visit: F,
) -> Result<(), BotError>
where
    F: FnMut(&[CardFace], &[CardFace]) -> Result<(), BotError>,
{
    let mut hidden_faces = Vec::with_capacity(perspective.hidden_slots.len());
    let mut available_faces = perspective.available_faces.clone();
    available_faces.sort_by(|left, right| compare_faces(*left, *right, &perspective.turnup));

    enumerate_hidden_faces(
        perspective,
        0,
        &mut available_faces,
        &mut hidden_faces,
        &mut visit,
    )
}

fn enumerate_hidden_faces<F>(
    perspective: &PerspectiveRequest,
    index: usize,
    available_faces: &mut Vec<CardFace>,
    hidden_faces: &mut Vec<CardFace>,
    visit: &mut F,
) -> Result<(), BotError>
where
    F: FnMut(&[CardFace], &[CardFace]) -> Result<(), BotError>,
{
    if index == perspective.hidden_slots.len() {
        let mut hand_faces = Vec::with_capacity(perspective.opponent_hand_ids.len());
        return enumerate_hand_faces(
            perspective,
            available_faces,
            0,
            &mut hand_faces,
            hidden_faces,
            visit,
        );
    }

    for face_index in 0..available_faces.len() {
        let face = available_faces.remove(face_index);
        hidden_faces.push(face);
        enumerate_hidden_faces(perspective, index + 1, available_faces, hidden_faces, visit)?;
        hidden_faces.pop();
        available_faces.insert(face_index, face);
    }

    Ok(())
}

fn enumerate_hand_faces<F>(
    perspective: &PerspectiveRequest,
    available_faces: &[CardFace],
    start_index: usize,
    chosen_faces: &mut Vec<CardFace>,
    hidden_faces: &[CardFace],
    visit: &mut F,
) -> Result<(), BotError>
where
    F: FnMut(&[CardFace], &[CardFace]) -> Result<(), BotError>,
{
    if chosen_faces.len() == perspective.opponent_hand_ids.len() {
        let mut hand_faces = chosen_faces.to_vec();
        hand_faces.sort_by(|left, right| compare_faces(*left, *right, &perspective.turnup));
        return visit(hidden_faces, &hand_faces);
    }

    let remaining = perspective.opponent_hand_ids.len() - chosen_faces.len();
    for face_index in start_index..=available_faces.len() - remaining {
        chosen_faces.push(available_faces[face_index]);
        enumerate_hand_faces(
            perspective,
            available_faces,
            face_index + 1,
            chosen_faces,
            hidden_faces,
            visit,
        )?;
        chosen_faces.pop();
    }

    Ok(())
}

fn determinize_match(
    game: &Match,
    perspective: &PerspectiveRequest,
    hidden_faces: &[CardFace],
    hand_faces: &[CardFace],
) -> Result<Match, BotError> {
    let mut match_state = game.export_state();
    let current_hand = match_state
        .current_hand
        .as_mut()
        .ok_or(BotError::Engine(EngineError::NoActiveHand))?;

    let opponent_hand = match perspective.opponent {
        0 => &mut current_hand.state.hands.zero,
        1 => &mut current_hand.state.hands.one,
        _ => return Err(BotError::Engine(EngineError::InvalidInitialState)),
    };
    *opponent_hand = perspective
        .opponent_hand_ids
        .iter()
        .zip(hand_faces.iter())
        .map(|(id, face)| face.card(id.clone()))
        .collect();

    for (slot, face) in perspective.hidden_slots.iter().zip(hidden_faces.iter()) {
        match *slot {
            HiddenSlot::Completed {
                round_index,
                play_index,
            } => {
                let play = &mut current_hand.state.completed_rounds[round_index].plays[play_index];
                play.card.rank = face.rank;
                play.card.suit = face.suit;
            }
            HiddenSlot::Current { play_index } => {
                let play = &mut current_hand.state.current_round.plays[play_index];
                play.card.rank = face.rank;
                play.card.suit = face.suit;
            }
        }
    }

    Match::from_state(match_state).map_err(BotError::from)
}

fn evaluate_root_actions(
    game: &Match,
    player: Player,
    legal_actions: &[Action],
) -> Result<Vec<i32>, BotError> {
    let mut values = Vec::with_capacity(legal_actions.len());

    for action in legal_actions {
        let mut next = game.clone();
        next.apply_action(player, action)?;
        values.push(minimax_hand_value(&next, player)?);
    }

    Ok(values)
}

fn minimax_hand_value(game: &Match, player: Player) -> Result<i32, BotError> {
    let Some(current_hand) = game.current_hand() else {
        return Err(BotError::Engine(EngineError::NoActiveHand));
    };

    if current_hand.is_hand_over() {
        let winner = current_hand
            .hand_winner()
            .ok_or(BotError::Engine(EngineError::HandAlreadyDecided))?;
        let awarded_points = current_hand.state().hand_value as i32;
        return Ok(if winner == player {
            awarded_points
        } else {
            -awarded_points
        });
    }

    let current_player = game
        .current_player()
        .ok_or(BotError::Engine(EngineError::HandAlreadyDecided))?;
    let mut best_value = if current_player == player {
        i32::MIN
    } else {
        i32::MAX
    };

    for action in game.strategic_legal_actions_for_current_player()? {
        let mut next = game.clone();
        next.apply_action_for_current_player(&action)?;
        let next_value = minimax_hand_value(&next, player)?;
        if current_player == player {
            best_value = best_value.max(next_value);
        } else {
            best_value = best_value.min(next_value);
        }
    }

    Ok(best_value)
}

fn full_deck() -> Vec<CardFace> {
    const RANKS: [Rank; 10] = [
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
    const SUITS: [Suit; 4] = [Suit::Diamonds, Suit::Spades, Suit::Hearts, Suit::Clubs];

    let mut deck = Vec::with_capacity(RANKS.len() * SUITS.len());
    for rank in RANKS {
        for suit in SUITS {
            deck.push(CardFace { rank, suit });
        }
    }
    deck
}

fn compare_faces(left: CardFace, right: CardFace, turnup: &Turnup) -> std::cmp::Ordering {
    face_strength(left, turnup)
        .cmp(&face_strength(right, turnup))
        .then_with(|| {
            left.suit
                .manilha_strength()
                .cmp(&right.suit.manilha_strength())
        })
}

fn face_strength(face: CardFace, turnup: &Turnup) -> usize {
    if face.rank == turnup.rank.next_for_manilha() {
        10 + face.suit.manilha_strength()
    } else {
        face.rank.index()
    }
}

impl CardFace {
    fn card(self, id: std::sync::Arc<str>) -> Card {
        Card {
            id,
            rank: self.rank,
            suit: self.suit,
        }
    }
}

impl From<&Card> for CardFace {
    fn from(value: &Card) -> Self {
        Self {
            rank: value.rank,
            suit: value.suit,
        }
    }
}
