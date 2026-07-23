use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Action, EngineError, Hands, Match, MatchSnapshot, MatchState, Player, PlayerMatchView, Score,
    Turnup,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchConfig {
    pub starting_dealer: Player,
    pub score: Score,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandStart {
    pub turnup: Turnup,
    pub hands: Hands,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionResult {
    pub acted_by: Player,
    pub snapshot: MatchSnapshot,
}

#[derive(Debug, Clone)]
pub struct ApiMatch {
    inner: Match,
}

impl ApiMatch {
    pub fn new(config: MatchConfig) -> Result<Self, EngineError> {
        let inner = Match::new(config.starting_dealer, config.score)?;
        Ok(Self { inner })
    }

    pub fn from_state(state: MatchState) -> Result<Self, EngineError> {
        let inner = Match::from_state(state)?;
        Ok(Self { inner })
    }

    pub fn public_view(&self) -> MatchSnapshot {
        self.inner.public_view()
    }

    pub fn export_state(&self) -> MatchState {
        self.inner.export_state()
    }

    pub fn export_state_value(&self) -> serde_json::Result<Value> {
        serde_json::to_value(self.export_state())
    }

    pub fn start_hand(&mut self, hand: HandStart) -> Result<MatchSnapshot, EngineError> {
        self.inner.start_hand(hand.turnup, hand.hands)?;
        Ok(self.public_view())
    }

    pub fn legal_actions_for_current_player(&self) -> Result<Vec<Action>, EngineError> {
        self.inner.legal_actions_for_current_player()
    }

    pub fn legal_actions_for_current_player_value(&self) -> Result<Value, EngineError> {
        let actions = self.legal_actions_for_current_player()?;
        serde_json::to_value(actions).map_err(|_| EngineError::SerializationFailed)
    }

    pub fn player_view(&self, player: Player) -> Result<PlayerMatchView, EngineError> {
        self.inner.player_view(player)
    }

    pub fn player_view_value(&self, player: Player) -> Result<Value, EngineError> {
        let view = self.player_view(player)?;
        serde_json::to_value(view).map_err(|_| EngineError::SerializationFailed)
    }

    pub fn apply_action_for_current_player(
        &mut self,
        action: &Action,
    ) -> Result<ActionResult, EngineError> {
        let acted_by = self.inner.apply_action_for_current_player(action)?;
        Ok(ActionResult {
            acted_by,
            snapshot: self.public_view(),
        })
    }

    pub fn apply_action_for_current_player_value(
        &mut self,
        action: &Action,
    ) -> Result<Value, EngineError> {
        let result = self.apply_action_for_current_player(action)?;
        serde_json::to_value(result).map_err(|_| EngineError::SerializationFailed)
    }
}
