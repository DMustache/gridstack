use async_trait::async_trait;
use http::Method;

use crate::{
    application::ports::RoomsApi,
    domain::{
        error::ClientError,
        rooms::{
            CreateRoomInfo, CreateRoomView, GetRoomEventView, GetRoomMembersQuery,
            GetRoomMembersView, GetRoomMessagesQuery, GetRoomMessagesView, InviteUserInfo,
            InviteUserView, JoinRoomInfo, JoinRoomView, JoinedMembersView, JoinedRoomsView,
            LeaveRoomInfo, LeaveRoomView, RoomStateEventView, SendReceiptInfo, SendReceiptView,
            SendRoomEventView, SetReadMarkersInfo, SetReadMarkersView, SetRoomStateWithKeyView,
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
            "/_matrix/client/v3/rooms/{}/send/{}/{}",
            percent_encode_path_segment(room_id),
            percent_encode_path_segment(event_type),
            percent_encode_path_segment(transaction_id),
        );
        self.matrix_http_client
            .send_json(Method::PUT, &path, Some(access_token), Some(content))
            .await
    }

    async fn get_joined_members(
        &self,
        access_token: &str,
        room_id: &str,
    ) -> Result<JoinedMembersView, ClientError> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/joined_members",
            percent_encode_path_segment(room_id)
        );
        self.matrix_http_client
            .send_json::<(), JoinedMembersView>(Method::GET, &path, Some(access_token), None)
            .await
    }

    async fn get_room_members(
        &self,
        access_token: &str,
        room_id: &str,
        query: &GetRoomMembersQuery,
    ) -> Result<GetRoomMembersView, ClientError> {
        let mut query_fields = Vec::new();
        if let Some(at_token) = query.at_token.as_deref() {
            query_fields.push(format!("at={}", percent_encode_query_value(at_token)));
        }
        if let Some(membership) = query.membership.as_deref() {
            query_fields.push(format!(
                "membership={}",
                percent_encode_query_value(membership)
            ));
        }
        if let Some(not_membership) = query.not_membership.as_deref() {
            query_fields.push(format!(
                "not_membership={}",
                percent_encode_query_value(not_membership)
            ));
        }
        let mut path = format!(
            "/_matrix/client/v3/rooms/{}/members",
            percent_encode_path_segment(room_id)
        );
        if !query_fields.is_empty() {
            path = format!("{path}?{}", query_fields.join("&"));
        }

        self.matrix_http_client
            .send_json_with_query::<(), GetRoomMembersView>(Method::GET, &path, Some(access_token), None)
            .await
    }

    async fn invite_user_to_room(
        &self,
        access_token: &str,
        room_id: &str,
        request: &InviteUserInfo,
    ) -> Result<InviteUserView, ClientError> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/invite",
            percent_encode_path_segment(room_id)
        );
        self.matrix_http_client
            .send_json(Method::POST, &path, Some(access_token), Some(request))
            .await
    }

    async fn get_room_event(
        &self,
        access_token: &str,
        room_id: &str,
        event_id: &str,
    ) -> Result<GetRoomEventView, ClientError> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/event/{}",
            percent_encode_path_segment(room_id),
            percent_encode_path_segment(event_id)
        );
        self.matrix_http_client
            .send_json::<(), GetRoomEventView>(Method::GET, &path, Some(access_token), None)
            .await
    }

    async fn get_room_state_with_key(
        &self,
        access_token: &str,
        room_id: &str,
        event_type: &str,
        state_key: Option<&str>,
        request_full_event: bool,
    ) -> Result<serde_json::Value, ClientError> {
        let mut path = if let Some(state_key_value) = state_key {
            format!(
                "/_matrix/client/v3/rooms/{}/state/{}/{}",
                percent_encode_path_segment(room_id),
                percent_encode_path_segment(event_type),
                percent_encode_path_segment(state_key_value),
            )
        } else {
            format!(
                "/_matrix/client/v3/rooms/{}/state/{}",
                percent_encode_path_segment(room_id),
                percent_encode_path_segment(event_type),
            )
        };

        if request_full_event {
            path = format!("{path}?format=event");
        }

        self.matrix_http_client
            .send_json_with_query::<(), serde_json::Value>(Method::GET, &path, Some(access_token), None)
            .await
    }

    async fn set_room_state_with_key(
        &self,
        access_token: &str,
        room_id: &str,
        event_type: &str,
        state_key: &str,
        content: &serde_json::Value,
    ) -> Result<SetRoomStateWithKeyView, ClientError> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/state/{}/{}",
            percent_encode_path_segment(room_id),
            percent_encode_path_segment(event_type),
            percent_encode_path_segment(state_key),
        );

        self.matrix_http_client
            .send_json(Method::PUT, &path, Some(access_token), Some(content))
            .await
    }

    async fn send_room_receipt(
        &self,
        access_token: &str,
        room_id: &str,
        receipt_type: &str,
        event_id: &str,
        request: &SendReceiptInfo,
    ) -> Result<SendReceiptView, ClientError> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/receipt/{}/{}",
            percent_encode_path_segment(room_id),
            percent_encode_path_segment(receipt_type),
            percent_encode_path_segment(event_id),
        );

        self.matrix_http_client
            .send_json(Method::POST, &path, Some(access_token), Some(request))
            .await
    }

    async fn set_room_read_markers(
        &self,
        access_token: &str,
        room_id: &str,
        request: &SetReadMarkersInfo,
    ) -> Result<SetReadMarkersView, ClientError> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/read_markers",
            percent_encode_path_segment(room_id)
        );
        self.matrix_http_client
            .send_json(Method::POST, &path, Some(access_token), Some(request))
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

fn percent_encode_path_segment(value: &str) -> String {
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
