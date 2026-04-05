use models::register::uiaa::AuthorizationData;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, validator::Validate)]
pub struct BodyInfo {
    #[serde(rename = "auth")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<AuthorizationData>,

    /// ID of the client device. If this does not correspond to a known client device, a new device will be created. The server will auto-generate a device_id if this is not specified.
    #[serde(rename = "device_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,

    /// If true, an `access_token` and `device_id` should not be returned from this call, therefore preventing an automatic login. Defaults to false.
    #[serde(rename = "inhibit_login")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inhibit_login: Option<bool>,

    /// A display name to assign to the newly-created device. Ignored if `device_id` corresponds to a known device.
    #[serde(rename = "initial_device_display_name")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_device_display_name: Option<String>,

    /// The desired password for the account.
    #[serde(rename = "password")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// If true, the client supports refresh tokens.
    #[serde(rename = "refresh_token")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<bool>,

    /// The basis for the localpart of the desired Matrix ID. If omitted, the homeserver MUST generate a Matrix ID local part.
    #[serde(rename = "username")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}
