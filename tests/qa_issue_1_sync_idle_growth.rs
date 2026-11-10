//! Black-box QA for issue #1: idle `/sync` must not accumulate to-device
//! events, grow its response, or expose server-internal events.
//!
//! Runs against a live server and database, so it is ignored by default.
//!
//! Required:
//! - `GRIDSTACK_QA_BASE_URL`, for example `http://127.0.0.1:3000`
//!
//! The server rate-limits to 120 requests per minute per client, so run these
//! tests at most a few times per minute.

use std::time::Duration;

use serde_json::{Value, json};

const SERVER_INTERNAL_ACCOUNT_DATA_TYPE: &str = "org.matrix.msc3575.sync_stream_position";
const SERVER_INTERNAL_TO_DEVICE_TYPE: &str = "org.matrix.gridstack.sync.hint";
const IDLE_SYNC_COUNT: usize = 8;

struct QaClient {
    http: reqwest::Client,
    base_url: String,
    access_token: String,
}

impl QaClient {
    async fn register_fresh_user() -> Self {
        let base_url = std::env::var("GRIDSTACK_QA_BASE_URL")
            .expect("set GRIDSTACK_QA_BASE_URL to a running gridstack server");
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("http client");
        let register_url = format!("{base_url}/_matrix/client/v3/register");
        let username = format!("qa_issue1_{}", uuid::Uuid::new_v4().simple());
        let password = "Str0ngPassw0rd!x";

        let challenge: Value = http
            .post(&register_url)
            .json(&json!({ "username": username, "password": password }))
            .send()
            .await
            .expect("register challenge request")
            .json()
            .await
            .expect("register challenge body");
        let session = challenge["session"]
            .as_str()
            .unwrap_or_else(|| panic!("no interactive-auth session in {challenge}"));

        let registered: Value = http
            .post(&register_url)
            .json(&json!({
                "username": username,
                "password": password,
                "auth": { "type": "m.login.password", "session": session },
            }))
            .send()
            .await
            .expect("register request")
            .json()
            .await
            .expect("register body");
        let access_token = registered["access_token"]
            .as_str()
            .unwrap_or_else(|| panic!("no access_token in {registered}"))
            .to_owned();

        Self {
            http,
            base_url,
            access_token,
        }
    }

    /// Returns the parsed body and its size in bytes.
    async fn sync(&self, since: Option<&str>) -> (Value, usize) {
        let mut url = format!("{}/_matrix/client/v3/sync?timeout=0", self.base_url);
        if let Some(since) = since {
            url.push_str("&since=");
            url.push_str(since);
        }
        let response = self
            .http
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .expect("sync request");
        let status = response.status();
        let body = response.bytes().await.expect("sync body");
        let parsed: Value = serde_json::from_slice(&body).expect("sync json");
        assert!(status.is_success(), "sync failed with {status}: {parsed}");
        (parsed, body.len())
    }

    /// Runs `count` syncs, each using the previous `next_batch`, like a client.
    async fn sync_repeatedly(&self, count: usize) -> Vec<(Value, usize)> {
        let mut responses: Vec<(Value, usize)> = Vec::with_capacity(count);
        for _ in 0..count {
            let since = responses
                .last()
                .and_then(|(body, _)| body["next_batch"].as_str().map(str::to_owned));
            responses.push(self.sync(since.as_deref()).await);
        }
        responses
    }
}

fn events_of_type(events: &Value, event_type: &str) -> usize {
    events.as_array().map_or(0, |events| {
        events
            .iter()
            .filter(|event| event["type"].as_str() == Some(event_type))
            .count()
    })
}

#[tokio::test]
#[ignore = "needs a running server: set GRIDSTACK_QA_BASE_URL"]
async fn idle_syncs_do_not_accumulate_to_device_events() {
    let client = QaClient::register_fresh_user().await;

    let responses = client.sync_repeatedly(IDLE_SYNC_COUNT).await;

    let to_device_counts: Vec<usize> = responses
        .iter()
        .map(|(body, _)| body["to_device"]["events"].as_array().map_or(0, Vec::len))
        .collect();
    assert_eq!(
        to_device_counts,
        vec![0; IDLE_SYNC_COUNT],
        "nobody sent this user a to-device message"
    );
}

#[tokio::test]
#[ignore = "needs a running server: set GRIDSTACK_QA_BASE_URL"]
async fn idle_sync_response_size_is_stable() {
    let client = QaClient::register_fresh_user().await;

    let responses = client.sync_repeatedly(IDLE_SYNC_COUNT).await;

    let sizes: Vec<usize> = responses.iter().map(|(_, size)| *size).collect();
    assert!(
        sizes.windows(2).skip(1).all(|pair| pair[0] == pair[1]),
        "idle sync responses changed size after the first sync: {sizes:?} bytes"
    );
}

#[tokio::test]
#[ignore = "needs a running server: set GRIDSTACK_QA_BASE_URL"]
async fn sync_does_not_expose_server_internal_events() {
    let client = QaClient::register_fresh_user().await;

    let responses = client.sync_repeatedly(3).await;

    for (body, _) in &responses {
        assert_eq!(
            events_of_type(&body["to_device"]["events"], SERVER_INTERNAL_TO_DEVICE_TYPE),
            0,
            "internal sync hint leaked into to_device: {body}"
        );
        assert_eq!(
            events_of_type(
                &body["account_data"]["events"],
                SERVER_INTERNAL_ACCOUNT_DATA_TYPE
            ),
            0,
            "internal stream-position marker leaked into account_data: {body}"
        );
    }
}
