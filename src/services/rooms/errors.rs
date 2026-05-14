use thiserror::Error;

#[derive(Debug, Error)]
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
