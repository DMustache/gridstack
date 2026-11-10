//! QA tests for issue #1: `/sync` must not write on read, and idle syncs must
//! not accumulate to-device events.
//!
//! The repository double models the intended contract: one monotonic stream
//! shared by room and to-device events, and to-device delivery filtered by the
//! client's `since` token.

use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::{
    infrastructure::{
        chrono_clock::ChronoClock, configuration::AuthenticationConfiguration,
        json_web_token::JsonWebToken, server_name::ServerName,
    },
    services::{
        authorization::{
            entities::AccessSession,
            persistence::{
                AuthorizationPersistence, InMemorySessionRepository,
                access_session_storage_unit::AccessSessionStorageUnit,
            },
            service::AuthorizationService,
        },
        errors::DomainError,
        synchronization::{
            entities::{SyncBatch, SyncRequest},
            persistence::{
                FilterRepository, StateEventRecord, TimelineEventRecord, UserRoomMembershipRecord,
            },
            service::SyncronizationService,
        },
    },
};

const SYNCING_USER: &str = "@alice:localhost";
const OTHER_USER: &str = "@bob:localhost";
const SERVER_INTERNAL_ACCOUNT_DATA_TYPE: &str = "org.matrix.msc3575.sync_stream_position";
const SERVER_INTERNAL_TO_DEVICE_TYPE: &str = "org.matrix.gridstack.sync.hint";

#[derive(Clone, Debug)]
struct StoredToDeviceEvent {
    stream_position: i64,
    recipient_user_identifier: String,
    sender_user_identifier: String,
    event_type: String,
    content: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RecordedWrite {
    GlobalAccountData { event_type: String },
    ToDeviceEnqueue { event_type: String },
    Presence,
}

#[derive(Default)]
struct RecordingFilterRepository {
    current_stream_position: Mutex<i64>,
    to_device_events: Mutex<Vec<StoredToDeviceEvent>>,
    global_account_data: Mutex<Vec<(String, String, Value)>>,
    recorded_writes: Mutex<Vec<RecordedWrite>>,
}

impl RecordingFilterRepository {
    fn advance_stream(&self) -> i64 {
        let mut current_stream_position = self
            .current_stream_position
            .lock()
            .expect("stream position lock");
        *current_stream_position += 1;
        *current_stream_position
    }

    fn record(&self, write: RecordedWrite) {
        self.recorded_writes
            .lock()
            .expect("recorded writes lock")
            .push(write);
    }

    fn recorded_writes(&self) -> Vec<RecordedWrite> {
        self.recorded_writes
            .lock()
            .expect("recorded writes lock")
            .clone()
    }

    /// Simulates another user sending a to-device message, as
    /// `PUT /sendToDevice` would. Not recorded as a sync-side write.
    fn deliver_to_device_event_from_other_user(&self, event_type: &str, content: Value) {
        let stream_position = self.advance_stream();
        self.to_device_events
            .lock()
            .expect("to-device lock")
            .push(StoredToDeviceEvent {
                stream_position,
                recipient_user_identifier: SYNCING_USER.to_owned(),
                sender_user_identifier: OTHER_USER.to_owned(),
                event_type: event_type.to_owned(),
                content,
            });
    }
}

impl FilterRepository for RecordingFilterRepository {
    fn create_filter(
        &self,
        _user_identifier: &str,
        _filter_payload: Value,
    ) -> Result<String, DomainError> {
        Ok("0".to_owned())
    }

    fn fetch_filter(
        &self,
        _user_identifier: &str,
        _filter_identifier: &str,
    ) -> Result<Option<Value>, DomainError> {
        Ok(None)
    }

    fn fetch_current_sync_stream_position(&self) -> Result<i64, DomainError> {
        Ok(*self
            .current_stream_position
            .lock()
            .expect("stream position lock"))
    }

    fn fetch_user_room_memberships(
        &self,
        _user_identifier: &str,
    ) -> Result<Vec<UserRoomMembershipRecord>, DomainError> {
        Ok(Vec::new())
    }

    fn fetch_room_timeline_events_since(
        &self,
        _room_identifier: &str,
        _since_stream_position: i64,
        _limit: usize,
    ) -> Result<Vec<TimelineEventRecord>, DomainError> {
        Ok(Vec::new())
    }

    fn fetch_room_state_events_since(
        &self,
        _room_identifier: &str,
        _since_stream_position: i64,
    ) -> Result<Vec<StateEventRecord>, DomainError> {
        Ok(Vec::new())
    }

    fn fetch_room_current_state_events(
        &self,
        _room_identifier: &str,
    ) -> Result<Vec<StateEventRecord>, DomainError> {
        Ok(Vec::new())
    }

    fn fetch_room_timeline_prev_batch(
        &self,
        _room_identifier: &str,
        since_stream_position: i64,
    ) -> Result<i64, DomainError> {
        Ok(since_stream_position)
    }

    fn fetch_room_ephemeral_receipt_events_since(
        &self,
        _room_identifier: &str,
        _user_identifier: &str,
        _since_stream_position: i64,
    ) -> Result<Vec<Value>, DomainError> {
        Ok(Vec::new())
    }

    fn fetch_room_account_data_events(
        &self,
        _room_identifier: &str,
        _user_identifier: &str,
        _since_stream_position: i64,
        _full_state: bool,
    ) -> Result<Vec<Value>, DomainError> {
        Ok(Vec::new())
    }

    fn upsert_presence(&self, _user_identifier: &str, _presence: &str) -> Result<(), DomainError> {
        self.record(RecordedWrite::Presence);
        Ok(())
    }

    fn fetch_presence_events_for_users(
        &self,
        _user_identifiers: &[String],
    ) -> Result<Vec<Value>, DomainError> {
        Ok(Vec::new())
    }

    fn upsert_global_account_data_event(
        &self,
        user_identifier: &str,
        event_type: &str,
        content: Value,
    ) -> Result<(), DomainError> {
        self.record(RecordedWrite::GlobalAccountData {
            event_type: event_type.to_owned(),
        });
        let mut global_account_data = self.global_account_data.lock().expect("account data lock");
        global_account_data.retain(|(stored_user, stored_type, _)| {
            stored_user != user_identifier || stored_type != event_type
        });
        global_account_data.push((user_identifier.to_owned(), event_type.to_owned(), content));
        drop(global_account_data);
        Ok(())
    }

    fn fetch_global_account_data_events(
        &self,
        user_identifier: &str,
    ) -> Result<Vec<Value>, DomainError> {
        Ok(self
            .global_account_data
            .lock()
            .expect("account data lock")
            .iter()
            .filter(|(stored_user, _, _)| stored_user == user_identifier)
            .map(|(_, event_type, content)| json!({ "type": event_type, "content": content }))
            .collect())
    }

    fn enqueue_to_device_event(
        &self,
        user_identifier: &str,
        event_type: &str,
        content: Value,
    ) -> Result<i64, DomainError> {
        self.record(RecordedWrite::ToDeviceEnqueue {
            event_type: event_type.to_owned(),
        });
        let stream_position = self.advance_stream();
        self.to_device_events
            .lock()
            .expect("to-device lock")
            .push(StoredToDeviceEvent {
                stream_position,
                recipient_user_identifier: user_identifier.to_owned(),
                sender_user_identifier: user_identifier.to_owned(),
                event_type: event_type.to_owned(),
                content,
            });
        Ok(stream_position)
    }

    fn fetch_to_device_events_since(
        &self,
        user_identifier: &str,
        since_stream_position: i64,
    ) -> Result<Vec<Value>, DomainError> {
        Ok(self
            .to_device_events
            .lock()
            .expect("to-device lock")
            .iter()
            .filter(|event| {
                event.recipient_user_identifier == user_identifier
                    && event.stream_position > since_stream_position
            })
            .map(|event| {
                json!({
                    "type": event.event_type,
                    "sender": event.sender_user_identifier,
                    "content": event.content,
                })
            })
            .collect())
    }
}

struct SyncTestHarness {
    repository: Arc<RecordingFilterRepository>,
    service: SyncronizationService,
    access_session: AccessSessionStorageUnit,
}

impl SyncTestHarness {
    fn new() -> Self {
        let repository = Arc::new(RecordingFilterRepository::default());
        let server_name = ServerName::try_new("localhost").expect("valid server name");
        let session_storage_path = std::env::temp_dir().join(format!(
            "gridstack-sync-test-sessions-{}.tsv",
            uuid::Uuid::new_v4()
        ));
        let authorization_service = Arc::new(AuthorizationService::new(
            // Pool is built unchecked: no connection is opened by these tests.
            Arc::new(AuthorizationPersistence::new(
                "postgres://unused:unused@127.0.0.1:1/unused",
            )),
            Arc::new(InMemorySessionRepository::from_storage_path(
                session_storage_path,
            )),
            Arc::new(JsonWebToken),
            Arc::new(ChronoClock),
            &server_name,
            AuthenticationConfiguration {
                allow_registration: false,
                access_token_expiry_seconds: 3600,
                refresh_token_expiry_seconds: 3600,
                json_web_token_secret: "test-only-secret".to_owned(),
                password_pepper: "test-only-pepper".to_owned(),
                appservices: Vec::new(),
            },
        ));
        let access_session: AccessSession = serde_json::from_value(json!({
            "access_token": "test-access-token",
            "user_identifier": SYNCING_USER,
            "device_id": "TESTDEVICE",
            "expires_at_seconds": i64::MAX,
        }))
        .expect("valid access session");

        Self {
            service: SyncronizationService::new(
                Arc::clone(&repository) as Arc<dyn FilterRepository>,
                authorization_service,
            ),
            repository,
            access_session: access_session.into(),
        }
    }

    async fn sync(&self, since: Option<&str>) -> SyncBatch {
        self.service
            .sync(
                &self.access_session,
                SyncRequest {
                    filter: None,
                    since: since.map(str::to_owned),
                    full_state: false,
                    set_presence: None,
                    timeout_milliseconds: 0,
                    use_state_after: false,
                    timeline_limit: 10,
                },
            )
            .await
            .expect("sync succeeds")
    }

    /// Runs `count` syncs, each using the previous `next_batch`, like a client.
    async fn sync_repeatedly(&self, count: usize) -> Vec<SyncBatch> {
        let mut batches: Vec<SyncBatch> = Vec::with_capacity(count);
        for _ in 0..count {
            let since = batches.last().map(|batch| batch.next_batch.clone());
            batches.push(self.sync(since.as_deref()).await);
        }
        batches
    }
}

fn event_types(events: &[Value]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| event.get("type").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// Presence is excluded on purpose: the spec marks the user online when
/// `set_presence` is omitted, so a presence write is expected.
#[tokio::test]
async fn sync_writes_no_internal_account_data_or_to_device_events() {
    let harness = SyncTestHarness::new();

    harness.sync(None).await;

    let unexpected_writes: Vec<RecordedWrite> = harness
        .repository
        .recorded_writes()
        .into_iter()
        .filter(|write| *write != RecordedWrite::Presence)
        .collect();
    assert_eq!(
        unexpected_writes,
        Vec::new(),
        "GET /sync must not write account data or enqueue to-device events"
    );
}

#[tokio::test]
async fn idle_syncs_return_no_to_device_events() {
    let harness = SyncTestHarness::new();

    let batches = harness.sync_repeatedly(5).await;

    let to_device_counts: Vec<usize> = batches
        .iter()
        .map(|batch| batch.to_device.events.len())
        .collect();
    assert_eq!(
        to_device_counts,
        vec![0; 5],
        "nobody sent a to-device message, so no sync may return one"
    );
}

#[tokio::test]
async fn sync_never_exposes_server_internal_events() {
    let harness = SyncTestHarness::new();

    let batches = harness.sync_repeatedly(3).await;

    for batch in &batches {
        assert!(
            !event_types(&batch.to_device.events)
                .contains(&SERVER_INTERNAL_TO_DEVICE_TYPE.to_owned()),
            "internal sync hint leaked into to_device"
        );
        assert!(
            !event_types(&batch.account_data.events)
                .contains(&SERVER_INTERNAL_ACCOUNT_DATA_TYPE.to_owned()),
            "internal stream-position marker leaked into account_data"
        );
    }
}

/// Guard for the fix: removing the hint must not break real to-device delivery.
#[tokio::test]
async fn to_device_event_is_delivered_exactly_once() {
    let harness = SyncTestHarness::new();
    harness
        .repository
        .deliver_to_device_event_from_other_user("m.room_key_request", json!({ "action": "x" }));

    let batches = harness.sync_repeatedly(3).await;

    let deliveries: Vec<usize> = batches
        .iter()
        .map(|batch| {
            event_types(&batch.to_device.events)
                .iter()
                .filter(|event_type| *event_type == "m.room_key_request")
                .count()
        })
        .collect();
    assert_eq!(
        deliveries,
        vec![1, 0, 0],
        "a to-device event must arrive on the first sync and never again"
    );
}
