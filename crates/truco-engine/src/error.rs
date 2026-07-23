use thiserror::Error;

/// Stable machine-readable errors expected by the fixture contract.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    #[error("invalid initial state")]
    InvalidInitialState,
    #[error("hand already decided")]
    HandAlreadyDecided,
    #[error("no active hand")]
    NoActiveHand,
    #[error("hand still in progress")]
    HandStillInProgress,
    #[error("match already decided")]
    MatchAlreadyDecided,
    #[error("act out of turn")]
    ActOutOfTurn,
    #[error("card not in hand")]
    CardNotInHand,
    #[error("hide not allowed in first round")]
    HideNotAllowedInFirstRound,
    #[error("raise not allowed")]
    RaiseNotAllowed,
    #[error("invalid raise target")]
    InvalidRaiseTarget,
    #[error("no pending raise")]
    NoPendingRaise,
    #[error("no pending eleven decision")]
    NoPendingElevenDecision,
    #[error("a pending raise must be answered first")]
    RaisePending,
    #[error("a pending decision must be answered first")]
    DecisionPending,
    #[error("invalid player")]
    InvalidPlayer,
    #[error("serialization failed")]
    SerializationFailed,
}

impl EngineError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInitialState => "INVALID_INITIAL_STATE",
            Self::HandAlreadyDecided => "HAND_ALREADY_DECIDED",
            Self::NoActiveHand => "NO_ACTIVE_HAND",
            Self::HandStillInProgress => "HAND_STILL_IN_PROGRESS",
            Self::MatchAlreadyDecided => "MATCH_ALREADY_DECIDED",
            Self::ActOutOfTurn => "ACT_OUT_OF_TURN",
            Self::CardNotInHand => "CARD_NOT_IN_HAND",
            Self::HideNotAllowedInFirstRound => "HIDE_NOT_ALLOWED_IN_FIRST_ROUND",
            Self::RaiseNotAllowed => "RAISE_NOT_ALLOWED",
            Self::InvalidRaiseTarget => "INVALID_RAISE_TARGET",
            Self::NoPendingRaise => "NO_PENDING_RAISE",
            Self::NoPendingElevenDecision => "NO_PENDING_ELEVEN_DECISION",
            Self::RaisePending => "RAISE_PENDING",
            Self::DecisionPending => "DECISION_PENDING",
            Self::InvalidPlayer => "INVALID_PLAYER",
            Self::SerializationFailed => "SERIALIZATION_FAILED",
        }
    }
}
