use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct CheckUsernameAvailableInfo {
    pub username: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckUsernameAvailableView {
    pub available: bool,
}
