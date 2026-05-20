use diesel::{
    BoolExpressionMethods, ExpressionMethods, JoinOnDsl, NullableExpressionMethods,
    OptionalExtension, QueryDsl, RunQueryDsl,
    dsl::{max, min},
    insert_into,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::RwLock,
};

use crate::{infrastructure::schema, services::errors::DomainError};

pub trait FilterRepository: Send + Sync {
    fn create_filter(
        &self,
        user_identifier: &str,
        filter_payload: Value,
    ) -> Result<String, DomainError>;
    fn fetch_filter(
        &self,
        user_identifier: &str,
        filter_identifier: &str,
    ) -> Result<Option<Value>, DomainError>;
    fn fetch_current_sync_stream_position(&self) -> Result<i64, DomainError>;
    fn fetch_user_room_memberships(
        &self,
        user_identifier: &str,
    ) -> Result<Vec<UserRoomMembershipRecord>, DomainError>;
    fn fetch_room_timeline_events_since(
        &self,
        room_identifier: &str,
        since_stream_position: i64,
        limit: usize,
    ) -> Result<Vec<TimelineEventRecord>, DomainError>;
    fn fetch_room_state_events_since(
        &self,
        room_identifier: &str,
        since_stream_position: i64,
    ) -> Result<Vec<StateEventRecord>, DomainError>;
    fn fetch_room_current_state_events(
        &self,
        room_identifier: &str,
    ) -> Result<Vec<StateEventRecord>, DomainError>;
    fn fetch_room_timeline_prev_batch(
        &self,
        room_identifier: &str,
        since_stream_position: i64,
    ) -> Result<i64, DomainError>;
    fn fetch_room_ephemeral_receipt_events_since(
        &self,
        room_identifier: &str,
        user_identifier: &str,
        since_stream_position: i64,
    ) -> Result<Vec<Value>, DomainError>;
    fn fetch_room_account_data_events(
        &self,
        room_identifier: &str,
        user_identifier: &str,
        since_stream_position: i64,
        full_state: bool,
    ) -> Result<Vec<Value>, DomainError>;
    fn upsert_presence(&self, user_identifier: &str, presence: &str) -> Result<(), DomainError>;
    fn fetch_presence_events_for_users(
        &self,
        user_identifiers: &[String],
    ) -> Result<Vec<Value>, DomainError>;
    fn upsert_global_account_data_event(
        &self,
        user_identifier: &str,
        event_type: &str,
        content: Value,
    ) -> Result<(), DomainError>;
    fn fetch_global_account_data_events(
        &self,
        user_identifier: &str,
    ) -> Result<Vec<Value>, DomainError>;
    fn enqueue_to_device_event(
        &self,
        user_identifier: &str,
        event_type: &str,
        content: Value,
    ) -> Result<i64, DomainError>;
    fn fetch_to_device_events_since(
        &self,
        user_identifier: &str,
        since_stream_position: i64,
    ) -> Result<Vec<Value>, DomainError>;
}

#[derive(Clone, Debug)]
pub struct UserRoomMembershipRecord {
    pub room_id: String,
    pub membership: String,
}

#[derive(Clone, Debug)]
pub struct TimelineEventRecord {
    pub event_id: String,
    pub event_type: String,
    pub sender_user_id: String,
    pub state_key: Option<String>,
    pub origin_server_ts: i64,
    pub content_json: Value,
    pub unsigned_json: Option<Value>,
    pub stream_position: i64,
}

#[derive(Clone, Debug)]
pub struct StateEventRecord {
    pub event_id: String,
    pub event_type: String,
    pub sender_user_id: String,
    pub state_key: Option<String>,
    pub origin_server_ts: i64,
    pub content_json: Value,
    pub unsigned_json: Option<Value>,
}

pub struct SyncronizationPersistence {
    connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    presence_store: RwLock<HashMap<String, String>>,
    account_data_store: RwLock<HashMap<String, HashMap<String, Value>>>,
    to_device_store: RwLock<Vec<ToDeviceRecord>>,
    state_directory_path: PathBuf,
}

#[derive(Clone, Debug)]
struct ToDeviceRecord {
    stream_position: i64,
    user_identifier: String,
    event_type: String,
    content: Value,
}

impl SyncronizationPersistence {
    #[must_use]
    pub fn new(database_url: &str) -> Self {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let connection_pool = r2d2::Pool::builder().build_unchecked(manager);
        let state_directory_path = PathBuf::from("generated/state");
        Self {
            connection_pool,
            presence_store: RwLock::new(load_presence_store(&state_directory_path)),
            account_data_store: RwLock::new(load_account_data_store(&state_directory_path)),
            to_device_store: RwLock::new(load_to_device_store(&state_directory_path)),
            state_directory_path,
        }
    }
}

impl FilterRepository for SyncronizationPersistence {
    fn create_filter(
        &self,
        user_identifier: &str,
        filter_payload: Value,
    ) -> Result<String, DomainError> {
        use schema::user_filters;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let created = insert_into(user_filters::table)
            .values((
                user_filters::user_id.eq(user_identifier),
                user_filters::filter_json.eq(filter_payload),
            ))
            .returning(user_filters::id)
            .get_result::<i64>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(created.to_string())
    }

    fn fetch_filter(
        &self,
        user_identifier: &str,
        filter_identifier: &str,
    ) -> Result<Option<Value>, DomainError> {
        use schema::user_filters;

        let filter_identifier_number = match filter_identifier.parse::<i64>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        user_filters::table
            .filter(user_filters::user_id.eq(user_identifier))
            .filter(user_filters::id.eq(filter_identifier_number))
            .select(user_filters::filter_json)
            .first::<Value>(&mut connection)
            .optional()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))
    }

    fn fetch_current_sync_stream_position(&self) -> Result<i64, DomainError> {
        use schema::room_sync_stream;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let stream_position = room_sync_stream::table
            .select(max(room_sync_stream::stream_position))
            .first::<Option<i64>>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        Ok(stream_position.unwrap_or(0))
    }

    fn fetch_user_room_memberships(
        &self,
        user_identifier: &str,
    ) -> Result<Vec<UserRoomMembershipRecord>, DomainError> {
        use schema::room_membership_projection;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let rows = room_membership_projection::table
            .filter(room_membership_projection::user_id.eq(user_identifier))
            .select((
                room_membership_projection::room_id,
                room_membership_projection::membership,
            ))
            .load::<(String, String)>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(room_id, membership)| UserRoomMembershipRecord {
                room_id,
                membership,
            })
            .collect())
    }

    fn fetch_room_timeline_events_since(
        &self,
        room_identifier: &str,
        since_stream_position: i64,
        limit: usize,
    ) -> Result<Vec<TimelineEventRecord>, DomainError> {
        use schema::{room_events, room_sync_stream};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);

        let rows = room_sync_stream::table
            .inner_join(room_events::table.on(room_events::event_id.eq(room_sync_stream::event_id)))
            .filter(room_sync_stream::room_id.eq(room_identifier))
            .filter(room_sync_stream::stream_position.gt(since_stream_position))
            .order(room_sync_stream::stream_position.asc())
            .limit(limit)
            .select((
                room_events::event_id,
                room_events::event_type,
                room_events::sender_user_id,
                room_events::state_key.nullable(),
                room_events::origin_server_ts,
                room_events::content_json,
                room_events::unsigned_json,
                room_sync_stream::stream_position,
            ))
            .load::<(
                String,
                String,
                String,
                Option<String>,
                i64,
                Value,
                Option<Value>,
                i64,
            )>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let (
                    event_id,
                    event_type,
                    sender_user_id,
                    state_key,
                    origin_server_ts,
                    content_json,
                    unsigned_json,
                    stream_position,
                ) = row;
                TimelineEventRecord {
                    event_id,
                    event_type,
                    sender_user_id,
                    state_key,
                    origin_server_ts,
                    content_json,
                    unsigned_json,
                    stream_position,
                }
            })
            .collect())
    }

    fn fetch_room_state_events_since(
        &self,
        room_identifier: &str,
        since_stream_position: i64,
    ) -> Result<Vec<StateEventRecord>, DomainError> {
        use schema::{room_events, room_sync_stream};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let rows = room_sync_stream::table
            .inner_join(room_events::table.on(room_events::event_id.eq(room_sync_stream::event_id)))
            .filter(room_sync_stream::room_id.eq(room_identifier))
            .filter(room_sync_stream::stream_position.gt(since_stream_position))
            .filter(room_events::state_key.is_not_null())
            .order(room_sync_stream::stream_position.asc())
            .select((
                room_events::event_id,
                room_events::event_type,
                room_events::sender_user_id,
                room_events::state_key.nullable(),
                room_events::origin_server_ts,
                room_events::content_json,
                room_events::unsigned_json,
                room_sync_stream::stream_position,
            ))
            .load::<(
                String,
                String,
                String,
                Option<String>,
                i64,
                Value,
                Option<Value>,
                i64,
            )>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let mut latest_by_state_key = BTreeMap::<(String, String), StateEventRecord>::new();
        for (
            event_id,
            event_type,
            sender_user_id,
            state_key,
            origin_server_ts,
            content_json,
            unsigned_json,
            _stream_position,
        ) in rows
        {
            let state_key_value = state_key.unwrap_or_default();
            latest_by_state_key.insert(
                (event_type.clone(), state_key_value.clone()),
                StateEventRecord {
                    event_id,
                    event_type,
                    sender_user_id,
                    state_key: Some(state_key_value),
                    origin_server_ts,
                    content_json,
                    unsigned_json,
                },
            );
        }

        Ok(latest_by_state_key.into_values().collect())
    }

    fn fetch_room_current_state_events(
        &self,
        room_identifier: &str,
    ) -> Result<Vec<StateEventRecord>, DomainError> {
        use schema::{room_current_state, room_events};

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let rows = room_current_state::table
            .inner_join(
                room_events::table.on(room_events::event_id.eq(room_current_state::event_id)),
            )
            .filter(room_current_state::room_id.eq(room_identifier))
            .order((room_events::event_type.asc(), room_events::state_key.asc()))
            .select((
                room_events::event_id,
                room_events::event_type,
                room_events::sender_user_id,
                room_events::state_key.nullable(),
                room_events::origin_server_ts,
                room_events::content_json,
                room_events::unsigned_json,
            ))
            .load::<(
                String,
                String,
                String,
                Option<String>,
                i64,
                Value,
                Option<Value>,
            )>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let (
                    event_id,
                    event_type,
                    sender_user_id,
                    state_key,
                    origin_server_ts,
                    content_json,
                    unsigned_json,
                ) = row;
                StateEventRecord {
                    event_id,
                    event_type,
                    sender_user_id,
                    state_key,
                    origin_server_ts,
                    content_json,
                    unsigned_json,
                }
            })
            .collect())
    }

    fn fetch_room_timeline_prev_batch(
        &self,
        room_identifier: &str,
        since_stream_position: i64,
    ) -> Result<i64, DomainError> {
        use schema::room_sync_stream;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let min_position = room_sync_stream::table
            .filter(room_sync_stream::room_id.eq(room_identifier))
            .filter(room_sync_stream::stream_position.gt(since_stream_position))
            .select(min(room_sync_stream::stream_position))
            .first::<Option<i64>>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        Ok(min_position
            .unwrap_or(since_stream_position)
            .saturating_sub(1))
    }

    fn fetch_room_ephemeral_receipt_events_since(
        &self,
        room_identifier: &str,
        user_identifier: &str,
        since_stream_position: i64,
    ) -> Result<Vec<Value>, DomainError> {
        use schema::room_receipts;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let rows = room_receipts::table
            .filter(room_receipts::room_id.eq(room_identifier))
            .filter(room_receipts::stream_position.gt(since_stream_position))
            .filter(
                room_receipts::receipt_type
                    .eq("m.read")
                    .or(room_receipts::receipt_type
                        .eq("m.read.private")
                        .and(room_receipts::user_id.eq(user_identifier))),
            )
            .order(room_receipts::stream_position.asc())
            .select((
                room_receipts::event_id,
                room_receipts::receipt_type,
                room_receipts::user_id,
                room_receipts::thread_id,
                room_receipts::receipt_ts,
            ))
            .load::<(String, String, String, String, i64)>(&mut connection)
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(event_id, receipt_type, user_id, thread_id, receipt_ts)| {
                let mut receipt_content = serde_json::Map::new();
                receipt_content.insert("ts".to_owned(), Value::Number(receipt_ts.into()));
                if !thread_id.is_empty() {
                    receipt_content.insert("thread_id".to_owned(), Value::String(thread_id));
                }

                serde_json::json!({
                    "type": "m.receipt",
                    "content": {
                        event_id: {
                            receipt_type: {
                                user_id: receipt_content
                            }
                        }
                    }
                })
            })
            .collect())
    }

    fn fetch_room_account_data_events(
        &self,
        room_identifier: &str,
        user_identifier: &str,
        since_stream_position: i64,
        full_state: bool,
    ) -> Result<Vec<Value>, DomainError> {
        use schema::room_read_markers;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let event_ids = if full_state {
            room_read_markers::table
                .filter(room_read_markers::room_id.eq(room_identifier))
                .filter(room_read_markers::user_id.eq(user_identifier))
                .order(room_read_markers::stream_position.desc())
                .select(room_read_markers::fully_read_event_id)
                .first::<String>(&mut connection)
                .optional()
                .map_err(|error| DomainError::InvalidRequest(error.to_string()))?
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            room_read_markers::table
                .filter(room_read_markers::room_id.eq(room_identifier))
                .filter(room_read_markers::user_id.eq(user_identifier))
                .filter(room_read_markers::stream_position.gt(since_stream_position))
                .order(room_read_markers::stream_position.asc())
                .select(room_read_markers::fully_read_event_id)
                .load::<String>(&mut connection)
                .map_err(|error| DomainError::InvalidRequest(error.to_string()))?
        };

        Ok(event_ids
            .into_iter()
            .map(|fully_read_event_id| {
                serde_json::json!({
                    "type": "m.fully_read",
                    "content": {
                        "event_id": fully_read_event_id
                    }
                })
            })
            .collect())
    }

    fn upsert_presence(&self, user_identifier: &str, presence: &str) -> Result<(), DomainError> {
        {
            let mut presence_store = self.presence_store.write().map_err(|_| {
                DomainError::InvalidRequest("presence storage lock failure".to_owned())
            })?;
            presence_store.insert(user_identifier.to_owned(), presence.to_owned());
        }
        persist_presence_store(&self.state_directory_path, &self.presence_store)
    }

    fn fetch_presence_events_for_users(
        &self,
        user_identifiers: &[String],
    ) -> Result<Vec<Value>, DomainError> {
        let presence_store = self
            .presence_store
            .read()
            .map_err(|_| DomainError::InvalidRequest("presence storage lock failure".to_owned()))?;
        Ok(user_identifiers
            .iter()
            .filter_map(|user_identifier| {
                presence_store.get(user_identifier).map(|presence| {
                    serde_json::json!({
                        "type": "m.presence",
                        "sender": user_identifier,
                        "content": { "presence": presence }
                    })
                })
            })
            .collect())
    }

    fn upsert_global_account_data_event(
        &self,
        user_identifier: &str,
        event_type: &str,
        content: Value,
    ) -> Result<(), DomainError> {
        {
            let mut account_data_store = self
                .account_data_store
                .write()
                .map_err(|_| DomainError::InvalidRequest("account data lock failure".to_owned()))?;
            let user_events = account_data_store
                .entry(user_identifier.to_owned())
                .or_insert_with(HashMap::new);
            user_events.insert(event_type.to_owned(), content);
        }
        persist_account_data_store(&self.state_directory_path, &self.account_data_store)
    }

    fn fetch_global_account_data_events(
        &self,
        user_identifier: &str,
    ) -> Result<Vec<Value>, DomainError> {
        let account_data_store = self
            .account_data_store
            .read()
            .map_err(|_| DomainError::InvalidRequest("account data lock failure".to_owned()))?;
        let events = account_data_store
            .get(user_identifier)
            .map(|event_by_type| {
                event_by_type
                    .iter()
                    .map(|(event_type, content)| {
                        serde_json::json!({
                            "type": event_type,
                            "content": content
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(events)
    }

    fn enqueue_to_device_event(
        &self,
        user_identifier: &str,
        event_type: &str,
        content: Value,
    ) -> Result<i64, DomainError> {
        let stream_position = self.fetch_current_sync_stream_position()?.saturating_add(1);
        {
            let mut to_device_store = self
                .to_device_store
                .write()
                .map_err(|_| DomainError::InvalidRequest("to-device lock failure".to_owned()))?;
            to_device_store.push(ToDeviceRecord {
                stream_position,
                user_identifier: user_identifier.to_owned(),
                event_type: event_type.to_owned(),
                content,
            });
        }
        persist_to_device_store(&self.state_directory_path, &self.to_device_store)?;
        Ok(stream_position)
    }

    fn fetch_to_device_events_since(
        &self,
        user_identifier: &str,
        since_stream_position: i64,
    ) -> Result<Vec<Value>, DomainError> {
        let to_device_store = self
            .to_device_store
            .read()
            .map_err(|_| DomainError::InvalidRequest("to-device lock failure".to_owned()))?;
        Ok(to_device_store
            .iter()
            .filter(|record| {
                record.user_identifier == user_identifier
                    && record.stream_position > since_stream_position
            })
            .map(|record| {
                serde_json::json!({
                    "type": record.event_type,
                    "sender": user_identifier,
                    "content": record.content
                })
            })
            .collect())
    }
}

fn load_presence_store(state_directory_path: &PathBuf) -> HashMap<String, String> {
    let file_path = state_directory_path.join("presence.tsv");
    let Ok(file) = fs::File::open(file_path) else {
        return HashMap::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| {
            let (user_identifier, presence) = line.split_once('\t')?;
            Some((user_identifier.to_owned(), presence.to_owned()))
        })
        .collect()
}

fn persist_presence_store(
    state_directory_path: &PathBuf,
    presence_store: &RwLock<HashMap<String, String>>,
) -> Result<(), DomainError> {
    fs::create_dir_all(state_directory_path)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    let file_path = state_directory_path.join("presence.tsv");
    let snapshot = presence_store
        .read()
        .map_err(|_| DomainError::InvalidRequest("presence storage lock failure".to_owned()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_path)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    for (user_identifier, presence) in snapshot.iter() {
        writeln!(file, "{user_identifier}\t{presence}")
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    }
    file.flush()
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    Ok(())
}

fn load_account_data_store(
    state_directory_path: &PathBuf,
) -> HashMap<String, HashMap<String, Value>> {
    let file_path = state_directory_path.join("account_data.json");
    let Ok(raw) = fs::read_to_string(file_path) else {
        return HashMap::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn persist_account_data_store(
    state_directory_path: &PathBuf,
    account_data_store: &RwLock<HashMap<String, HashMap<String, Value>>>,
) -> Result<(), DomainError> {
    fs::create_dir_all(state_directory_path)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    let file_path = state_directory_path.join("account_data.json");
    let snapshot = account_data_store
        .read()
        .map_err(|_| DomainError::InvalidRequest("account data lock failure".to_owned()))?;
    let payload = serde_json::to_vec_pretty(&*snapshot)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    fs::write(file_path, payload).map_err(|error| DomainError::InvalidRequest(error.to_string()))
}

fn load_to_device_store(state_directory_path: &PathBuf) -> Vec<ToDeviceRecord> {
    let file_path = state_directory_path.join("to_device.jsonl");
    let Ok(file) = fs::File::open(file_path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<ToDeviceRecordWire>(&line).ok())
        .map(ToDeviceRecord::from)
        .collect()
}

fn persist_to_device_store(
    state_directory_path: &PathBuf,
    to_device_store: &RwLock<Vec<ToDeviceRecord>>,
) -> Result<(), DomainError> {
    fs::create_dir_all(state_directory_path)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    let file_path = state_directory_path.join("to_device.jsonl");
    let snapshot = to_device_store
        .read()
        .map_err(|_| DomainError::InvalidRequest("to-device lock failure".to_owned()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file_path)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    for record in snapshot.iter() {
        let payload = serde_json::to_string(&ToDeviceRecordWire::from(record.clone()))
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        writeln!(file, "{payload}")
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    }
    file.flush()
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ToDeviceRecordWire {
    stream_position: i64,
    user_identifier: String,
    event_type: String,
    content: Value,
}

impl From<ToDeviceRecordWire> for ToDeviceRecord {
    fn from(value: ToDeviceRecordWire) -> Self {
        Self {
            stream_position: value.stream_position,
            user_identifier: value.user_identifier,
            event_type: value.event_type,
            content: value.content,
        }
    }
}

impl From<ToDeviceRecord> for ToDeviceRecordWire {
    fn from(value: ToDeviceRecord) -> Self {
        Self {
            stream_position: value.stream_position,
            user_identifier: value.user_identifier,
            event_type: value.event_type,
            content: value.content,
        }
    }
}
