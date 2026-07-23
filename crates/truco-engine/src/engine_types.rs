use serde::{Deserialize, Serialize};

use crate::{state::Visibility, PendingDecisionKind, PendingRaise, Player, Score, Suit};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicState {
    pub next_player: Option<Player>,
    pub hand_value: u8,
    pub hand_winner: Option<Player>,
    pub match_winner: Option<Player>,
    pub score: Score,
    pub completed_rounds: Vec<PublicCompletedRound>,
    pub current_round: PublicRound,
    pub pending_raise: Option<PendingRaise>,
    pub pending_decision: Option<PublicDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicCompletedRound {
    pub leader: Player,
    pub winner: Option<Player>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicRound {
    pub leader: Player,
    pub plays: Vec<PublicPlay>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicPlay {
    pub player: Player,
    pub visibility: Visibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<PublicCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicCard {
    pub rank: crate::Rank,
    pub suit: Suit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicDecision {
    #[serde(rename = "type")]
    pub kind: PendingDecisionKind,
    pub player: Player,
}
