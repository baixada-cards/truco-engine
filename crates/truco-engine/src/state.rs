use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::cards::{Card, Turnup};

pub type Player = u8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Score {
    #[serde(rename = "0")]
    pub zero: u8,
    #[serde(rename = "1")]
    pub one: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletedRound {
    pub leader: Player,
    pub winner: Option<Player>,
    #[serde(default, skip_serializing_if = "SmallVec::is_empty")]
    pub plays: SmallVec<[PlayedCard; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayedCard {
    pub player: Player,
    pub visibility: Visibility,
    pub card: Card,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentRound {
    pub leader: Player,
    pub plays: SmallVec<[PlayedCard; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRaise {
    pub raised_by: Player,
    pub to: u8,
    pub previous_value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendingDecisionKind {
    #[serde(rename = "mao_de_onze")]
    MaoDeOnze,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDecision {
    #[serde(rename = "type")]
    pub kind: PendingDecisionKind,
    pub player: Player,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub dealer: Player,
    pub next_player: Option<Player>,
    pub score: Score,
    pub hand_value: u8,
    pub turnup: Turnup,
    pub hands: Hands,
    pub completed_rounds: SmallVec<[CompletedRound; 3]>,
    pub current_round: CurrentRound,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_raised_by: Option<Player>,
    pub pending_raise: Option<PendingRaise>,
    #[serde(default)]
    pub pending_decision: Option<PendingDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hands {
    #[serde(rename = "0")]
    pub zero: SmallVec<[Card; 3]>,
    #[serde(rename = "1")]
    pub one: SmallVec<[Card; 3]>,
}

impl Hands {
    pub fn player(&self, player: Player) -> &[Card] {
        match player {
            0 => &self.zero,
            1 => &self.one,
            _ => &[],
        }
    }
}

impl Score {
    /// Award points to a player, saturating at [`crate::MATCH_TARGET`].
    ///
    /// Panics if `player` is not `0` or `1`; engine call sites only pass
    /// players that already went through turn validation.
    pub fn award(&mut self, player: Player, points: u8) {
        let entry = match player {
            0 => &mut self.zero,
            1 => &mut self.one,
            _ => unreachable!("player ids are validated to 0|1 before scoring"),
        };
        *entry = entry.saturating_add(points).min(crate::MATCH_TARGET);
    }
}
