use thiserror::Error;

use crate::services::rooms::entities::RoomCreateContractError;

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
    #[error("forbidden")]
    Forbidden,
    #[error("internal error")]
    Internal,
}

impl From<RoomCreateContractError> for RoomsApplicationError {
    fn from(error: RoomCreateContractError) -> Self {
        match error {
            RoomCreateContractError::UnsupportedRoomVersion { .. } => Self::UnsupportedRoomVersion,
            RoomCreateContractError::InvalidRoomStateEventType
            | RoomCreateContractError::InvalidRoomStateEventContentType { .. }
            | RoomCreateContractError::InitialStateContainsRoomCreateEvent
            | RoomCreateContractError::EmptyRoomCreationEventPlan
            | RoomCreateContractError::InvalidRoomCreationEventOrder { .. }
            | RoomCreateContractError::InvalidRoomCreateContent { .. } => Self::InvalidRoomState,
            RoomCreateContractError::InvalidHomeServerName { .. }
            | RoomCreateContractError::InvalidGeneratedRoomIdentifier { .. }
            | RoomCreateContractError::InvalidRoomIdentifier(_)
            | RoomCreateContractError::InvalidRoomAlias
            | RoomCreateContractError::InvalidInviteUserIdentifier
            | RoomCreateContractError::InvalidInviteThirdPartyIdentifier
            | RoomCreateContractError::RoomDomainAndCreatorDomainMismatch { .. }
            | RoomCreateContractError::InvalidCreationContent
            | RoomCreateContractError::InvalidPowerLevelContentOverride => Self::InvalidParameter,
        }
    }
}
