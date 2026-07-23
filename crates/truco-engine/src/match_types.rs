use serde::{Deserialize, Serialize};

use crate::{Card, GameState, Player, PublicState, Score};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineState {
    pub state: GameState,
    pub hand_winner: Option<Player>,
    pub match_winner: Option<Player>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchState {
    pub next_dealer: Player,
    pub score: Score,
    pub winner: Option<Player>,
    pub current_hand: Option<EngineState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchSnapshot {
    pub score: Score,
    pub winner: Option<Player>,
    pub next_dealer: Player,
    pub current_player: Option<Player>,
    pub hand_in_progress: bool,
    pub hand: Option<PublicState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerHandView {
    pub public_state: PublicState,
    pub hand: Vec<Card>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerMatchView {
    pub player: Player,
    pub score: Score,
    pub winner: Option<Player>,
    pub next_dealer: Player,
    pub current_player: Option<Player>,
    pub hand_in_progress: bool,
    pub hand: Option<PlayerHandView>,
}
