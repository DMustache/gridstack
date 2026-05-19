use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct SendRoomEventView {
    pub event_id: String,
}
