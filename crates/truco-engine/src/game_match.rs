use crate::{
    match_types::{MatchSnapshot, MatchState, PlayerMatchView},
    rules::{other_player, score_winner},
    Action, Engine, EngineError, Hands, Player, Score, Turnup,
};

#[derive(Debug, Clone)]
pub struct Match {
    next_dealer: Player,
    score: Score,
    current_hand: Option<Engine>,
    winner: Option<Player>,
}

impl Match {
    pub fn from_state(state: MatchState) -> Result<Self, EngineError> {
        if !matches!(state.next_dealer, 0 | 1) {
            return Err(EngineError::InvalidInitialState);
        }

        let winner = score_winner(&state.score);
        if state.winner != winner {
            return Err(EngineError::InvalidInitialState);
        }

        let current_hand = match state.current_hand {
            Some(current_hand) => {
                let engine = Engine::from_exported_state(current_hand)?;
                if engine.state().score != state.score {
                    return Err(EngineError::InvalidInitialState);
                }
                if engine.match_winner() != winner {
                    return Err(EngineError::InvalidInitialState);
                }
                // `next_dealer` only flips once the hand is over, so it must
                // match the embedded hand's dealer while play is live and be
                // the other player once the hand has been decided.
                let expected_dealer = if engine.is_hand_over() {
                    other_player(engine.state().dealer)
                } else {
                    engine.state().dealer
                };
                if state.next_dealer != expected_dealer {
                    return Err(EngineError::InvalidInitialState);
                }
                Some(engine)
            }
            None => None,
        };

        Ok(Self {
            next_dealer: state.next_dealer,
            score: state.score,
            current_hand,
            winner,
        })
    }

    pub fn new(starting_dealer: Player, score: Score) -> Result<Self, EngineError> {
        if !matches!(starting_dealer, 0 | 1) {
            return Err(EngineError::InvalidInitialState);
        }

        Ok(Self {
            next_dealer: starting_dealer,
            winner: score_winner(&score),
            score,
            current_hand: None,
        })
    }

    pub fn score(&self) -> &Score {
        &self.score
    }

    pub fn winner(&self) -> Option<Player> {
        self.winner
    }

    pub fn current_hand(&self) -> Option<&Engine> {
        self.current_hand.as_ref()
    }

    pub fn dealer_for_next_hand(&self) -> Player {
        self.next_dealer
    }

    pub fn start_hand(&mut self, turnup: Turnup, hands: Hands) -> Result<&Engine, EngineError> {
        if self.winner.is_some() {
            return Err(EngineError::MatchAlreadyDecided);
        }
        if self
            .current_hand
            .as_ref()
            .is_some_and(|hand| !hand.is_hand_over())
        {
            return Err(EngineError::HandStillInProgress);
        }

        let hand = Engine::new_hand(self.next_dealer, self.score.clone(), turnup, hands)?;
        Ok(self.current_hand.insert(hand))
    }

    pub fn current_player(&self) -> Option<Player> {
        self.current_hand.as_ref().and_then(Engine::current_player)
    }

    pub fn public_view(&self) -> MatchSnapshot {
        MatchSnapshot {
            score: self.score.clone(),
            winner: self.winner,
            next_dealer: self.next_dealer,
            current_player: self.current_player(),
            hand_in_progress: self
                .current_hand
                .as_ref()
                .is_some_and(|hand| !hand.is_hand_over()),
            hand: self.current_hand.as_ref().map(Engine::public_state),
        }
    }

    pub fn player_view(&self, player: Player) -> Result<PlayerMatchView, EngineError> {
        if !matches!(player, 0 | 1) {
            return Err(EngineError::InvalidPlayer);
        }

        Ok(PlayerMatchView {
            player,
            score: self.score.clone(),
            winner: self.winner,
            next_dealer: self.next_dealer,
            current_player: self.current_player(),
            hand_in_progress: self
                .current_hand
                .as_ref()
                .is_some_and(|hand| !hand.is_hand_over()),
            hand: self
                .current_hand
                .as_ref()
                .map(|hand| hand.player_view(player))
                .transpose()?,
        })
    }

    pub fn export_state(&self) -> MatchState {
        MatchState {
            next_dealer: self.next_dealer,
            score: self.score.clone(),
            winner: self.winner,
            current_hand: self.current_hand.as_ref().map(Engine::export_state),
        }
    }

    pub fn legal_actions_for_current_player(&self) -> Result<Vec<Action>, EngineError> {
        self.active_hand()?.legal_actions_for_current_player()
    }

    pub fn strategic_legal_actions_for_current_player(&self) -> Result<Vec<Action>, EngineError> {
        self.active_hand()?
            .strategic_legal_actions_for_current_player()
    }

    pub fn apply_action_for_current_player(
        &mut self,
        action: &Action,
    ) -> Result<Player, EngineError> {
        let acted = self
            .active_hand_mut()?
            .apply_action_for_current_player(action)?;
        self.sync_from_active_hand();
        Ok(acted)
    }

    pub fn legal_actions(&self, player: Player) -> Result<Vec<Action>, EngineError> {
        self.active_hand()?.legal_actions(player)
    }

    pub fn strategic_legal_actions(&self, player: Player) -> Result<Vec<Action>, EngineError> {
        self.active_hand()?.strategic_legal_actions(player)
    }

    pub fn apply_action(&mut self, player: Player, action: &Action) -> Result<(), EngineError> {
        self.active_hand_mut()?.apply_action(player, action)?;
        self.sync_from_active_hand();
        Ok(())
    }

    fn active_hand(&self) -> Result<&Engine, EngineError> {
        self.current_hand.as_ref().ok_or(EngineError::NoActiveHand)
    }

    fn active_hand_mut(&mut self) -> Result<&mut Engine, EngineError> {
        self.current_hand.as_mut().ok_or(EngineError::NoActiveHand)
    }

    fn sync_from_active_hand(&mut self) {
        let Some(hand) = &self.current_hand else {
            return;
        };

        self.score = hand.state().score.clone();
        self.winner = hand.match_winner();

        if hand.is_hand_over() {
            self.next_dealer = other_player(hand.state().dealer);
        }
    }
}
