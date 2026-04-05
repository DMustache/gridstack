use models::register::UserAccountType;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
pub struct QueryParameters {
    #[serde(rename = "kind", default)]
    pub kind: UserAccountType,
}
