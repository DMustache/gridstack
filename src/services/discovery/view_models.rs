use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct WellKnownClientResponseViewModel {
    #[serde(rename = "m.homeserver")]
    pub homeserver: WellKnownServerViewModel,
}

#[derive(Clone, Debug, Serialize)]
pub struct WellKnownServerViewModel {
    #[serde(rename = "base_url")]
    pub base_url: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct WellKnownSupportResponseViewModel {
    #[serde(rename = "support_page")]
    pub support_page: String,
}
