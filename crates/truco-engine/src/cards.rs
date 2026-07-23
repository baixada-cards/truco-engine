use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Rank ordering follows the agreed non-manilha fixture order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rank {
    #[serde(rename = "4")]
    Four,
    #[serde(rename = "5")]
    Five,
    #[serde(rename = "6")]
    Six,
    #[serde(rename = "7")]
    Seven,
    #[serde(rename = "Q")]
    Queen,
    #[serde(rename = "J")]
    Jack,
    #[serde(rename = "K")]
    King,
    #[serde(rename = "A")]
    Ace,
    #[serde(rename = "2")]
    Two,
    #[serde(rename = "3")]
    Three,
}

impl Rank {
    pub fn index(self) -> usize {
        match self {
            Self::Four => 0,
            Self::Five => 1,
            Self::Six => 2,
            Self::Seven => 3,
            Self::Queen => 4,
            Self::Jack => 5,
            Self::King => 6,
            Self::Ace => 7,
            Self::Two => 8,
            Self::Three => 9,
        }
    }

    pub fn next_for_manilha(self) -> Self {
        match self {
            Self::Four => Self::Five,
            Self::Five => Self::Six,
            Self::Six => Self::Seven,
            Self::Seven => Self::Queen,
            Self::Queen => Self::Jack,
            Self::Jack => Self::King,
            Self::King => Self::Ace,
            Self::Ace => Self::Two,
            Self::Two => Self::Three,
            Self::Three => Self::Four,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    #[serde(rename = "DIAMONDS")]
    Diamonds,
    #[serde(rename = "SPADES")]
    Spades,
    #[serde(rename = "HEARTS")]
    Hearts,
    #[serde(rename = "CLUBS")]
    Clubs,
}

impl Suit {
    pub fn manilha_strength(self) -> usize {
        match self {
            Self::Diamonds => 0,
            Self::Spades => 1,
            Self::Hearts => 2,
            Self::Clubs => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Turnup {
    pub rank: Rank,
    pub suit: Suit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    /// Stable identifier used by actions to address the card. `Arc<str>` so
    /// cloning a card (the solver clones an `Engine` per tree node) bumps a
    /// refcount instead of reallocating; serializes as a plain JSON string.
    pub id: Arc<str>,
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub fn is_manilha(&self, turnup: &Turnup) -> bool {
        self.rank == turnup.rank.next_for_manilha()
    }
}
