use diesel::{
    OptionalExtension, QueryableByName, RunQueryDsl,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
    sql_query,
    sql_types::{BigInt, Jsonb, Nullable, Text},
};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::RwLock,
};

use crate::services::errors::DomainError;

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

#[derive(QueryableByName)]
struct CreatedFilterRecord {
    #[diesel(sql_type = BigInt)]
    id: i64,
}

#[derive(QueryableByName)]
struct StoredFilterRecord {
    #[diesel(sql_type = Jsonb)]
    filter_json: Value,
}

#[derive(QueryableByName)]
struct CurrentSyncStreamPositionRecord {
    #[diesel(sql_type = Nullable<BigInt>)]
    stream_position: Option<i64>,
}

#[derive(QueryableByName)]
struct UserRoomMembershipSqlRecord {
    #[diesel(sql_type = Text)]
    room_id: String,
    #[diesel(sql_type = Text)]
    membership: String,
}

#[derive(QueryableByName)]
struct TimelineEventSqlRecord {
    #[diesel(sql_type = Text)]
    event_id: String,
    #[diesel(sql_type = Text)]
    event_type: String,
    #[diesel(sql_type = Text)]
    sender_user_id: String,
    #[diesel(sql_type = Nullable<Text>)]
    state_key: Option<String>,
    #[diesel(sql_type = BigInt)]
    origin_server_ts: i64,
    #[diesel(sql_type = Jsonb)]
    content_json: Value,
    #[diesel(sql_type = Nullable<Jsonb>)]
    unsigned_json: Option<Value>,
    #[diesel(sql_type = BigInt)]
    stream_position: i64,
}

#[derive(QueryableByName)]
struct StateEventSqlRecord {
    #[diesel(sql_type = Text)]
    event_id: String,
    #[diesel(sql_type = Text)]
    event_type: String,
    #[diesel(sql_type = Text)]
    sender_user_id: String,
    #[diesel(sql_type = Nullable<Text>)]
    state_key: Option<String>,
    #[diesel(sql_type = BigInt)]
    origin_server_ts: i64,
    #[diesel(sql_type = Jsonb)]
    content_json: Value,
    #[diesel(sql_type = Nullable<Jsonb>)]
    unsigned_json: Option<Value>,
}

#[derive(QueryableByName)]
struct PrevBatchSqlRecord {
    #[diesel(sql_type = Nullable<BigInt>)]
    stream_position: Option<i64>,
}

impl FilterRepository for SyncronizationPersistence {
    fn create_filter(
        &self,
        user_identifier: &str,
        filter_payload: Value,
    ) -> Result<String, DomainError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let created = sql_query(
            "INSERT INTO public.user_filters (user_id, filter_json) VALUES ($1, $2) RETURNING id",
        )
        .bind::<Text, _>(user_identifier)
        .bind::<Jsonb, _>(filter_payload)
        .get_result::<CreatedFilterRecord>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(created.id.to_string())
    }

    fn fetch_filter(
        &self,
        user_identifier: &str,
        filter_identifier: &str,
    ) -> Result<Option<Value>, DomainError> {
        let filter_identifier_number = match filter_identifier.parse::<i64>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };

        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        let stored =
            sql_query("SELECT filter_json FROM public.user_filters WHERE user_id = $1 AND id = $2")
                .bind::<Text, _>(user_identifier)
                .bind::<BigInt, _>(filter_identifier_number)
                .get_result::<StoredFilterRecord>(&mut connection)
                .optional()
                .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(stored.map(|record| record.filter_json))
    }

    fn fetch_current_sync_stream_position(&self) -> Result<i64, DomainError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let row = sql_query(
            "SELECT MAX(stream_position) AS stream_position FROM public.room_sync_stream",
        )
        .get_result::<CurrentSyncStreamPositionRecord>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        Ok(row.stream_position.unwrap_or(0))
    }

    fn fetch_user_room_memberships(
        &self,
        user_identifier: &str,
    ) -> Result<Vec<UserRoomMembershipRecord>, DomainError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let rows = sql_query(
            "SELECT room_id, membership FROM public.room_membership_projection WHERE user_id = $1",
        )
        .bind::<Text, _>(user_identifier)
        .load::<UserRoomMembershipSqlRecord>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|row| UserRoomMembershipRecord {
                room_id: row.room_id,
                membership: row.membership,
            })
            .collect())
    }

    fn fetch_room_timeline_events_since(
        &self,
        room_identifier: &str,
        since_stream_position: i64,
        limit: usize,
    ) -> Result<Vec<TimelineEventRecord>, DomainError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let rows = sql_query(
            "SELECT e.event_id, e.event_type, e.sender_user_id, e.state_key, e.origin_server_ts, e.content_json, e.unsigned_json, s.stream_position
            FROM public.room_sync_stream s
            INNER JOIN public.room_events e ON e.event_id = s.event_id
            WHERE s.room_id = $1 AND s.stream_position > $2
            ORDER BY s.stream_position ASC
            LIMIT $3",
        )
        .bind::<Text, _>(room_identifier)
        .bind::<BigInt, _>(since_stream_position)
        .bind::<BigInt, _>(i64::try_from(limit).unwrap_or(i64::MAX))
        .load::<TimelineEventSqlRecord>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| TimelineEventRecord {
                event_id: row.event_id,
                event_type: row.event_type,
                sender_user_id: row.sender_user_id,
                state_key: row.state_key,
                origin_server_ts: row.origin_server_ts,
                content_json: row.content_json,
                unsigned_json: row.unsigned_json,
                stream_position: row.stream_position,
            })
            .collect())
    }

    fn fetch_room_state_events_since(
        &self,
        room_identifier: &str,
        since_stream_position: i64,
    ) -> Result<Vec<StateEventRecord>, DomainError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let rows = sql_query(
            "SELECT DISTINCT ON (e.event_type, COALESCE(e.state_key, ''))
                e.event_id, e.event_type, e.sender_user_id, e.state_key, e.origin_server_ts, e.content_json, e.unsigned_json
            FROM public.room_sync_stream s
            INNER JOIN public.room_events e ON e.event_id = s.event_id
            WHERE s.room_id = $1 AND s.stream_position > $2 AND e.state_key IS NOT NULL
            ORDER BY e.event_type, COALESCE(e.state_key, ''), s.stream_position DESC",
        )
        .bind::<Text, _>(room_identifier)
        .bind::<BigInt, _>(since_stream_position)
        .load::<StateEventSqlRecord>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| StateEventRecord {
                event_id: row.event_id,
                event_type: row.event_type,
                sender_user_id: row.sender_user_id,
                state_key: row.state_key,
                origin_server_ts: row.origin_server_ts,
                content_json: row.content_json,
                unsigned_json: row.unsigned_json,
            })
            .collect())
    }

    fn fetch_room_current_state_events(
        &self,
        room_identifier: &str,
    ) -> Result<Vec<StateEventRecord>, DomainError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let rows = sql_query(
            "SELECT e.event_id, e.event_type, e.sender_user_id, e.state_key, e.origin_server_ts, e.content_json, e.unsigned_json
            FROM public.room_current_state cs
            INNER JOIN public.room_events e ON e.event_id = cs.event_id
            WHERE cs.room_id = $1
            ORDER BY e.event_type ASC, e.state_key ASC",
        )
        .bind::<Text, _>(room_identifier)
        .load::<StateEventSqlRecord>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| StateEventRecord {
                event_id: row.event_id,
                event_type: row.event_type,
                sender_user_id: row.sender_user_id,
                state_key: row.state_key,
                origin_server_ts: row.origin_server_ts,
                content_json: row.content_json,
                unsigned_json: row.unsigned_json,
            })
            .collect())
    }

    fn fetch_room_timeline_prev_batch(
        &self,
        room_identifier: &str,
        since_stream_position: i64,
    ) -> Result<i64, DomainError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        let row = sql_query(
            "SELECT MIN(stream_position) AS stream_position
            FROM public.room_sync_stream
            WHERE room_id = $1 AND stream_position > $2",
        )
        .bind::<Text, _>(room_identifier)
        .bind::<BigInt, _>(since_stream_position)
        .get_result::<PrevBatchSqlRecord>(&mut connection)
        .map_err(|error| DomainError::InvalidRequest(error.to_string()))?;
        Ok(row
            .stream_position
            .unwrap_or(since_stream_position)
            .saturating_sub(1))
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
