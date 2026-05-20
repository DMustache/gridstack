use crate::infrastructure::server_name::ServerName;
use crate::services::authorization::entities::AuthorizedUserIdentifier;
use crate::services::events::entities::StateEventKind;
use crate::services::events::service::EventsService;
use crate::services::rooms::entities::parse_room_state_event_content;
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::RoomStateWriteRepository;

use super::room_access::{require_joined_room_version, require_room_identifier};

pub struct SetRoomStateWithKeyUseCase<'a, Repository>
where
    Repository: RoomStateWriteRepository + ?Sized,
{
    room_repository: &'a Repository,
    events_service: &'a EventsService,
    server_name: &'a ServerName,
}

impl<'a, Repository> SetRoomStateWithKeyUseCase<'a, Repository>
where
    Repository: RoomStateWriteRepository + ?Sized,
{
    pub fn new(
        room_repository: &'a Repository,
        events_service: &'a EventsService,
        server_name: &'a ServerName,
    ) -> Self {
        Self {
            room_repository,
            events_service,
            server_name,
        }
    }

    pub fn execute(
        &self,
        user_id: &AuthorizedUserIdentifier,
        room_id: String,
        event_type: String,
        state_key: String,
        content: serde_json::Value,
    ) -> Result<String, RoomsApplicationError> {
        require_room_identifier(&room_id)?;
        if !content.is_object() {
            return Err(RoomsApplicationError::InvalidParameter);
        }

        let room_version = require_joined_room_version(
            self.room_repository,
            self.events_service,
            &room_id,
            user_id.as_existing_user_identifier(),
        )?;

        let state_event_kind = event_type
            .parse::<StateEventKind>()
            .map_err(|_| RoomsApplicationError::InvalidParameter)?;
        self.validate_canonical_alias_state_if_needed(&room_id, &state_event_kind, &content)?;

        let event_content = parse_room_state_event_content(state_event_kind.clone(), content)
            .map_err(|_| RoomsApplicationError::InvalidRoomState)?;

        let event_write_contract = self
            .events_service
            .create_state_event(
                room_id.clone(),
                room_version,
                user_id.as_existing_user_identifier().as_str().to_owned(),
                state_event_kind,
                state_key,
                event_content,
                None,
            )
            .map_err(RoomsApplicationError::from)?;

        let event_id = event_write_contract
            .event_rows
            .first()
            .map(|event| event.event_id.clone())
            .ok_or(RoomsApplicationError::Internal)?;

        self.room_repository
            .append_room_event(&event_write_contract)
            .map_err(|error| super::map_room_persistence_error(error, room_id))?;

        Ok(event_id)
    }

    fn validate_canonical_alias_state_if_needed(
        &self,
        room_id: &str,
        state_event_kind: &StateEventKind,
        content: &serde_json::Value,
    ) -> Result<(), RoomsApplicationError> {
        if !matches!(state_event_kind, StateEventKind::RoomCanonicalAlias) {
            return Ok(());
        }

        let mut aliases = Vec::new();
        if let Some(alias_value) = content.get("alias") {
            let alias = alias_value
                .as_str()
                .ok_or(RoomsApplicationError::InvalidParameter)?;
            aliases.push(alias.to_owned());
        }
        if let Some(alt_aliases_value) = content.get("alt_aliases") {
            let alt_aliases = alt_aliases_value
                .as_array()
                .ok_or(RoomsApplicationError::InvalidParameter)?;
            for alias_value in alt_aliases {
                let alias = alias_value
                    .as_str()
                    .ok_or(RoomsApplicationError::InvalidParameter)?;
                aliases.push(alias.to_owned());
            }
        }

        for alias in aliases {
            let alias_localpart = parse_room_alias_localpart_for_local_server(
                alias.as_str(),
                self.server_name.as_str(),
            )
            .ok_or(RoomsApplicationError::InvalidParameter)?;

            let bound_room_id = self
                .room_repository
                .fetch_room_id_by_alias_localpart(alias_localpart)
                .map_err(|_| RoomsApplicationError::Internal)?;
            if bound_room_id.as_deref() != Some(room_id) {
                return Err(RoomsApplicationError::BadAlias);
            }
        }

        Ok(())
    }
}

fn parse_room_alias_localpart_for_local_server<'a>(
    room_alias: &'a str,
    server_name: &str,
) -> Option<&'a str> {
    let alias_without_prefix = room_alias.strip_prefix('#')?;
    let (localpart, alias_server_name) = alias_without_prefix.split_once(':')?;
    if localpart.trim().is_empty()
        || alias_server_name.trim().is_empty()
        || alias_server_name != server_name
    {
        return None;
    }
    Some(localpart)
}
