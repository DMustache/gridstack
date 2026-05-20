use serde::{Deserialize, Serialize};

use crate::services::rooms::{
    entities::{GetRoomMembersCommand, RoomMembersChunk, RoomMembershipFilter, RoomStateEventView},
    errors::RoomsApplicationError,
};

#[derive(Clone, Debug, Deserialize, Default)]
pub struct GetRoomMembersQuery {
    #[serde(rename = "at")]
    pub at_token: Option<String>,
    pub membership: Option<String>,
    pub not_membership: Option<String>,
}

impl TryFrom<GetRoomMembersQuery> for GetRoomMembersCommand {
    type Error = RoomsApplicationError;

    fn try_from(value: GetRoomMembersQuery) -> Result<Self, Self::Error> {
        let membership = parse_membership_filter(value.membership.as_deref())?;
        let not_membership = parse_membership_filter(value.not_membership.as_deref())?;

        Ok(Self {
            at_token: value.at_token,
            membership,
            not_membership,
        })
    }
}

fn parse_membership_filter(
    value: Option<&str>,
) -> Result<Option<RoomMembershipFilter>, RoomsApplicationError> {
    value.map_or(Ok(None), |raw_value| {
        RoomMembershipFilter::parse(raw_value)
            .map(Some)
            .ok_or(RoomsApplicationError::InvalidParameter)
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct GetRoomMembersView {
    pub chunk: Vec<RoomStateEventView>,
}

impl From<RoomMembersChunk> for GetRoomMembersView {
    fn from(value: RoomMembersChunk) -> Self {
        Self {
            chunk: value
                .chunk
                .into_iter()
                .map(RoomStateEventView::from)
                .collect(),
        }
    }
}
