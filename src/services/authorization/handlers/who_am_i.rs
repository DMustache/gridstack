use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct WhoAmIView {
    #[serde(rename = "user_id")]
    pub user_id: String,

    #[serde(rename = "is_guest")]
    pub is_guest: bool,

    #[serde(rename = "device_id", skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}
