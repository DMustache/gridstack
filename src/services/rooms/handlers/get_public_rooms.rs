use serde::Deserialize;

use crate::services::rooms::{
    entities::{GetPublicRoomsCommand, PublicRoomsView},
    errors::RoomsApplicationError,
};

#[derive(Clone, Debug, Deserialize, Default)]
pub struct GetPublicRoomsQuery {
    pub limit: Option<i64>,
    pub server: Option<String>,
    pub since: Option<String>,
}

impl TryFrom<GetPublicRoomsQuery> for GetPublicRoomsCommand {
    type Error = RoomsApplicationError;

    fn try_from(value: GetPublicRoomsQuery) -> Result<Self, Self::Error> {
        let limit = match value.limit {
            Some(limit) => {
                usize::try_from(limit).map_err(|_| RoomsApplicationError::InvalidParameter)?
            }
            None => 10,
        };

        Ok(Self {
            limit,
            server: value.server,
            since: value.since,
        })
    }
}

pub type GetPublicRoomsView = PublicRoomsView;
