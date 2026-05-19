use serde::{Deserialize, Serialize};

use crate::services::rooms::{
    entities::{
        GetRoomMessagesCommand, RoomEventFilter, RoomMessageDirection, RoomMessagesPage,
        RoomTimelineEventView,
    },
    errors::RoomsApplicationError,
};

#[derive(Clone, Debug, Deserialize, Default)]
pub struct GetRoomMessagesQuery {
    #[serde(rename = "from")]
    pub from_token: Option<String>,
    #[serde(rename = "to")]
    pub to_token: Option<String>,
    pub dir: Option<String>,
    pub limit: Option<i64>,
    pub filter: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Default)]
struct RoomEventFilterPayload {
    pub types: Option<Vec<String>>,
    pub not_types: Option<Vec<String>>,
    pub senders: Option<Vec<String>>,
    pub not_senders: Option<Vec<String>>,
    pub contains_url: Option<bool>,
}

impl TryFrom<GetRoomMessagesQuery> for GetRoomMessagesCommand {
    type Error = RoomsApplicationError;

    fn try_from(value: GetRoomMessagesQuery) -> Result<Self, Self::Error> {
        let direction = match value.dir.as_deref() {
            Some("b") => RoomMessageDirection::Backward,
            Some("f") => RoomMessageDirection::Forward,
            _ => return Err(RoomsApplicationError::InvalidParameter),
        };

        let limit = match value.limit {
            Some(limit) => {
                usize::try_from(limit).map_err(|_| RoomsApplicationError::InvalidParameter)?
            }
            None => 10,
        };

        let filter = value
            .filter
            .as_deref()
            .map(parse_room_event_filter)
            .transpose()?;

        Ok(Self {
            from_token: value.from_token,
            to_token: value.to_token,
            direction,
            limit,
            filter,
        })
    }
}

fn parse_room_event_filter(raw_filter: &str) -> Result<RoomEventFilter, RoomsApplicationError> {
    let parsed_filter: RoomEventFilterPayload =
        serde_json::from_str(raw_filter).map_err(|_| RoomsApplicationError::InvalidParameter)?;

    Ok(RoomEventFilter {
        types: parsed_filter.types,
        not_types: parsed_filter.not_types,
        senders: parsed_filter.senders,
        not_senders: parsed_filter.not_senders,
        contains_url: parsed_filter.contains_url,
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct GetRoomMessagesView {
    pub start: String,
    pub chunk: Vec<RoomTimelineEventView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<RoomTimelineEventView>,
}

impl From<RoomMessagesPage> for GetRoomMessagesView {
    fn from(value: RoomMessagesPage) -> Self {
        Self {
            start: value.start,
            chunk: value
                .chunk
                .into_iter()
                .map(RoomTimelineEventView::from)
                .collect(),
            end: value.end,
            state: value
                .state
                .into_iter()
                .map(RoomTimelineEventView::from)
                .collect(),
        }
    }
}
