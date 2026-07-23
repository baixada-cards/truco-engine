use crate::{
    engine_types::{
        PublicCard, PublicCompletedRound, PublicDecision, PublicPlay, PublicRound, PublicState,
    },
    match_types::{EngineState, PlayerHandView},
    rules::{
        compare_plays, decide_hand_winner, eleven_hand_decision_player, first_round_leader,
        next_raise_after, other_player, score_triggers_eleven_hand, score_winner,
        starting_hand_value, ELEVEN_AUTO_HAND_VALUE, ELEVEN_FOLD_AWARD,
    },
    state::Visibility,
    validation::{
        normalize_state, player_hand_mut, validate_exported_engine_state, validate_normalized_state,
    },
    Action, CompletedRound, CurrentRound, EngineError, GameState, Hands, PendingDecision,
    PendingDecisionKind, PendingRaise, PlayedCard, Player, Score, Turnup, MATCH_TARGET,
};
use serde_json::Value;
use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub struct Engine {
    state: GameState,
    hand_winner: Option<Player>,
    match_winner: Option<Player>,
    first_round_leader: Player,
}

impl Engine {
    pub fn new_hand(
        dealer: Player,
        score: Score,
        turnup: Turnup,
        hands: Hands,
    ) -> Result<Self, EngineError> {
        let leader = other_player(dealer);
        let hand_value = starting_hand_value(&score);
        let pending_decision = eleven_hand_decision_player(&score).map(|player| PendingDecision {
            kind: PendingDecisionKind::MaoDeOnze,
            player,
        });
        let next_player = pending_decision
            .as_ref()
            .map(|decision| decision.player)
            .unwrap_or(leader);

        Self::from_state(GameState {
            dealer,
            next_player: Some(next_player),
            score,
            hand_value,
            turnup,
            hands,
            completed_rounds: SmallVec::new(),
            current_round: CurrentRound {
                leader,
                plays: SmallVec::new(),
            },
            last_raised_by: None,
            pending_raise: None,
            pending_decision,
        })
    }

    /// Build an engine over a live (undecided) hand state.
    ///
    /// The input is normalized before validation: cards that appear both in a
    /// hand and in the current round's plays are removed from the hand, and
    /// `last_raised_by` is inferred from `pending_raise` (or cleared when the
    /// hand is still at its initial value with no raise pending). The state
    /// observable through [`Engine::state`] is the normalized one.
    pub fn from_state(state: GameState) -> Result<Self, EngineError> {
        let state = normalize_state(state);
        validate_normalized_state(&state)?;
        let first_round_leader = first_round_leader(state.dealer, &state.completed_rounds);
        Ok(Self {
            state,
            hand_winner: None,
            match_winner: None,
            first_round_leader,
        })
    }

    pub fn from_exported_state(exported: EngineState) -> Result<Self, EngineError> {
        let state = normalize_state(exported.state);
        validate_exported_engine_state(&state, exported.hand_winner, exported.match_winner)?;
        let match_winner = score_winner(&state.score);
        if exported.match_winner != match_winner {
            return Err(EngineError::InvalidInitialState);
        }

        Ok(Self {
            first_round_leader: first_round_leader(state.dealer, &state.completed_rounds),
            state,
            hand_winner: exported.hand_winner,
            match_winner,
        })
    }

    pub fn legal_actions(&self, player: Player) -> Result<Vec<Action>, EngineError> {
        self.ensure_active_turn(player)?;

        if let Some(decision) = &self.state.pending_decision {
            if decision.player != player {
                return Err(EngineError::ActOutOfTurn);
            }
            return Ok(vec![
                Action::AcceptEleven,
                Action::FoldEleven,
                Action::ConcedeHand,
            ]);
        }

        if let Some(pending_raise) = &self.state.pending_raise {
            let mut actions = vec![Action::AcceptRaise, Action::Fold, Action::ConcedeHand];
            if self.re_raise_available(player) {
                let next_value = next_raise_after(pending_raise.to)
                    .expect("re-raise should have a next ladder step");
                actions.push(Action::Raise { to: next_value });
            }
            return Ok(actions);
        }

        let mut actions = Vec::new();
        for card in self.state.hands.player(player) {
            actions.push(Action::PlayFaceUp {
                card_id: card.id.clone(),
            });
            if self.hidden_play_allowed() {
                actions.push(Action::PlayFaceDown {
                    card_id: card.id.clone(),
                });
            }
        }

        if self.raise_available(player) {
            if let Some(next_value) = next_raise_after(self.state.hand_value) {
                actions.push(Action::Raise { to: next_value });
            }
        }
        actions.push(Action::ConcedeHand);

        Ok(actions)
    }

    pub fn strategic_legal_actions(&self, player: Player) -> Result<Vec<Action>, EngineError> {
        Ok(filter_strategic_actions(self.legal_actions(player)?))
    }

    pub fn current_player(&self) -> Option<Player> {
        self.state.next_player
    }

    pub fn hand_winner(&self) -> Option<Player> {
        self.hand_winner
    }

    pub fn match_winner(&self) -> Option<Player> {
        self.match_winner
    }

    pub fn is_hand_over(&self) -> bool {
        self.hand_winner.is_some()
    }

    pub fn is_match_over(&self) -> bool {
        self.match_winner.is_some()
    }

    pub fn legal_actions_for_current_player(&self) -> Result<Vec<Action>, EngineError> {
        let player = self
            .current_player()
            .ok_or(EngineError::HandAlreadyDecided)?;
        self.legal_actions(player)
    }

    pub fn strategic_legal_actions_for_current_player(&self) -> Result<Vec<Action>, EngineError> {
        Ok(filter_strategic_actions(
            self.legal_actions_for_current_player()?,
        ))
    }

    pub fn apply_action_for_current_player(
        &mut self,
        action: &Action,
    ) -> Result<Player, EngineError> {
        let player = self
            .current_player()
            .ok_or(EngineError::HandAlreadyDecided)?;
        self.apply_action(player, action)?;
        Ok(player)
    }

    pub fn apply_action(&mut self, player: Player, action: &Action) -> Result<(), EngineError> {
        self.ensure_active_turn(player)?;

        match action {
            Action::PlayFaceUp { card_id } => self.apply_play(player, card_id, Visibility::Up),
            Action::PlayFaceDown { card_id } => self.apply_play(player, card_id, Visibility::Down),
            Action::Raise { to } => self.apply_raise(player, *to),
            Action::AcceptRaise => self.apply_accept_raise(),
            Action::Fold => self.apply_fold(),
            Action::AcceptEleven => self.apply_accept_eleven(),
            Action::FoldEleven => self.apply_fold_eleven(),
            Action::ConcedeHand => self.apply_concede_hand(player),
        }
    }

    pub fn public_state(&self) -> PublicState {
        PublicState {
            next_player: self.state.next_player,
            hand_value: self.state.hand_value,
            hand_winner: self.hand_winner,
            match_winner: self.match_winner,
            score: self.state.score.clone(),
            completed_rounds: self
                .state
                .completed_rounds
                .iter()
                .map(|round| PublicCompletedRound {
                    leader: round.leader,
                    winner: round.winner,
                })
                .collect(),
            current_round: PublicRound {
                leader: self.state.current_round.leader,
                plays: self
                    .state
                    .current_round
                    .plays
                    .iter()
                    .map(|play| PublicPlay {
                        player: play.player,
                        visibility: play.visibility,
                        card: match play.visibility {
                            Visibility::Up => Some(PublicCard {
                                rank: play.card.rank,
                                suit: play.card.suit,
                            }),
                            Visibility::Down => None,
                        },
                    })
                    .collect(),
            },
            pending_raise: self.state.pending_raise.clone(),
            pending_decision: self
                .state
                .pending_decision
                .as_ref()
                .map(|decision| PublicDecision {
                    kind: decision.kind,
                    player: decision.player,
                }),
        }
    }

    pub fn public_state_value(&self) -> Result<Value, EngineError> {
        serde_json::to_value(self.public_state()).map_err(|_| EngineError::SerializationFailed)
    }

    pub fn player_view(&self, player: Player) -> Result<PlayerHandView, EngineError> {
        if !matches!(player, 0 | 1) {
            return Err(EngineError::InvalidPlayer);
        }

        Ok(PlayerHandView {
            public_state: self.public_state(),
            hand: self.state.hands.player(player).to_vec(),
        })
    }

    pub fn export_state(&self) -> EngineState {
        EngineState {
            state: self.state.clone(),
            hand_winner: self.hand_winner,
            match_winner: self.match_winner,
        }
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    fn ensure_active_turn(&self, player: Player) -> Result<(), EngineError> {
        if self.hand_winner.is_some() {
            return Err(EngineError::HandAlreadyDecided);
        }

        match self.state.next_player {
            Some(expected) if expected == player => Ok(()),
            _ => Err(EngineError::ActOutOfTurn),
        }
    }

    fn hidden_play_allowed(&self) -> bool {
        !self.state.completed_rounds.is_empty()
    }

    fn raise_available(&self, player: Player) -> bool {
        self.state.pending_raise.is_none()
            && self.state.pending_decision.is_none()
            && self.state.hand_value < MATCH_TARGET
            && !score_triggers_eleven_hand(&self.state.score)
            && self.state.last_raised_by != Some(player)
    }

    fn re_raise_available(&self, player: Player) -> bool {
        self.state
            .pending_raise
            .as_ref()
            .is_some_and(|pending_raise| {
                next_raise_after(pending_raise.to).is_some()
                    && pending_raise.raised_by != player
                    && self.state.last_raised_by != Some(player)
            })
    }

    fn apply_play(
        &mut self,
        player: Player,
        card_id: &str,
        visibility: Visibility,
    ) -> Result<(), EngineError> {
        if self.state.pending_decision.is_some() {
            return Err(EngineError::DecisionPending);
        }
        if self.state.pending_raise.is_some() {
            return Err(EngineError::RaisePending);
        }
        if matches!(visibility, Visibility::Down) && !self.hidden_play_allowed() {
            return Err(EngineError::HideNotAllowedInFirstRound);
        }

        let hand = player_hand_mut(&mut self.state, player);
        let index = hand
            .iter()
            .position(|card| card.id.as_ref() == card_id)
            .ok_or(EngineError::CardNotInHand)?;
        let card = hand.remove(index);
        self.state.current_round.plays.push(PlayedCard {
            player,
            visibility,
            card,
        });

        if self.state.current_round.plays.len() == 1 {
            self.state.next_player = Some(other_player(player));
            return Ok(());
        }

        self.resolve_round();
        Ok(())
    }

    fn apply_raise(&mut self, player: Player, to: u8) -> Result<(), EngineError> {
        if self.state.pending_decision.is_some() {
            return Err(EngineError::RaiseNotAllowed);
        }
        if score_triggers_eleven_hand(&self.state.score) {
            return Err(EngineError::RaiseNotAllowed);
        }

        // A re-raise implicitly accepts the pending value, so it escalates
        // from `pending_raise.to`; a fresh raise escalates from `hand_value`.
        let (available, escalates_from) = match &self.state.pending_raise {
            Some(pending_raise) => (self.re_raise_available(player), pending_raise.to),
            None => (self.raise_available(player), self.state.hand_value),
        };
        if !available {
            return Err(EngineError::RaiseNotAllowed);
        }
        let Some(expected) = next_raise_after(escalates_from) else {
            return Err(EngineError::RaiseNotAllowed);
        };
        if to != expected {
            return Err(EngineError::InvalidRaiseTarget);
        }

        self.state.pending_raise = Some(PendingRaise {
            raised_by: player,
            to,
            previous_value: escalates_from,
        });
        self.state.last_raised_by = Some(player);
        self.state.next_player = Some(other_player(player));
        Ok(())
    }

    fn apply_accept_raise(&mut self) -> Result<(), EngineError> {
        let pending_raise = self
            .state
            .pending_raise
            .take()
            .ok_or(EngineError::NoPendingRaise)?;
        self.state.hand_value = pending_raise.to;
        self.state.next_player = if self.state.current_round.plays.is_empty() {
            Some(self.state.current_round.leader)
        } else {
            Some(other_player(self.state.current_round.plays[0].player))
        };
        Ok(())
    }

    fn apply_fold(&mut self) -> Result<(), EngineError> {
        let pending_raise = self
            .state
            .pending_raise
            .take()
            .ok_or(EngineError::NoPendingRaise)?;
        self.end_hand(pending_raise.raised_by, pending_raise.previous_value);
        Ok(())
    }

    fn apply_accept_eleven(&mut self) -> Result<(), EngineError> {
        self.state
            .pending_decision
            .take()
            .ok_or(EngineError::NoPendingElevenDecision)?;
        self.state.hand_value = ELEVEN_AUTO_HAND_VALUE;
        self.state.next_player = Some(self.state.current_round.leader);
        Ok(())
    }

    fn apply_fold_eleven(&mut self) -> Result<(), EngineError> {
        let decision = self
            .state
            .pending_decision
            .take()
            .ok_or(EngineError::NoPendingElevenDecision)?;
        self.end_hand(other_player(decision.player), ELEVEN_FOLD_AWARD);
        Ok(())
    }

    fn apply_concede_hand(&mut self, player: Player) -> Result<(), EngineError> {
        // A pending re-raise implicitly accepted `previous_value`, so a
        // concession pays the same as a fold there — never less.
        let awarded = self
            .state
            .pending_raise
            .as_ref()
            .map(|pending_raise| pending_raise.previous_value)
            .unwrap_or(self.state.hand_value);
        self.end_hand(other_player(player), awarded);
        Ok(())
    }

    fn resolve_round(&mut self) {
        let plays = &self.state.current_round.plays;
        let winner = compare_plays(&self.state.turnup, &plays[0], &plays[1]);
        let leader = self.state.current_round.leader;
        self.state.completed_rounds.push(CompletedRound {
            leader,
            winner,
            plays: plays.clone(),
        });
        self.state.current_round.plays.clear();
        self.state.current_round.leader = winner.unwrap_or(leader);

        if let Some(hand_winner) =
            decide_hand_winner(&self.state.completed_rounds, self.first_round_leader)
        {
            self.end_hand(hand_winner, self.state.hand_value);
            return;
        }

        self.state.next_player = Some(self.state.current_round.leader);
    }

    fn end_hand(&mut self, winner: Player, awarded_value: u8) {
        self.hand_winner = Some(winner);
        self.state.pending_raise = None;
        self.state.pending_decision = None;
        self.state.next_player = None;

        self.state.score.award(winner, awarded_value);
        self.match_winner = score_winner(&self.state.score);
    }
}

fn filter_strategic_actions(actions: Vec<Action>) -> Vec<Action> {
    actions.into_iter().filter(Action::is_strategic).collect()
}
