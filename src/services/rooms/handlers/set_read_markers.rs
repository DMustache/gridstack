use serde::Serialize;

use crate::services::rooms::entities::{SetReadMarkersCommand, SetReadMarkersRequestDto};

pub use crate::services::rooms::entities::SetReadMarkersRequestDto as SetReadMarkersInfo;

#[derive(Clone, Debug, Default, Serialize)]
pub struct SetReadMarkersView {}

impl From<SetReadMarkersRequestDto> for SetReadMarkersCommand {
    fn from(value: SetReadMarkersRequestDto) -> Self {
        Self {
            fully_read_event_id: value.fully_read_event_id,
            read_event_id: value.read_event_id,
            private_read_event_id: value.private_read_event_id,
        }
    }
}
