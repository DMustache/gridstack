use serde::{Deserialize, Serialize};

use crate::{
    infrastructure::user_identifier::UserIdentifier,
    services::{
        events::entities::{PreviousRoom, RoomEventValidationError},
        rooms::entities::versions::RoomVersion,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCreateEvent {
    pub state_key: String,
    pub content: RoomCreateContent,
}

impl RoomCreateEvent {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        if !self.state_key.is_empty() {
            return Err(RoomEventValidationError::InvalidRoomCreateStateKey {
                received_state_key: self.state_key.clone(),
            });
        }

        self.content.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCreateContent {
    #[serde(default)]
    pub additional_creators: Option<Vec<UserIdentifier>>,
    pub creator: Option<UserIdentifier>,
    #[serde(rename = "m.federate", default = "default_true")]
    pub federate: bool,
    pub predecessor: Option<PreviousRoom>,
    #[serde(default)]
    pub room_version: RoomVersion,
    #[serde(rename = "type")]
    pub room_type: Option<String>,
}

impl RoomCreateContent {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        if self.room_version.supports_additional_creators() {
            if let Some(additional_creators) = self.additional_creators.as_ref()
                && additional_creators.is_empty()
            {
                return Err(RoomEventValidationError::AdditionalCreatorsCannotBeEmpty {
                    room_version: self.room_version,
                });
            }
        } else if self.additional_creators.is_some() {
            return Err(RoomEventValidationError::AdditionalCreatorsNotSupported {
                room_version: self.room_version,
            });
        }

        if self.room_version.requires_creator_field() && self.creator.is_none() {
            return Err(RoomEventValidationError::CreatorRequired {
                room_version: self.room_version,
            });
        }

        if !self.room_version.requires_creator_field() && self.creator.is_some() {
            return Err(RoomEventValidationError::CreatorNotSupported {
                room_version: self.room_version,
            });
        }

        if let Some(predecessor) = self.predecessor.as_ref() {
            predecessor.validate()?;
        }

        Ok(())
    }
}

const fn default_true() -> bool {
    true
}
