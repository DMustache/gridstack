use serde_json::{Value, json};

use crate::services::{
    discovery::view_models::{
        WellKnownClientResponseViewModel, WellKnownServerViewModel,
        WellKnownSupportResponseViewModel,
    },
    state::ApplicationState,
};

pub fn get_well_known_client(
    application_state: &ApplicationState,
) -> WellKnownClientResponseViewModel {
    WellKnownClientResponseViewModel {
        homeserver: WellKnownServerViewModel {
            base_url: format!("http://{}", application_state.home_server_name),
        },
    }
}

pub fn get_well_known_policy_server() -> Value {
    json!({
        "m.policy_server": {
            "base_url": "https://matrix.org"
        }
    })
}

pub fn get_well_known_support(
    application_state: &ApplicationState,
) -> WellKnownSupportResponseViewModel {
    WellKnownSupportResponseViewModel {
        support_page: format!("http://{}/support", application_state.home_server_name),
    }
}
