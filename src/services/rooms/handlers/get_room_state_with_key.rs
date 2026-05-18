use serde::Deserialize;
use serde_json::{Map, Value};

use crate::services::{rooms::errors::RoomsApplicationError, rooms::handlers::RoomStateEventView};

#[derive(Clone, Debug, Deserialize, Default)]
pub struct GetRoomStateWithKeyFormatQuery {
    pub format: Option<String>,
}

impl GetRoomStateWithKeyFormatQuery {
    pub fn request_full_event(&self) -> Result<bool, RoomsApplicationError> {
        match self.format.as_deref() {
            None | Some("content") => Ok(false),
            Some("event") => Ok(true),
            Some(_) => Err(RoomsApplicationError::InvalidParameter),
        }
    }
}

pub fn room_state_event_to_value(view: RoomStateEventView) -> Value {
    let mut value = Map::new();
    value.insert("content".to_owned(), view.content);
    value.insert("event_id".to_owned(), Value::String(view.event_id));
    value.insert(
        "origin_server_ts".to_owned(),
        Value::Number(view.origin_server_ts.into()),
    );
    value.insert("room_id".to_owned(), Value::String(view.room_id));
    value.insert("sender".to_owned(), Value::String(view.sender));
    value.insert("state_key".to_owned(), Value::String(view.state_key));
    value.insert("type".to_owned(), Value::String(view.event_type));
    if let Some(unsigned) = view.unsigned {
        value.insert("unsigned".to_owned(), unsigned);
    }
    Value::Object(value)
}
