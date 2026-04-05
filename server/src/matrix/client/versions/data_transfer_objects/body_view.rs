use std::collections::HashMap;

use serde::Serialize;

use crate::state::{ServerState, UnstableFeature};

#[derive(Serialize)]
pub struct BodyView {
    pub unstable_features: HashMap<String, bool>,
    pub versions: Vec<String>,
}

impl BodyView {
    pub fn new(state: &ServerState, is_authorized: bool) -> Self {
        Self {
            unstable_features: state
                .get_unstable_features()
                .into_iter()
                .map(|feature| unstable_feature_entry(feature, is_authorized))
                .collect::<HashMap<String, bool>>(),
            versions: state.get_versions(),
        }
    }
}

fn unstable_feature_entry(feature: UnstableFeature, is_authorized: bool) -> (String, bool) {
    (
        feature.name.to_owned(),
        !feature.is_authorization_required || is_authorized,
    )
}
