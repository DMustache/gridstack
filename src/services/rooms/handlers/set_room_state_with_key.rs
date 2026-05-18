use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct SetRoomStateWithKeyView {
    pub event_id: String,
}
