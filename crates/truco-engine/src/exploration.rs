use std::collections::HashMap;

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ApiMatch, Card, EngineError, HandStart, Hands, MatchState, Player, Rank, Suit, Turnup,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplorationMatchSpec {
    pub base_state: MatchState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_hand: Option<PendingHandSpec>,
    #[serde(default)]
    pub hidden_ranges: HiddenRanges,
    #[serde(default)]
    pub hidden_plays: HiddenPlayRanges,
    #[serde(default)]
    pub completed_hidden_plays: Vec<CompletedHiddenPlayOverride>,
    #[serde(default)]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingHandSpec {
    pub turnup: Turnup,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiddenRanges {
    #[serde(rename = "0", default, skip_serializing_if = "Option::is_none")]
    pub zero: Option<PlayerRangeSpec>,
    #[serde(rename = "1", default, skip_serializing_if = "Option::is_none")]
    pub one: Option<PlayerRangeSpec>,
}

impl HiddenRanges {
    pub fn player(&self, player: Player) -> Option<&PlayerRangeSpec> {
        match player {
            0 => self.zero.as_ref(),
            1 => self.one.as_ref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiddenPlayRanges {
    #[serde(rename = "0", default, skip_serializing_if = "Option::is_none")]
    pub zero: Option<HiddenPlayRangeSpec>,
    #[serde(rename = "1", default, skip_serializing_if = "Option::is_none")]
    pub one: Option<HiddenPlayRangeSpec>,
}

impl HiddenPlayRanges {
    pub fn player(&self, player: Player) -> Option<&HiddenPlayRangeSpec> {
        match player {
            0 => self.zero.as_ref(),
            1 => self.one.as_ref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiddenPlayRangeSpec {
    pub weighted_cards: Vec<WeightedCardOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletedHiddenPlayOverride {
    pub round_index: usize,
    pub player: Player,
    pub weighted_cards: Vec<WeightedCardOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerRangeSpec {
    pub weighted_hands: Vec<WeightedHandOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedHandOption {
    pub weight: u32,
    pub hand: HandPattern,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedCardOption {
    pub weight: u32,
    pub card: CardConstraint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HandPattern {
    #[serde(rename = "exact")]
    Exact { cards: Vec<CardFace> },
    #[serde(rename = "ordered")]
    Ordered { slots: Vec<CardConstraint> },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CardFace {
    pub rank: Rank,
    pub suit: Suit,
}

impl CardFace {
    fn to_card_with_id(&self, id: std::sync::Arc<str>) -> Card {
        Card {
            id,
            rank: self.rank,
            suit: self.suit,
        }
    }
}

impl From<&Card> for CardFace {
    fn from(card: &Card) -> Self {
        Self {
            rank: card.rank,
            suit: card.suit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardConstraint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exact: Option<CardFace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_ranks: Option<Vec<Rank>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_suits: Option<Vec<Suit>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_manilha: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strength_at_least: Option<Rank>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strength_at_most: Option<Rank>,
}

#[derive(Debug, Error)]
pub enum ExplorationError {
    #[error("{0}")]
    Engine(#[from] EngineError),
    #[error("exploration spec requires an active hand")]
    NoActiveHand,
    #[error("exploration spec requires `pending_hand` when no active hand is present")]
    MissingPendingHandSpec,
    #[error("`pending_hand` may only be used when no active hand is present")]
    PendingHandRequiresNoActiveHand,
    #[error("hidden-play exploration requires an active hand")]
    HiddenPlayRequiresActiveHand,
    #[error("no hidden face-down play exists for that player")]
    NoHiddenPlayToOverride,
    #[error("no completed-round hidden face-down play exists at that target")]
    NoCompletedHiddenPlayToOverride,
    #[error("invalid player range")]
    InvalidPlayerRange,
    #[error("no range candidates matched the spec")]
    NoRangeCandidates,
}

impl ExplorationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Engine(error) => error.code(),
            Self::NoActiveHand => "NO_ACTIVE_HAND",
            Self::MissingPendingHandSpec => "MISSING_PENDING_HAND_SPEC",
            Self::PendingHandRequiresNoActiveHand => "PENDING_HAND_REQUIRES_NO_ACTIVE_HAND",
            Self::HiddenPlayRequiresActiveHand => "HIDDEN_PLAY_REQUIRES_ACTIVE_HAND",
            Self::NoHiddenPlayToOverride => "NO_HIDDEN_PLAY_TO_OVERRIDE",
            Self::NoCompletedHiddenPlayToOverride => "NO_COMPLETED_HIDDEN_PLAY_TO_OVERRIDE",
            Self::InvalidPlayerRange => "INVALID_PLAYER_RANGE",
            Self::NoRangeCandidates => "NO_RANGE_CANDIDATES",
        }
    }
}

pub fn resolve_match_spec(spec: &ExplorationMatchSpec) -> Result<MatchState, ExplorationError> {
    let api_match = ApiMatch::from_state(spec.base_state.clone())?;
    let match_state = api_match.export_state();
    let has_ranges = spec.hidden_ranges.zero.is_some() || spec.hidden_ranges.one.is_some();
    let has_hidden_plays = spec.hidden_plays.zero.is_some() || spec.hidden_plays.one.is_some();
    let has_completed_hidden_plays = !spec.completed_hidden_plays.is_empty();

    match (
        &match_state.current_hand,
        &spec.pending_hand,
        has_ranges,
        has_hidden_plays,
        has_completed_hidden_plays,
    ) {
        (_, None, false, false, false) => Ok(match_state),
        (Some(_), Some(_), _, _, _) => Err(ExplorationError::PendingHandRequiresNoActiveHand),
        (Some(_), None, _, _, _) => resolve_active_hand_ranges(
            match_state,
            &spec.hidden_ranges,
            &spec.hidden_plays,
            &spec.completed_hidden_plays,
            spec.seed,
        ),
        (None, _, _, true, _) | (None, _, _, _, true) => {
            Err(ExplorationError::HiddenPlayRequiresActiveHand)
        }
        (None, None, true, false, false) => Err(ExplorationError::MissingPendingHandSpec),
        (None, Some(pending_hand), _, false, false) => {
            resolve_pending_hand_spec(match_state, pending_hand, &spec.hidden_ranges, spec.seed)
        }
    }
}

fn resolve_active_hand_ranges(
    mut match_state: MatchState,
    hidden_ranges: &HiddenRanges,
    hidden_plays: &HiddenPlayRanges,
    completed_hidden_plays: &[CompletedHiddenPlayOverride],
    seed: Option<u64>,
) -> Result<MatchState, ExplorationError> {
    let participating_players: Vec<Player> = [0_u8, 1_u8]
        .into_iter()
        .filter(|player| {
            hidden_ranges.player(*player).is_some() || hidden_plays.player(*player).is_some()
        })
        .collect();
    let completed_targets: Vec<CompletedHiddenPlayTarget> = completed_hidden_plays
        .iter()
        .map(|override_spec| CompletedHiddenPlayTarget {
            round_index: override_spec.round_index,
            player: override_spec.player,
        })
        .collect();

    let current_hand_ref = match_state
        .current_hand
        .as_ref()
        .ok_or(ExplorationError::NoActiveHand)?;
    let available = available_cards(
        current_hand_ref,
        hidden_ranges,
        hidden_plays,
        completed_hidden_plays,
    );
    let current_hand = match_state
        .current_hand
        .as_mut()
        .ok_or(ExplorationError::NoActiveHand)?;

    let mut candidate_map: HashMap<Player, Vec<WeightedResolvedPrivateState>> = HashMap::new();
    for player in &participating_players {
        let hand_size = current_hand.state.hands.player(*player).len();
        let fixed_hidden_play = current_hidden_play(current_hand, *player);
        if hidden_plays.player(*player).is_some() && fixed_hidden_play.is_none() {
            return Err(ExplorationError::NoHiddenPlayToOverride);
        }

        let hidden_play_candidates = build_hidden_play_candidates(
            &available,
            fixed_hidden_play.as_ref(),
            hidden_plays.player(*player),
            &current_hand.state.turnup,
        )?;
        let mut private_candidates = Vec::new();

        for hidden_play_candidate in hidden_play_candidates {
            let mut hand_available = available.clone();
            if let Some(card) = &hidden_play_candidate.hidden_play {
                remove_card_faces(&mut hand_available, std::slice::from_ref(card));
            }

            let hand_candidates = build_hand_candidates(
                &hand_available,
                hand_size,
                hidden_ranges.player(*player),
                current_hand.state.hands.player(*player),
                &current_hand.state.turnup,
            )?;

            for hand_candidate in hand_candidates {
                private_candidates.push(WeightedResolvedPrivateState {
                    weight: hidden_play_candidate
                        .weight
                        .saturating_mul(hand_candidate.weight as u64),
                    hidden_play: hidden_play_candidate.hidden_play.clone(),
                    hand_cards: hand_candidate.cards,
                });
            }
        }

        if private_candidates.is_empty() {
            return Err(ExplorationError::NoRangeCandidates);
        }
        candidate_map.insert(*player, private_candidates);
    }

    let mut weighted_assignments = Vec::new();
    let mut assignment = HashMap::new();
    build_assignments(
        &participating_players,
        0,
        &candidate_map,
        &mut assignment,
        &mut weighted_assignments,
    );

    if weighted_assignments.is_empty() {
        return Err(ExplorationError::NoRangeCandidates);
    }

    let mut completed_candidate_map: HashMap<
        CompletedHiddenPlayTarget,
        Vec<WeightedResolvedCompletedHiddenPlay>,
    > = HashMap::new();
    for override_spec in completed_hidden_plays {
        let target = CompletedHiddenPlayTarget {
            round_index: override_spec.round_index,
            player: override_spec.player,
        };
        completed_hidden_play(current_hand, target)
            .ok_or(ExplorationError::NoCompletedHiddenPlayToOverride)?;
        let candidates = build_completed_hidden_play_candidates(
            &available,
            target,
            override_spec,
            &current_hand.state.turnup,
        )?;
        completed_candidate_map.insert(target, candidates);
    }

    let mut weighted_completed_assignments = Vec::new();
    let mut completed_assignment = HashMap::new();
    build_completed_assignments(
        &completed_targets,
        0,
        &completed_candidate_map,
        &mut completed_assignment,
        &mut weighted_completed_assignments,
    );

    if weighted_completed_assignments.is_empty() {
        return Err(ExplorationError::NoRangeCandidates);
    }

    let weighted_assignments: Vec<WeightedJointAssignment> = weighted_assignments
        .iter()
        .flat_map(|private_assignment| {
            weighted_completed_assignments
                .iter()
                .filter_map(|completed_assignment| {
                    if private_and_completed_overlap(
                        &private_assignment.assignments,
                        &completed_assignment.assignments,
                    ) {
                        None
                    } else {
                        Some(WeightedJointAssignment {
                            weight: private_assignment
                                .weight
                                .saturating_mul(completed_assignment.weight),
                            private_assignments: private_assignment.assignments.clone(),
                            completed_assignments: completed_assignment.assignments.clone(),
                        })
                    }
                })
        })
        .collect();

    if weighted_assignments.is_empty() {
        return Err(ExplorationError::NoRangeCandidates);
    }

    let mut rng = seed.map(seed_to_rng).unwrap_or_else(StdRng::from_entropy);
    let index = sample_weighted_index(&weighted_assignments, &mut rng)
        .ok_or(ExplorationError::NoRangeCandidates)?;
    let selected = &weighted_assignments[index];

    for player in participating_players {
        let chosen = selected
            .private_assignments
            .get(&player)
            .ok_or(ExplorationError::NoRangeCandidates)?;
        if hidden_plays.player(player).is_some() {
            let play = current_hand
                .state
                .current_round
                .plays
                .iter_mut()
                .find(|play| play.player == player)
                .ok_or(ExplorationError::NoHiddenPlayToOverride)?;
            let hidden_face = chosen
                .hidden_play
                .as_ref()
                .ok_or(ExplorationError::NoHiddenPlayToOverride)?;
            play.card.rank = hidden_face.rank;
            play.card.suit = hidden_face.suit;
        }
        if hidden_ranges.player(player).is_some() {
            let existing_ids: Vec<std::sync::Arc<str>> = current_hand
                .state
                .hands
                .player(player)
                .iter()
                .map(|card| card.id.clone())
                .collect();
            let mut cards = chosen.hand_cards.clone();
            cards.sort_by_key(|card| strength_key(card, &current_hand.state.turnup));
            let resolved_cards: smallvec::SmallVec<[Card; 3]> = existing_ids
                .into_iter()
                .zip(cards.iter())
                .map(|(id, card)| card.to_card_with_id(id))
                .collect();
            match player {
                0 => current_hand.state.hands.zero = resolved_cards,
                1 => current_hand.state.hands.one = resolved_cards,
                _ => return Err(ExplorationError::InvalidPlayerRange),
            }
        }
    }

    for (target, card) in &selected.completed_assignments {
        let round = current_hand
            .state
            .completed_rounds
            .get_mut(target.round_index)
            .ok_or(ExplorationError::NoCompletedHiddenPlayToOverride)?;
        let play = round
            .plays
            .iter_mut()
            .find(|play| {
                play.player == target.player
                    && matches!(play.visibility, crate::state::Visibility::Down)
            })
            .ok_or(ExplorationError::NoCompletedHiddenPlayToOverride)?;
        play.card.rank = card.rank;
        play.card.suit = card.suit;
    }

    Ok(ApiMatch::from_state(match_state)?.export_state())
}

fn resolve_pending_hand_spec(
    match_state: MatchState,
    pending_hand: &PendingHandSpec,
    hidden_ranges: &HiddenRanges,
    seed: Option<u64>,
) -> Result<MatchState, ExplorationError> {
    if match_state.current_hand.is_some() {
        return Err(ExplorationError::PendingHandRequiresNoActiveHand);
    }

    let mut api_match = ApiMatch::from_state(match_state)?;
    let ranged_players: Vec<Player> = [0_u8, 1_u8]
        .into_iter()
        .filter(|player| hidden_ranges.player(*player).is_some())
        .collect();
    let mut available_cards = full_deck();
    let turnup_face = CardFace {
        rank: pending_hand.turnup.rank,
        suit: pending_hand.turnup.suit,
    };
    available_cards.retain(|candidate| *candidate != turnup_face);

    let mut candidate_map: HashMap<Player, Vec<WeightedResolvedPrivateState>> = HashMap::new();
    for player in &ranged_players {
        let range = hidden_ranges
            .player(*player)
            .ok_or(ExplorationError::InvalidPlayerRange)?;
        let mut weighted_hands = Vec::new();
        for option in &range.weighted_hands {
            if option.weight == 0 {
                continue;
            }
            let matches =
                enumerate_matching_hands(&available_cards, 3, &option.hand, &pending_hand.turnup);
            for cards in matches {
                weighted_hands.push(WeightedResolvedHand {
                    weight: option.weight,
                    cards,
                });
            }
        }
        if weighted_hands.is_empty() {
            return Err(ExplorationError::NoRangeCandidates);
        }
        candidate_map.insert(
            *player,
            weighted_hands
                .into_iter()
                .map(|hand| WeightedResolvedPrivateState {
                    weight: hand.weight as u64,
                    hidden_play: None,
                    hand_cards: hand.cards,
                })
                .collect(),
        );
    }

    let mut weighted_assignments = Vec::new();
    let mut current_assignment = HashMap::new();
    build_assignments(
        &ranged_players,
        0,
        &candidate_map,
        &mut current_assignment,
        &mut weighted_assignments,
    );
    if weighted_assignments.is_empty() {
        weighted_assignments.push(WeightedAssignment {
            weight: 1,
            assignments: HashMap::new(),
        });
    }

    let mut rng = seed.map(seed_to_rng).unwrap_or_else(StdRng::from_entropy);
    let assignment_index = sample_weighted_index(&weighted_assignments, &mut rng)
        .ok_or(ExplorationError::NoRangeCandidates)?;
    let selected_assignment = &weighted_assignments[assignment_index];
    for resolved in selected_assignment.assignments.values() {
        remove_card_faces(&mut available_cards, &resolved.hand_cards);
    }

    let mut hands = Hands {
        zero: smallvec::SmallVec::new(),
        one: smallvec::SmallVec::new(),
    };

    for player in [0_u8, 1_u8] {
        let cards = if let Some(resolved) = selected_assignment.assignments.get(&player) {
            resolved.hand_cards.clone()
        } else {
            sample_uniform_hand(&available_cards, 3, &mut rng)?
        };
        remove_card_faces(&mut available_cards, &cards);
        let resolved_cards = cards_with_generated_ids(player, cards, &pending_hand.turnup);
        match player {
            0 => hands.zero = resolved_cards,
            1 => hands.one = resolved_cards,
            _ => return Err(ExplorationError::InvalidPlayerRange),
        }
    }

    api_match.start_hand(HandStart {
        turnup: pending_hand.turnup.clone(),
        hands,
    })?;

    Ok(api_match.export_state())
}

fn available_cards(
    current_hand: &crate::match_types::EngineState,
    hidden_ranges: &HiddenRanges,
    hidden_plays: &HiddenPlayRanges,
    completed_hidden_plays: &[CompletedHiddenPlayOverride],
) -> Vec<CardFace> {
    let turnup = CardFace {
        rank: current_hand.state.turnup.rank,
        suit: current_hand.state.turnup.suit,
    };
    let mut available = full_deck();
    available.retain(|candidate| *candidate != turnup);

    for (round_index, round) in current_hand.state.completed_rounds.iter().enumerate() {
        for play in &round.plays {
            let override_completed_hidden_play =
                matches!(play.visibility, crate::state::Visibility::Down)
                    && completed_hidden_plays.iter().any(|override_spec| {
                        override_spec.round_index == round_index
                            && override_spec.player == play.player
                    });
            if !override_completed_hidden_play {
                let face = CardFace::from(&play.card);
                available.retain(|candidate| *candidate != face);
            }
        }
    }
    for play in &current_hand.state.current_round.plays {
        let override_hidden_play = matches!(play.visibility, crate::state::Visibility::Down)
            && hidden_plays.player(play.player).is_some();
        if !override_hidden_play {
            let face = CardFace::from(&play.card);
            available.retain(|candidate| *candidate != face);
        }
    }
    for player in [0_u8, 1_u8] {
        if hidden_ranges.player(player).is_some() {
            continue;
        }
        for card in current_hand.state.hands.player(player) {
            let face = CardFace::from(card);
            available.retain(|candidate| *candidate != face);
        }
    }

    available
}

fn current_hidden_play(
    current_hand: &crate::match_types::EngineState,
    player: Player,
) -> Option<CardFace> {
    current_hand
        .state
        .current_round
        .plays
        .iter()
        .find(|play| {
            play.player == player && matches!(play.visibility, crate::state::Visibility::Down)
        })
        .map(|play| CardFace::from(&play.card))
}

fn completed_hidden_play(
    current_hand: &crate::match_types::EngineState,
    target: CompletedHiddenPlayTarget,
) -> Option<CardFace> {
    current_hand
        .state
        .completed_rounds
        .get(target.round_index)
        .and_then(|round| {
            round
                .plays
                .iter()
                .find(|play| {
                    play.player == target.player
                        && matches!(play.visibility, crate::state::Visibility::Down)
                })
                .map(|play| CardFace::from(&play.card))
        })
}

fn build_hidden_play_candidates(
    available: &[CardFace],
    fixed_hidden_play: Option<&CardFace>,
    override_spec: Option<&HiddenPlayRangeSpec>,
    turnup: &Turnup,
) -> Result<Vec<WeightedResolvedHiddenPlay>, ExplorationError> {
    if let Some(spec) = override_spec {
        let mut candidates = Vec::new();
        for option in &spec.weighted_cards {
            if option.weight == 0 {
                continue;
            }
            for card in available
                .iter()
                .filter(|card| constraint_matches(&option.card, card, turnup))
            {
                candidates.push(WeightedResolvedHiddenPlay {
                    weight: option.weight as u64,
                    hidden_play: Some(card.clone()),
                });
            }
        }
        if candidates.is_empty() {
            return Err(ExplorationError::NoRangeCandidates);
        }
        return Ok(candidates);
    }

    Ok(vec![WeightedResolvedHiddenPlay {
        weight: 1,
        hidden_play: fixed_hidden_play.cloned(),
    }])
}

fn build_completed_hidden_play_candidates(
    available: &[CardFace],
    _target: CompletedHiddenPlayTarget,
    override_spec: &CompletedHiddenPlayOverride,
    turnup: &Turnup,
) -> Result<Vec<WeightedResolvedCompletedHiddenPlay>, ExplorationError> {
    let mut matching_cards = Vec::new();
    for option in &override_spec.weighted_cards {
        if option.weight == 0 {
            continue;
        }
        for card in available
            .iter()
            .filter(|card| constraint_matches(&option.card, card, turnup))
        {
            matching_cards.push(WeightedResolvedCompletedHiddenPlay {
                weight: option.weight as u64,
                card: card.clone(),
            });
        }
    }

    if matching_cards.is_empty() {
        return Err(ExplorationError::NoRangeCandidates);
    }
    Ok(matching_cards)
}

fn build_hand_candidates(
    available: &[CardFace],
    hand_size: usize,
    override_spec: Option<&PlayerRangeSpec>,
    fixed_hand: &[Card],
    turnup: &Turnup,
) -> Result<Vec<WeightedResolvedHand>, ExplorationError> {
    if let Some(spec) = override_spec {
        let mut weighted_hands = Vec::new();
        for option in &spec.weighted_hands {
            if option.weight == 0 {
                continue;
            }
            let matches = enumerate_matching_hands(available, hand_size, &option.hand, turnup);
            for cards in matches {
                weighted_hands.push(WeightedResolvedHand {
                    weight: option.weight,
                    cards,
                });
            }
        }
        if weighted_hands.is_empty() {
            return Err(ExplorationError::NoRangeCandidates);
        }
        return Ok(weighted_hands);
    }

    Ok(vec![WeightedResolvedHand {
        weight: 1,
        cards: fixed_hand.iter().map(CardFace::from).collect(),
    }])
}

fn sample_uniform_hand(
    available: &[CardFace],
    hand_size: usize,
    rng: &mut StdRng,
) -> Result<Vec<CardFace>, ExplorationError> {
    let combos = enumerate_all_hands(available, hand_size);
    if combos.is_empty() {
        return Err(ExplorationError::NoRangeCandidates);
    }
    let index = rng.gen_range(0..combos.len());
    Ok(combos[index].clone())
}

fn enumerate_all_hands(available: &[CardFace], hand_size: usize) -> Vec<Vec<CardFace>> {
    let mut combos = Vec::new();
    let mut current = Vec::new();
    combinations(available, hand_size, 0, &mut current, &mut combos);
    combos
}

fn remove_card_faces(available: &mut Vec<CardFace>, cards: &[CardFace]) {
    for card in cards {
        if let Some(index) = available.iter().position(|candidate| candidate == card) {
            available.remove(index);
        }
    }
}

fn cards_with_generated_ids(
    player: Player,
    mut cards: Vec<CardFace>,
    turnup: &Turnup,
) -> smallvec::SmallVec<[Card; 3]> {
    cards.sort_by_key(|card| strength_key(card, turnup));
    cards
        .into_iter()
        .enumerate()
        .map(|(index, card)| card.to_card_with_id(format!("p{player}c{index}").into()))
        .collect()
}

fn enumerate_matching_hands(
    available: &[CardFace],
    hand_size: usize,
    pattern: &HandPattern,
    turnup: &Turnup,
) -> Vec<Vec<CardFace>> {
    let mut combos = Vec::new();
    let mut current = Vec::new();
    combinations(available, hand_size, 0, &mut current, &mut combos);
    combos
        .into_iter()
        .filter(|combo| hand_pattern_matches(pattern, combo, turnup))
        .collect()
}

fn combinations(
    items: &[CardFace],
    choose: usize,
    start: usize,
    current: &mut Vec<CardFace>,
    out: &mut Vec<Vec<CardFace>>,
) {
    if current.len() == choose {
        out.push(current.clone());
        return;
    }

    for index in start..items.len() {
        current.push(items[index].clone());
        combinations(items, choose, index + 1, current, out);
        current.pop();
    }
}

fn hand_pattern_matches(pattern: &HandPattern, cards: &[CardFace], turnup: &Turnup) -> bool {
    match pattern {
        HandPattern::Exact { cards: expected } => {
            if expected.len() != cards.len() {
                return false;
            }
            let mut expected = expected.clone();
            let mut actual = cards.to_vec();
            expected.sort_by_key(|card| strength_key(card, turnup));
            actual.sort_by_key(|card| strength_key(card, turnup));
            expected == actual
        }
        HandPattern::Ordered { slots } => {
            if slots.len() != cards.len() {
                return false;
            }
            let mut ordered = cards.to_vec();
            ordered.sort_by_key(|card| strength_key(card, turnup));
            ordered
                .iter()
                .zip(slots.iter())
                .all(|(card, constraint)| constraint_matches(constraint, card, turnup))
        }
    }
}

fn constraint_matches(constraint: &CardConstraint, card: &CardFace, turnup: &Turnup) -> bool {
    if let Some(exact) = &constraint.exact {
        if exact != card {
            return false;
        }
    }
    if let Some(ranks) = &constraint.allowed_ranks {
        if !ranks.contains(&card.rank) {
            return false;
        }
    }
    if let Some(suits) = &constraint.allowed_suits {
        if !suits.contains(&card.suit) {
            return false;
        }
    }
    if let Some(is_manilha) = constraint.is_manilha {
        let card_is_manilha = card.rank == turnup.rank.next_for_manilha();
        if is_manilha != card_is_manilha {
            return false;
        }
    }
    if let Some(min_rank) = constraint.strength_at_least {
        if strength_key(card, turnup) < rank_strength_key(min_rank) {
            return false;
        }
    }
    if let Some(max_rank) = constraint.strength_at_most {
        if strength_key(card, turnup) > rank_strength_key(max_rank) {
            return false;
        }
    }

    true
}

fn strength_key(card: &CardFace, turnup: &Turnup) -> u8 {
    if card.rank == turnup.rank.next_for_manilha() {
        10 + card.suit.manilha_strength() as u8
    } else {
        card.rank.index() as u8
    }
}

fn rank_strength_key(rank: Rank) -> u8 {
    rank.index() as u8
}

fn full_deck() -> Vec<CardFace> {
    let mut cards = Vec::new();
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

    for rank in ranks {
        for suit in suits {
            cards.push(CardFace { rank, suit });
        }
    }

    cards
}

#[derive(Debug, Clone)]
struct WeightedResolvedHand {
    weight: u32,
    cards: Vec<CardFace>,
}

#[derive(Debug, Clone)]
struct WeightedResolvedHiddenPlay {
    weight: u64,
    hidden_play: Option<CardFace>,
}

#[derive(Debug, Clone)]
struct WeightedResolvedPrivateState {
    weight: u64,
    hidden_play: Option<CardFace>,
    hand_cards: Vec<CardFace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CompletedHiddenPlayTarget {
    round_index: usize,
    player: Player,
}

#[derive(Debug, Clone)]
struct WeightedResolvedCompletedHiddenPlay {
    weight: u64,
    card: CardFace,
}

#[derive(Debug, Clone)]
struct WeightedAssignment {
    weight: u64,
    assignments: HashMap<Player, WeightedResolvedPrivateState>,
}

#[derive(Debug, Clone)]
struct WeightedCompletedAssignment {
    weight: u64,
    assignments: HashMap<CompletedHiddenPlayTarget, CardFace>,
}

#[derive(Debug, Clone)]
struct WeightedJointAssignment {
    weight: u64,
    private_assignments: HashMap<Player, WeightedResolvedPrivateState>,
    completed_assignments: HashMap<CompletedHiddenPlayTarget, CardFace>,
}

fn build_assignments(
    players: &[Player],
    index: usize,
    candidate_map: &HashMap<Player, Vec<WeightedResolvedPrivateState>>,
    current: &mut HashMap<Player, WeightedResolvedPrivateState>,
    out: &mut Vec<WeightedAssignment>,
) {
    if index == players.len() {
        let weight = current
            .values()
            .fold(1_u64, |acc, state| acc.saturating_mul(state.weight));
        out.push(WeightedAssignment {
            weight,
            assignments: current.clone(),
        });
        return;
    }

    let player = players[index];
    let Some(candidates) = candidate_map.get(&player) else {
        return;
    };

    for candidate in candidates {
        if current
            .values()
            .any(|chosen| private_states_overlap(chosen, candidate))
        {
            continue;
        }
        current.insert(player, candidate.clone());
        build_assignments(players, index + 1, candidate_map, current, out);
        current.remove(&player);
    }
}

fn build_completed_assignments(
    targets: &[CompletedHiddenPlayTarget],
    index: usize,
    candidate_map: &HashMap<CompletedHiddenPlayTarget, Vec<WeightedResolvedCompletedHiddenPlay>>,
    current: &mut HashMap<CompletedHiddenPlayTarget, CardFace>,
    out: &mut Vec<WeightedCompletedAssignment>,
) {
    if index == targets.len() {
        let weight = current.iter().fold(1_u64, |acc, (target, card)| {
            let candidate_weight = candidate_map
                .get(target)
                .and_then(|candidates| candidates.iter().find(|candidate| candidate.card == *card))
                .map(|candidate| candidate.weight)
                .unwrap_or(1);
            acc.saturating_mul(candidate_weight)
        });
        out.push(WeightedCompletedAssignment {
            weight,
            assignments: current.clone(),
        });
        return;
    }

    let target = targets[index];
    let Some(candidates) = candidate_map.get(&target) else {
        return;
    };

    for candidate in candidates {
        if current.values().any(|chosen| chosen == &candidate.card) {
            continue;
        }
        current.insert(target, candidate.card.clone());
        build_completed_assignments(targets, index + 1, candidate_map, current, out);
        current.remove(&target);
    }
}

fn private_states_overlap(
    left: &WeightedResolvedPrivateState,
    right: &WeightedResolvedPrivateState,
) -> bool {
    let left_hidden = left.hidden_play.iter();
    let right_hidden = right.hidden_play.iter();
    let mut left_cards = left.hand_cards.iter().chain(left_hidden);
    let right_cards: Vec<&CardFace> = right.hand_cards.iter().chain(right_hidden).collect();
    left_cards.any(|left_card| right_cards.contains(&left_card))
}

fn private_and_completed_overlap(
    private_assignments: &HashMap<Player, WeightedResolvedPrivateState>,
    completed_assignments: &HashMap<CompletedHiddenPlayTarget, CardFace>,
) -> bool {
    private_assignments.values().any(|state| {
        state
            .hand_cards
            .iter()
            .chain(state.hidden_play.iter())
            .any(|card| {
                completed_assignments
                    .values()
                    .any(|completed| completed == card)
            })
    })
}

fn sample_weighted_index<T: WeightedItem>(items: &[T], rng: &mut StdRng) -> Option<usize> {
    let total_weight: u64 = items.iter().map(WeightedItem::weight).sum();
    if total_weight == 0 {
        return None;
    }

    let mut target = rng.gen_range(0..total_weight);
    for (index, item) in items.iter().enumerate() {
        if target < item.weight() {
            return Some(index);
        }
        target -= item.weight();
    }

    None
}

trait WeightedItem {
    fn weight(&self) -> u64;
}

impl WeightedItem for WeightedAssignment {
    fn weight(&self) -> u64 {
        self.weight
    }
}

impl WeightedItem for WeightedJointAssignment {
    fn weight(&self) -> u64 {
        self.weight
    }
}

impl WeightedItem for WeightedCompletedAssignment {
    fn weight(&self) -> u64 {
        self.weight
    }
}

fn seed_to_rng(seed: u64) -> StdRng {
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    StdRng::from_seed(bytes)
}
