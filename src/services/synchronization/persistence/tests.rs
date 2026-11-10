//! L4 persistence tests for to-device storage. They need a migrated Postgres:
//! `DATABASE_URL=... cargo test -- --ignored`.

use serde_json::{Value, json};

use super::{FilterRepository, SyncronizationPersistence};

const SENDER_USER: &str = "@sender:localhost";

fn persistence_from_environment() -> SyncronizationPersistence {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SyncronizationPersistence::new(&database_url)
}

fn unique_recipient_user() -> String {
    format!("@recipient-{}:localhost", uuid::Uuid::new_v4())
}

fn event_types(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| event.get("type").and_then(Value::as_str))
        .collect()
}

#[test]
#[ignore = "needs DATABASE_URL"]
fn enqueued_to_device_event_advances_shared_sync_stream() {
    let persistence = persistence_from_environment();
    let recipient_user = unique_recipient_user();

    let stream_position_before = persistence
        .fetch_current_sync_stream_position()
        .expect("current position");
    let enqueued_position = persistence
        .enqueue_to_device_event(&recipient_user, "DEVICE", SENDER_USER, "m.test", json!({}))
        .expect("enqueue");

    assert!(enqueued_position > stream_position_before);
    assert!(
        persistence
            .fetch_current_sync_stream_position()
            .expect("current position")
            >= enqueued_position
    );
}

#[test]
#[ignore = "needs DATABASE_URL"]
fn fetch_deletes_only_acknowledged_rows_of_that_device() {
    let persistence = persistence_from_environment();
    let recipient_user = unique_recipient_user();
    let enqueue = |device: &str, event_type: &str| {
        persistence
            .enqueue_to_device_event(&recipient_user, device, SENDER_USER, event_type, json!({}))
            .expect("enqueue")
    };
    let first_position = enqueue("DEVICE_A", "m.first");
    enqueue("DEVICE_A", "m.second");
    enqueue("DEVICE_B", "m.other_device");

    let after_acknowledgement = persistence
        .fetch_to_device_events_since(&recipient_user, "DEVICE_A", first_position)
        .expect("fetch since first");
    assert_eq!(event_types(&after_acknowledgement), vec!["m.second"]);
    assert_eq!(after_acknowledgement[0]["sender"], json!(SENDER_USER));

    let first_device_remaining = persistence
        .fetch_to_device_events_since(&recipient_user, "DEVICE_A", 0)
        .expect("fetch device A");
    assert_eq!(event_types(&first_device_remaining), vec!["m.second"]);

    let second_device_remaining = persistence
        .fetch_to_device_events_since(&recipient_user, "DEVICE_B", 0)
        .expect("fetch device B");
    assert_eq!(
        event_types(&second_device_remaining),
        vec!["m.other_device"]
    );
}
