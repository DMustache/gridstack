use std::{sync::Arc, time::Duration};

use crate::{
    infrastructure::user_identifier::UserIdentifier,
    services::{
        authorization::{
            entities::AuthorizedUserIdentifier,
            persistence::access_session_storage_unit::AccessSessionStorageUnit,
            service::AuthorizationService,
        },
        syncronization::{
            entities::{
                SyncBatch, SyncDeviceListsBatch, SyncEventsBatch, SyncFilterSelection, SyncRequest,
                SyncRoomsBatch,
            },
            errors::SyncronizationApplicationError,
            handlers::DefineFilterView,
            persistence::FilterRepository,
        },
    },
};
use serde_json::{Map, Value, json};

pub struct SyncronizationService {
    filter_repository: Arc<dyn FilterRepository>,
    authorization_service: Arc<AuthorizationService>,
}

impl SyncronizationService {
    pub fn new(
        filter_repository: Arc<dyn FilterRepository>,
        authorization_service: Arc<AuthorizationService>,
    ) -> Self {
        Self {
            filter_repository,
            authorization_service,
        }
    }

    pub fn define_filter(
        &self,
        access_session: &AccessSessionStorageUnit,
        user_id: &str,
        filter_payload: serde_json::Value,
    ) -> Result<DefineFilterView, SyncronizationApplicationError> {
        let authorized_user_identifier =
            self.require_authorized_user_for_request(access_session, user_id)?;

        let filter_id = self
            .filter_repository
            .create_filter(authorized_user_identifier.as_str(), filter_payload)
            .map_err(Self::map_domain_error)?;

        Ok(DefineFilterView { filter_id })
    }

    pub fn get_filter(
        &self,
        access_session: &AccessSessionStorageUnit,
        user_id: &str,
        filter_id: &str,
    ) -> Result<serde_json::Value, SyncronizationApplicationError> {
        let authorized_user_identifier =
            self.require_authorized_user_for_request(access_session, user_id)?;

        self.filter_repository
            .fetch_filter(authorized_user_identifier.as_str(), filter_id)
            .map_err(Self::map_domain_error)?
            .ok_or(SyncronizationApplicationError::NotFound)
    }

    pub async fn sync(
        &self,
        access_session: &AccessSessionStorageUnit,
        request: SyncRequest,
    ) -> Result<SyncBatch, SyncronizationApplicationError> {
        let authorized_user_identifier = self.require_authorized_user_for_request(
            access_session,
            access_session.user_identifier().as_str(),
        )?;

        let sync_filter_constraints = self
            .resolve_sync_filter_constraints(authorized_user_identifier.as_str(), request.filter)?;

        let since_stream_position = request
            .since
            .as_deref()
            .map(parse_sync_stream_position)
            .transpose()?
            .unwrap_or(0);

        let mut current_stream_position = self
            .filter_repository
            .fetch_current_sync_stream_position()
            .map_err(Self::map_domain_error)?;

        if !request.full_state
            && request.timeout_milliseconds > 0
            && current_stream_position <= since_stream_position
        {
            tokio::time::sleep(Duration::from_millis(request.timeout_milliseconds)).await;
            current_stream_position = self
                .filter_repository
                .fetch_current_sync_stream_position()
                .map_err(Self::map_domain_error)?;
        }

        let _use_state_after = request.use_state_after;
        self.persist_presence_update(
            authorized_user_identifier.as_str(),
            request.set_presence.as_ref(),
        )?;

        let memberships = self
            .filter_repository
            .fetch_user_room_memberships(authorized_user_identifier.as_str())
            .map_err(Self::map_domain_error)?;

        self.persist_account_data_snapshot(
            authorized_user_identifier.as_str(),
            current_stream_position,
        )?;

        self.persist_to_device_hint_event(
            authorized_user_identifier.as_str(),
            current_stream_position,
        )?;

        let presence_events = self
            .filter_repository
            .fetch_presence_events_for_users(&[authorized_user_identifier.as_str().to_owned()])
            .map_err(Self::map_domain_error)?;
        let account_data_events = self
            .filter_repository
            .fetch_global_account_data_events(authorized_user_identifier.as_str())
            .map_err(Self::map_domain_error)?;
        let to_device_events = self
            .filter_repository
            .fetch_to_device_events_since(
                authorized_user_identifier.as_str(),
                since_stream_position,
            )
            .map_err(Self::map_domain_error)?;

        let mut join_rooms = Map::new();
        let mut invite_rooms = Map::new();
        let mut leave_rooms = Map::new();

        for membership in memberships {
            if !sync_filter_constraints.room_allows(&membership.room_id, &membership.membership) {
                continue;
            }
            let room_entry = self.build_room_sync_entry(
                &membership.room_id,
                authorized_user_identifier.as_str(),
                since_stream_position,
                request.full_state,
                request.timeline_limit,
                &sync_filter_constraints,
            )?;

            match membership.membership.as_str() {
                "join" => {
                    join_rooms.insert(membership.room_id, room_entry);
                }
                "invite" => {
                    invite_rooms.insert(
                        membership.room_id,
                        json!({
                            "invite_state": {
                                "events": room_entry
                                    .get("state")
                                    .and_then(Value::as_object)
                                    .and_then(|state| state.get("events"))
                                    .cloned()
                                    .unwrap_or_else(|| Value::Array(Vec::new()))
                            }
                        }),
                    );
                }
                "leave" | "ban" => {
                    leave_rooms.insert(membership.room_id, room_entry);
                }
                _ => {}
            }
        }

        Ok(SyncBatch {
            next_batch: format!("s{current_stream_position}_0_0_0_0_0_0_0_0"),
            rooms: SyncRoomsBatch {
                join: join_rooms,
                invite: invite_rooms,
                leave: leave_rooms,
                knock: serde_json::Map::new(),
            },
            presence: SyncEventsBatch {
                events: presence_events,
            },
            account_data: SyncEventsBatch {
                events: account_data_events,
            },
            to_device: SyncEventsBatch {
                events: to_device_events,
            },
            device_lists: SyncDeviceListsBatch {
                changed: Vec::new(),
                left: Vec::new(),
            },
            device_one_time_keys_count: {
                let mut counts = serde_json::Map::new();
                counts.insert("signed_curve25519".to_owned(), Value::Number(0.into()));
                counts
            },
        })
    }

    fn build_room_sync_entry(
        &self,
        room_identifier: &str,
        user_identifier: &str,
        since_stream_position: i64,
        full_state: bool,
        timeline_limit: usize,
        sync_filter_constraints: &SyncFilterConstraints,
    ) -> Result<Value, SyncronizationApplicationError> {
        let timeline_events = self
            .filter_repository
            .fetch_room_timeline_events_since(
                room_identifier,
                since_stream_position,
                timeline_limit,
            )
            .map_err(Self::map_domain_error)?;
        let timeline_events = timeline_events
            .into_iter()
            .filter(|event| sync_filter_constraints.timeline_allows(event))
            .collect::<Vec<_>>();
        let state_events = if full_state {
            self.filter_repository
                .fetch_room_current_state_events(room_identifier)
                .map_err(Self::map_domain_error)?
        } else {
            self.filter_repository
                .fetch_room_state_events_since(room_identifier, since_stream_position)
                .map_err(Self::map_domain_error)?
        };
        let state_events = state_events
            .into_iter()
            .filter(|event| sync_filter_constraints.state_allows(event))
            .collect::<Vec<_>>();

        let limited = timeline_events.len() >= timeline_limit && timeline_limit > 0;
        let prev_batch = self
            .filter_repository
            .fetch_room_timeline_prev_batch(room_identifier, since_stream_position)
            .map_err(Self::map_domain_error)?;
        let ephemeral_events = self
            .filter_repository
            .fetch_room_ephemeral_receipt_events_since(
                room_identifier,
                user_identifier,
                since_stream_position,
            )
            .map_err(Self::map_domain_error)?;
        let room_account_data_events = self
            .filter_repository
            .fetch_room_account_data_events(
                room_identifier,
                user_identifier,
                since_stream_position,
                full_state,
            )
            .map_err(Self::map_domain_error)?;

        Ok(json!({
            "timeline": {
                "events": timeline_events
                    .into_iter()
                    .map(timeline_event_to_json)
                    .collect::<Vec<_>>(),
                "limited": limited,
                "prev_batch": format!("s{prev_batch}_0_0_0_0_0_0_0_0"),
            },
            "state": {
                "events": state_events
                    .into_iter()
                    .map(state_event_to_json)
                    .collect::<Vec<_>>()
            },
            "ephemeral": { "events": ephemeral_events },
            "account_data": { "events": room_account_data_events },
            "unread_notifications": {
                "highlight_count": 0,
                "notification_count": 0
            }
        }))
    }

    fn resolve_sync_filter_constraints(
        &self,
        user_identifier: &str,
        filter_selection: Option<SyncFilterSelection>,
    ) -> Result<SyncFilterConstraints, SyncronizationApplicationError> {
        let Some(filter_selection) = filter_selection else {
            return Ok(SyncFilterConstraints::default());
        };

        let filter_json = match filter_selection {
            SyncFilterSelection::InlineFilterDefinition(inline_filter_definition) => {
                inline_filter_definition
            }
            SyncFilterSelection::StoredFilterIdentifier(filter_identifier) => self
                .filter_repository
                .fetch_filter(user_identifier, &filter_identifier)
                .map_err(Self::map_domain_error)?
                .ok_or(SyncronizationApplicationError::InvalidParameter)?,
        };

        Ok(SyncFilterConstraints::from_json(filter_json))
    }

    fn require_authorized_user_for_request(
        &self,
        access_session: &AccessSessionStorageUnit,
        user_id: &str,
    ) -> Result<AuthorizedUserIdentifier, SyncronizationApplicationError> {
        let requested_user_identifier = UserIdentifier::try_from(user_id.to_owned())
            .map_err(|_| SyncronizationApplicationError::InvalidParameter)?;

        self.authorization_service
            .require_authorized_user(access_session, &requested_user_identifier)
            .map_err(|error| match error {
                crate::services::authorization::errors::AuthorizationApplicationError::Unauthorized => {
                    SyncronizationApplicationError::Unauthorized
                }
                crate::services::authorization::errors::AuthorizationApplicationError::Forbidden => {
                    SyncronizationApplicationError::Forbidden
                }
                _ => SyncronizationApplicationError::Internal,
            })
    }

    fn map_domain_error(
        error: crate::services::errors::DomainError,
    ) -> SyncronizationApplicationError {
        match error {
            crate::services::errors::DomainError::InvalidRequest(_reason) => {
                SyncronizationApplicationError::InvalidParameter
            }
            _ => SyncronizationApplicationError::Internal,
        }
    }

    fn persist_presence_update(
        &self,
        user_identifier: &str,
        set_presence: Option<&crate::services::syncronization::entities::SyncSetPresence>,
    ) -> Result<(), SyncronizationApplicationError> {
        let presence = match set_presence {
            Some(crate::services::syncronization::entities::SyncSetPresence::Offline) => "offline",
            Some(crate::services::syncronization::entities::SyncSetPresence::Unavailable) => {
                "unavailable"
            }
            Some(crate::services::syncronization::entities::SyncSetPresence::Online) | None => {
                "online"
            }
        };
        self.filter_repository
            .upsert_presence(user_identifier, presence)
            .map_err(Self::map_domain_error)
    }

    fn persist_account_data_snapshot(
        &self,
        user_identifier: &str,
        current_stream_position: i64,
    ) -> Result<(), SyncronizationApplicationError> {
        self.filter_repository
            .upsert_global_account_data_event(
                user_identifier,
                "org.matrix.msc3575.sync_stream_position",
                serde_json::json!({ "stream_position": current_stream_position }),
            )
            .map_err(Self::map_domain_error)
    }

    fn persist_to_device_hint_event(
        &self,
        user_identifier: &str,
        current_stream_position: i64,
    ) -> Result<(), SyncronizationApplicationError> {
        self.filter_repository
            .enqueue_to_device_event(
                user_identifier,
                "org.matrix.gridstack.sync.hint",
                serde_json::json!({ "stream_position": current_stream_position }),
            )
            .map(|_| ())
            .map_err(Self::map_domain_error)
    }
}

fn parse_sync_stream_position(sync_token: &str) -> Result<i64, SyncronizationApplicationError> {
    let token_without_prefix = sync_token
        .strip_prefix('s')
        .ok_or(SyncronizationApplicationError::InvalidParameter)?;
    let stream_position_fragment = token_without_prefix
        .split('_')
        .next()
        .ok_or(SyncronizationApplicationError::InvalidParameter)?;
    stream_position_fragment
        .parse::<i64>()
        .map_err(|_| SyncronizationApplicationError::InvalidParameter)
}

fn timeline_event_to_json(
    event: crate::services::syncronization::persistence::TimelineEventRecord,
) -> Value {
    let mut event_object = Map::new();
    event_object.insert("event_id".to_owned(), Value::String(event.event_id));
    event_object.insert("type".to_owned(), Value::String(event.event_type));
    event_object.insert("sender".to_owned(), Value::String(event.sender_user_id));
    event_object.insert(
        "origin_server_ts".to_owned(),
        Value::Number(event.origin_server_ts.into()),
    );
    event_object.insert("content".to_owned(), event.content_json);
    if let Some(state_key) = event.state_key {
        event_object.insert("state_key".to_owned(), Value::String(state_key));
    }
    if let Some(unsigned_json) = event.unsigned_json {
        event_object.insert("unsigned".to_owned(), unsigned_json);
    }
    Value::Object(event_object)
}

fn state_event_to_json(
    event: crate::services::syncronization::persistence::StateEventRecord,
) -> Value {
    let mut event_object = Map::new();
    event_object.insert("event_id".to_owned(), Value::String(event.event_id));
    event_object.insert("type".to_owned(), Value::String(event.event_type));
    event_object.insert("sender".to_owned(), Value::String(event.sender_user_id));
    event_object.insert(
        "origin_server_ts".to_owned(),
        Value::Number(event.origin_server_ts.into()),
    );
    event_object.insert("content".to_owned(), event.content_json);
    if let Some(state_key) = event.state_key {
        event_object.insert("state_key".to_owned(), Value::String(state_key));
    }
    if let Some(unsigned_json) = event.unsigned_json {
        event_object.insert("unsigned".to_owned(), unsigned_json);
    }
    Value::Object(event_object)
}

#[derive(Clone, Debug, Default)]
struct SyncFilterConstraints {
    include_leave: bool,
    rooms: Option<Vec<String>>,
    not_rooms: Option<Vec<String>>,
    timeline_filter: RoomEventFilterConstraints,
    state_filter: RoomEventFilterConstraints,
}

impl SyncFilterConstraints {
    fn from_json(filter_json: Value) -> Self {
        let room = filter_json.get("room");
        Self {
            include_leave: room
                .and_then(|value| value.get("include_leave"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            rooms: read_string_array(room.and_then(|value| value.get("rooms"))),
            not_rooms: read_string_array(room.and_then(|value| value.get("not_rooms"))),
            timeline_filter: RoomEventFilterConstraints::from_json(
                room.and_then(|value| value.get("timeline")),
            ),
            state_filter: RoomEventFilterConstraints::from_json(
                room.and_then(|value| value.get("state")),
            ),
        }
    }

    fn room_allows(&self, room_id: &str, membership: &str) -> bool {
        if matches!(membership, "leave" | "ban") && !self.include_leave {
            return false;
        }
        if let Some(rooms) = self.rooms.as_ref()
            && !rooms.iter().any(|value| value == room_id)
        {
            return false;
        }
        if let Some(not_rooms) = self.not_rooms.as_ref()
            && not_rooms.iter().any(|value| value == room_id)
        {
            return false;
        }
        true
    }

    fn timeline_allows(
        &self,
        event: &crate::services::syncronization::persistence::TimelineEventRecord,
    ) -> bool {
        self.timeline_filter.allows(
            &event.event_type,
            &event.sender_user_id,
            &event.content_json,
        )
    }

    fn state_allows(
        &self,
        event: &crate::services::syncronization::persistence::StateEventRecord,
    ) -> bool {
        self.state_filter.allows(
            &event.event_type,
            &event.sender_user_id,
            &event.content_json,
        )
    }
}

#[derive(Clone, Debug, Default)]
struct RoomEventFilterConstraints {
    types: Option<Vec<String>>,
    not_types: Option<Vec<String>>,
    senders: Option<Vec<String>>,
    not_senders: Option<Vec<String>>,
    contains_url: Option<bool>,
}

impl RoomEventFilterConstraints {
    fn from_json(filter_json: Option<&Value>) -> Self {
        Self {
            types: read_string_array(filter_json.and_then(|value| value.get("types"))),
            not_types: read_string_array(filter_json.and_then(|value| value.get("not_types"))),
            senders: read_string_array(filter_json.and_then(|value| value.get("senders"))),
            not_senders: read_string_array(filter_json.and_then(|value| value.get("not_senders"))),
            contains_url: filter_json
                .and_then(|value| value.get("contains_url"))
                .and_then(Value::as_bool),
        }
    }

    fn allows(&self, event_type: &str, sender: &str, content: &Value) -> bool {
        if let Some(types) = self.types.as_ref()
            && !types.iter().any(|value| value == event_type)
        {
            return false;
        }
        if let Some(not_types) = self.not_types.as_ref()
            && not_types.iter().any(|value| value == event_type)
        {
            return false;
        }
        if let Some(senders) = self.senders.as_ref()
            && !senders.iter().any(|value| value == sender)
        {
            return false;
        }
        if let Some(not_senders) = self.not_senders.as_ref()
            && not_senders.iter().any(|value| value == sender)
        {
            return false;
        }
        if let Some(contains_url) = self.contains_url {
            let has_url = content.get("url").is_some();
            if has_url != contains_url {
                return false;
            }
        }
        true
    }
}

fn read_string_array(value: Option<&Value>) -> Option<Vec<String>> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
}
