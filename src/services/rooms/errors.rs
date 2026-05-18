use thiserror::Error;

use crate::services::{
    events::service::EventCompilationError, rooms::entities::RoomValidationError,
};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RoomsApplicationError {
    #[error("missing or invalid access token")]
    Unauthorized,
    #[error("room alias is already in use")]
    RoomInUse,
    #[error("requested room version is unsupported")]
    UnsupportedRoomVersion,
    #[error("invalid room state")]
    InvalidRoomState,
    #[error("invalid request parameter")]
    InvalidParameter,
    #[error("invite blocked")]
    InviteBlocked,
    #[error("forbidden")]
    Forbidden,
    #[error("internal error")]
    Internal,
}

impl From<RoomValidationError> for RoomsApplicationError {
    fn from(value: RoomValidationError) -> Self {
        match value {
            RoomValidationError::UnsupportedRoomVersion { .. } => Self::UnsupportedRoomVersion,
            RoomValidationError::InvalidInitialStateEventType
            | RoomValidationError::InvalidInitialStateEventContent { .. }
            | RoomValidationError::InitialStateContainsRoomCreate => Self::InvalidRoomState,
            RoomValidationError::InvalidInviteUserIdentifier { .. }
            | RoomValidationError::InvalidRoomAlias
            | RoomValidationError::InvalidRoomName
            | RoomValidationError::InvalidRoomTopic
            | RoomValidationError::InvalidThirdPartyInvite
            | RoomValidationError::InvalidCreationContent
            | RoomValidationError::InvalidPredecessor => Self::InvalidParameter,
        }
    }
}

impl From<EventCompilationError> for RoomsApplicationError {
    fn from(value: EventCompilationError) -> Self {
        match value {
            EventCompilationError::UnsupportedRoomVersion { .. } => Self::UnsupportedRoomVersion,
            EventCompilationError::EmptyIntentPlan { .. }
            | EventCompilationError::InvalidEventType
            | EventCompilationError::InvalidEventContentShape { .. }
            | EventCompilationError::MissingStateKeyForStateEvent { .. }
            | EventCompilationError::MessageEventCannotHaveStateKey { .. }
            | EventCompilationError::StateKeyMustBeEmpty { .. }
            | EventCompilationError::InvalidMembershipStateKey { .. }
            | EventCompilationError::CreateEventMustBeFirst
            | EventCompilationError::CreatorJoinMustBeSecond
            | EventCompilationError::MissingSenderMembershipForAuth { .. } => {
                Self::InvalidRoomState
            }
        }
    }
}
