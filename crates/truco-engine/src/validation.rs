use crate::rules::{
    both_players_on_eleven, compare_plays, decide_hand_winner, eleven_hand_decision_player,
    expected_round_leader, first_round_leader, next_raise_after, other_player,
    score_triggers_eleven_hand, score_winner, starting_hand_value, ELEVEN_AUTO_HAND_VALUE,
};
use crate::state::Visibility;
use crate::{Card, EngineError, GameState, Player, INITIAL_HAND_VALUE, MATCH_TARGET, RAISE_LADDER};

pub fn validate_state(state: &GameState) -> Result<(), EngineError> {
    let state = normalize_state(state.clone());
    validate_normalized_state(&state)
}

pub(crate) fn validate_exported_engine_state(
    state: &GameState,
    hand_winner: Option<Player>,
    match_winner: Option<Player>,
) -> Result<(), EngineError> {
    let state = normalize_state(state.clone());
    if let Some(winner) = hand_winner {
        validate_finished_normalized_state(&state, winner, match_winner)
    } else {
        validate_normalized_state(&state)?;
        if score_winner(&state.score).is_some() || match_winner.is_some() {
            return Err(EngineError::InvalidInitialState);
        }
        Ok(())
    }
}

pub(crate) fn normalize_state(mut state: GameState) -> GameState {
    if state.last_raised_by.is_none() {
        state.last_raised_by = state
            .pending_raise
            .as_ref()
            .map(|pending_raise| pending_raise.raised_by);
    }
    if state.hand_value == 1 && state.pending_raise.is_none() {
        state.last_raised_by = None;
    }

    let played_cards: Vec<(Player, std::sync::Arc<str>)> = state
        .current_round
        .plays
        .iter()
        .filter(|play| is_player(play.player))
        .map(|play| (play.player, play.card.id.clone()))
        .collect();
    for (player, card_id) in played_cards {
        let hand = player_hand_mut(&mut state, player);
        if let Some(index) = hand.iter().position(|card| card.id == card_id) {
            hand.remove(index);
        }
    }
    state
}

pub(crate) fn player_hand_mut(
    state: &mut GameState,
    player: Player,
) -> &mut smallvec::SmallVec<[Card; 3]> {
    match player {
        0 => &mut state.hands.zero,
        1 => &mut state.hands.one,
        _ => unreachable!("player ids are validated to 0|1 before hand access"),
    }
}

pub(crate) fn validate_normalized_state(state: &GameState) -> Result<(), EngineError> {
    validate_shared_normalized_state(state)?;

    // A live hand cannot exist once the match is decided.
    if score_winner(&state.score).is_some() {
        return Err(EngineError::InvalidInitialState);
    }

    if both_players_on_eleven(&state.score) {
        if state.hand_value != starting_hand_value(&state.score) {
            return Err(EngineError::InvalidInitialState);
        }
        if state.pending_decision.is_some() {
            return Err(EngineError::InvalidInitialState);
        }
    }

    // Raises are forbidden for the whole hand whenever any player is on 11,
    // and a one-player-at-11 hand is worth 1 only while the decision is open
    // (accepting makes it 3; folding ends it).
    if score_triggers_eleven_hand(&state.score) && state.pending_raise.is_some() {
        return Err(EngineError::InvalidInitialState);
    }
    if eleven_hand_decision_player(&state.score).is_some() {
        let expected_value = if state.pending_decision.is_some() {
            INITIAL_HAND_VALUE
        } else {
            ELEVEN_AUTO_HAND_VALUE
        };
        if state.hand_value != expected_value {
            return Err(EngineError::InvalidInitialState);
        }
    }

    let expected_leader = expected_round_leader(state.dealer, &state.completed_rounds);
    if state.current_round.leader != expected_leader {
        return Err(EngineError::InvalidInitialState);
    }

    if let Some(play) = state.current_round.plays.first() {
        if play.player != state.current_round.leader {
            return Err(EngineError::InvalidInitialState);
        }
    }

    if let Some(pending_raise) = &state.pending_raise {
        if !raise_value_reachable_from(state.hand_value, pending_raise.previous_value) {
            return Err(EngineError::InvalidInitialState);
        }
        if !is_player(pending_raise.raised_by) {
            return Err(EngineError::InvalidInitialState);
        }
        if state.last_raised_by != Some(pending_raise.raised_by) {
            return Err(EngineError::InvalidInitialState);
        }
        let expected_to = next_raise_after(pending_raise.previous_value)
            .ok_or(EngineError::InvalidInitialState)?;
        if pending_raise.to != expected_to {
            return Err(EngineError::InvalidInitialState);
        }
    } else if state.hand_value == 1 {
        if state.last_raised_by.is_some() {
            return Err(EngineError::InvalidInitialState);
        }
    } else if state.last_raised_by.is_none() && !score_triggers_eleven_hand(&state.score) {
        return Err(EngineError::InvalidInitialState);
    }

    if let Some(decision) = &state.pending_decision {
        let expected_decision_player =
            eleven_hand_decision_player(&state.score).ok_or(EngineError::InvalidInitialState)?;
        if decision.player != expected_decision_player
            || !state.completed_rounds.is_empty()
            || !state.current_round.plays.is_empty()
        {
            return Err(EngineError::InvalidInitialState);
        }
    }

    if decide_hand_winner(
        &state.completed_rounds,
        first_round_leader(state.dealer, &state.completed_rounds),
    )
    .is_some()
    {
        return Err(EngineError::HandAlreadyDecided);
    }

    let expected_next_player = if let Some(decision) = &state.pending_decision {
        Some(decision.player)
    } else if let Some(pending_raise) = &state.pending_raise {
        Some(other_player(pending_raise.raised_by))
    } else if state.current_round.plays.is_empty() {
        Some(state.current_round.leader)
    } else {
        Some(other_player(state.current_round.plays[0].player))
    };

    if state.next_player != expected_next_player {
        return Err(EngineError::InvalidInitialState);
    }

    Ok(())
}

fn validate_finished_normalized_state(
    state: &GameState,
    hand_winner: Player,
    match_winner: Option<Player>,
) -> Result<(), EngineError> {
    validate_shared_normalized_state(state)?;

    if !is_player(hand_winner) || state.next_player.is_some() {
        return Err(EngineError::InvalidInitialState);
    }
    if state.pending_decision.is_some() || state.pending_raise.is_some() {
        return Err(EngineError::InvalidInitialState);
    }

    let expected_leader = expected_round_leader(state.dealer, &state.completed_rounds);
    if state.current_round.leader != expected_leader {
        return Err(EngineError::InvalidInitialState);
    }

    if let Some(play) = state.current_round.plays.first() {
        if play.player != state.current_round.leader {
            return Err(EngineError::InvalidInitialState);
        }
    }

    if let Some(resolved_winner) = decide_hand_winner(
        &state.completed_rounds,
        first_round_leader(state.dealer, &state.completed_rounds),
    ) {
        if resolved_winner != hand_winner {
            return Err(EngineError::InvalidInitialState);
        }
    }

    if match_winner != score_winner(&state.score) {
        return Err(EngineError::InvalidInitialState);
    }

    Ok(())
}

fn validate_shared_normalized_state(state: &GameState) -> Result<(), EngineError> {
    if !is_player(state.dealer)
        || !is_player(state.current_round.leader)
        || state.next_player.is_some_and(|player| !is_player(player))
        || state
            .last_raised_by
            .is_some_and(|player| !is_player(player))
    {
        return Err(EngineError::InvalidInitialState);
    }
    if state.score.zero > MATCH_TARGET || state.score.one > MATCH_TARGET {
        return Err(EngineError::InvalidInitialState);
    }
    if !RAISE_LADDER.contains(&state.hand_value) {
        return Err(EngineError::InvalidInitialState);
    }
    if state.completed_rounds.len() > 3 || state.current_round.plays.len() > 1 {
        return Err(EngineError::InvalidInitialState);
    }
    if state.pending_decision.is_some() && state.pending_raise.is_some() {
        return Err(EngineError::InvalidInitialState);
    }
    if state
        .current_round
        .plays
        .iter()
        .any(|play| !is_player(play.player))
    {
        return Err(EngineError::InvalidInitialState);
    }
    if state.completed_rounds.iter().any(|round| {
        !is_player(round.leader)
            || round.winner.is_some_and(|winner| !is_player(winner))
            || !valid_completed_round_history(round, state)
    }) {
        return Err(EngineError::InvalidInitialState);
    }
    validate_round_leader_chain(state)?;
    if state.completed_rounds.is_empty()
        && state
            .current_round
            .plays
            .iter()
            .any(|play| matches!(play.visibility, Visibility::Down))
    {
        return Err(EngineError::InvalidInitialState);
    }
    if has_duplicate_physical_cards(state) || has_duplicate_card_ids(state) {
        return Err(EngineError::InvalidInitialState);
    }
    validate_card_counts(state)?;

    Ok(())
}

/// Each round's leader is forced by the rules: the dealer's opponent leads
/// round 1 (the dealer is pé), and afterwards the previous round's winner —
/// or its leader, on a tie — leads. `Engine::from_state` derives
/// `first_round_leader` from this history, and that value decides
/// all-three-tied hands, so a corrupted chain corrupts hand resolution.
fn validate_round_leader_chain(state: &GameState) -> Result<(), EngineError> {
    if let Some(first) = state.completed_rounds.first() {
        if first.leader != other_player(state.dealer) {
            return Err(EngineError::InvalidInitialState);
        }
    }
    for pair in state.completed_rounds.windows(2) {
        if pair[1].leader != pair[0].winner.unwrap_or(pair[0].leader) {
            return Err(EngineError::InvalidInitialState);
        }
    }
    Ok(())
}

/// Card-count conservation: every completed round consumed exactly one card
/// per player and a current-round play consumes one more, so each hand must
/// hold exactly `3 - completed - played_now` cards. (Folded hands keep their
/// remaining cards, so this holds for finished states too.)
fn validate_card_counts(state: &GameState) -> Result<(), EngineError> {
    for player in [0u8, 1u8] {
        let played_now = state
            .current_round
            .plays
            .iter()
            .filter(|play| play.player == player)
            .count();
        let expected = 3usize
            .checked_sub(state.completed_rounds.len())
            .and_then(|left| left.checked_sub(played_now))
            .ok_or(EngineError::InvalidInitialState)?;
        if state.hands.player(player).len() != expected {
            return Err(EngineError::InvalidInitialState);
        }
    }
    Ok(())
}

fn all_physical_cards(state: &GameState) -> impl Iterator<Item = &Card> {
    state
        .hands
        .zero
        .iter()
        .chain(state.hands.one.iter())
        .chain(
            state
                .completed_rounds
                .iter()
                .flat_map(|round| round.plays.iter().map(|play| &play.card)),
        )
        .chain(state.current_round.plays.iter().map(|play| &play.card))
}

fn has_duplicate_physical_cards(state: &GameState) -> bool {
    let mut seen = vec![(state.turnup.rank, state.turnup.suit)];
    for card in all_physical_cards(state) {
        let key = (card.rank, card.suit);
        if seen.contains(&key) {
            return true;
        }
        seen.push(key);
    }
    false
}

/// Card ids must be unique: `apply_play` finds cards by id, so a duplicated
/// id makes action application ambiguous.
fn has_duplicate_card_ids(state: &GameState) -> bool {
    let mut seen: Vec<&str> = Vec::new();
    for card in all_physical_cards(state) {
        if seen.contains(&card.id.as_ref()) {
            return true;
        }
        seen.push(&card.id);
    }
    false
}

fn is_player(player: Player) -> bool {
    matches!(player, 0 | 1)
}

fn raise_value_reachable_from(current: u8, target: u8) -> bool {
    let mut step = Some(current);

    while let Some(value) = step {
        if value == target {
            return true;
        }
        step = next_raise_after(value);
    }

    false
}

fn valid_completed_round_history(round: &crate::CompletedRound, state: &GameState) -> bool {
    if round.plays.is_empty() {
        return true;
    }
    if round.plays.len() != 2 {
        return false;
    }
    let first = &round.plays[0];
    let second = &round.plays[1];
    first.player == round.leader
        && is_player(first.player)
        && is_player(second.player)
        && first.player != second.player
        && compare_plays(&state.turnup, first, second) == round.winner
}
