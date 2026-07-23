//! Rust engine crate for the 2-player Truco ruleset.

pub mod actions;
pub mod api;
pub mod bot_analysis;
pub mod cards;
pub mod engine;
mod engine_types;
pub mod error;
pub mod exploration;
pub mod fixture_runner;
pub mod fixtures;
pub mod game_match;
mod json_subset;
mod match_types;
mod rules;
pub mod state;
mod validation;

pub use actions::Action;
pub use api::{ActionResult, ApiMatch, HandStart, MatchConfig};
pub use bot_analysis::{
    analyze_tactical_turn, analyze_tactical_turn_sampled, BotAnalysisError, TacticalActionSummary,
    TacticalTurnAnalysis,
};
pub use cards::{Card, Rank, Suit, Turnup};
pub use engine::Engine;
pub use engine_types::{
    PublicCard, PublicCompletedRound, PublicDecision, PublicPlay, PublicRound, PublicState,
};
pub use error::EngineError;
pub use exploration::{
    resolve_match_spec, CardConstraint, CardFace, CompletedHiddenPlayOverride, ExplorationError,
    ExplorationMatchSpec, HandPattern, HiddenPlayRangeSpec, HiddenPlayRanges, HiddenRanges,
    PendingHandSpec, PlayerRangeSpec, WeightedCardOption, WeightedHandOption,
};
pub use fixture_runner::{execute_fixture, FixtureRunReport, FixtureRunStatus};
pub use fixtures::{EngineFixture, FixtureStep};
pub use game_match::Match;
pub use json_subset::assert_expected_subset;
pub use match_types::{EngineState, MatchSnapshot, MatchState, PlayerHandView, PlayerMatchView};
pub use state::{
    CompletedRound, CurrentRound, GameState, Hands, PendingDecision, PendingDecisionKind,
    PendingRaise, PlayedCard, Player, Score,
};
pub use validation::validate_state;

pub const MATCH_TARGET: u8 = 12;
pub const INITIAL_HAND_VALUE: u8 = 1;
pub const RAISE_LADDER: [u8; 5] = [1, 3, 6, 9, 12];
