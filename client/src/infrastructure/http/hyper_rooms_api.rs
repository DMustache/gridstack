use async_trait::async_trait;
use http::Method;

use crate::{
    application::ports::RoomsApi,
    domain::{
        error::ClientError,
        rooms::{
            CreateRoomInfo, CreateRoomView, GetRoomMessagesQuery, GetRoomMessagesView,
            JoinRoomInfo, JoinRoomView, JoinedRoomsView, LeaveRoomInfo, LeaveRoomView,
            RoomStateEventView, SendRoomEventView,
        },
    },
    infrastructure::http::matrix_http_client::MatrixHttpClient,
};

pub struct HyperRoomsApi {
    matrix_http_client: MatrixHttpClient,
}

impl HyperRoomsApi {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            matrix_http_client: MatrixHttpClient::new(base_url),
        }
    }
}

#[async_trait]
impl RoomsApi for HyperRoomsApi {
    async fn create_room(
        &self,
        access_token: &str,
        request: &CreateRoomInfo,
    ) -> Result<CreateRoomView, ClientError> {
        self.matrix_http_client
            .send_json(
                Method::POST,
                "/_matrix/client/v3/createRoom",
                Some(access_token),
                Some(request),
            )
            .await
    }

    async fn join_room_by_id(
        &self,
        access_token: &str,
        room_id: &str,
        request: &JoinRoomInfo,
    ) -> Result<JoinRoomView, ClientError> {
        let path = format!("/_matrix/client/v3/rooms/{room_id}/join");
        self.matrix_http_client
            .send_json(Method::POST, &path, Some(access_token), Some(request))
            .await
    }

    async fn leave_room_by_id(
        &self,
        access_token: &str,
        room_id: &str,
        request: &LeaveRoomInfo,
    ) -> Result<LeaveRoomView, ClientError> {
        let path = format!("/_matrix/client/v3/rooms/{room_id}/leave");
        self.matrix_http_client
            .send_json(Method::POST, &path, Some(access_token), Some(request))
            .await
    }

    async fn get_room_state(
        &self,
        access_token: &str,
        room_id: &str,
    ) -> Result<Vec<RoomStateEventView>, ClientError> {
        let path = format!("/_matrix/client/v3/rooms/{room_id}/state");
        self.matrix_http_client
            .send_json::<(), Vec<RoomStateEventView>>(Method::GET, &path, Some(access_token), None)
            .await
    }

    async fn get_room_messages(
        &self,
        access_token: &str,
        room_id: &str,
        query: &GetRoomMessagesQuery,
    ) -> Result<GetRoomMessagesView, ClientError> {
        let mut query_fields = vec![
            format!("dir={}", query.direction.as_query_value()),
            format!("limit={}", query.limit),
        ];

        if let Some(from_token) = query.from_token.as_deref() {
            query_fields.push(format!("from={}", percent_encode_query_value(from_token)));
        }
        if let Some(to_token) = query.to_token.as_deref() {
            query_fields.push(format!("to={}", percent_encode_query_value(to_token)));
        }
        if let Some(filter) = query.filter.as_ref() {
            let serialized_filter = serde_json::to_string(filter)
                .map_err(|error| ClientError::Serialization(error.to_string()))?;
            query_fields.push(format!(
                "filter={}",
                percent_encode_query_value(&serialized_filter)
            ));
        }

        let path = format!(
            "/_matrix/client/v3/rooms/{room_id}/messages?{}",
            query_fields.join("&")
        );

        self.matrix_http_client
            .send_json_with_query::<(), GetRoomMessagesView>(
                Method::GET,
                &path,
                Some(access_token),
                None,
            )
            .await
    }

    async fn get_joined_rooms(&self, access_token: &str) -> Result<JoinedRoomsView, ClientError> {
        self.matrix_http_client
            .send_json::<(), JoinedRoomsView>(
                Method::GET,
                "/_matrix/client/v3/joined_rooms",
                Some(access_token),
                None,
            )
            .await
    }

    async fn send_room_message_event(
        &self,
        access_token: &str,
        room_id: &str,
        event_type: &str,
        transaction_id: &str,
        content: &serde_json::Value,
    ) -> Result<SendRoomEventView, ClientError> {
        let path = format!(
            "/_matrix/client/v3/rooms/{room_id}/send/{event_type}/{transaction_id}"
        );
        self.matrix_http_client
            .send_json(Method::PUT, &path, Some(access_token), Some(content))
            .await
    }
}

fn percent_encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}
