use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct SendMessageEventView {
    pub event_id: String,
}
