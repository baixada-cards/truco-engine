use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Canonical action contract shared across implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "play_face_up")]
    PlayFaceUp { card_id: Arc<str> },
    #[serde(rename = "play_face_down")]
    PlayFaceDown { card_id: Arc<str> },
    #[serde(rename = "raise")]
    Raise { to: u8 },
    #[serde(rename = "accept_raise")]
    AcceptRaise,
    #[serde(rename = "fold")]
    Fold,
    #[serde(rename = "accept_eleven")]
    AcceptEleven,
    #[serde(rename = "fold_eleven")]
    FoldEleven,
    #[serde(rename = "concede_hand")]
    ConcedeHand,
}

impl Action {
    pub fn is_strategic(&self) -> bool {
        !matches!(self, Self::ConcedeHand)
    }
}
