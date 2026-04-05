use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyView {
    /// An access token for the account. This access token can then be used to authorize other requests. Required if the `inhibit_login` option is false.
    #[serde(rename = "access_token")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,

    // ID of the registered device. Will be the same as the corresponding parameter in the request, if one was specified. Required if the `inhibit_login` option is false.
    #[serde(rename = "device_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,

    // The lifetime of the access token, in milliseconds. Once the access token has expired a new access token can be obtained by using the provided refresh token. If no refresh token is provided, the client will need to re-log in to obtain a new access token. If not given, the client can assume that the access token will not expire.  Omitted if the `inhibit_login` option is true.
    #[serde(rename = "expires_in_ms")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in_ms: Option<i32>,

    /// The server_name of the homeserver on which the account has been registered.  **Deprecated**. Clients should extract the server_name from `user_id` (by splitting at the first colon) if they require it. Note also that `homeserver` is not spelt this way.
    #[serde(rename = "home_server")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_server: Option<String>,

    // A refresh token for the account. This token can be used to obtain a new access token when it expires by calling the `/refresh` endpoint.  Omitted if the `inhibit_login` option is true.
    #[serde(rename = "refresh_token")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    // A fully-qualified Matrix user ID (MXID) that has been registered.  Any user ID returned by this API must conform to the grammar given in the [Matrix specification](https://spec.matrix.org/unstable/appendices/#user-identifiers).
    #[serde(rename = "user_id")]
    pub user_id: String,
}
