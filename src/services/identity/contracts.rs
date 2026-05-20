use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct PublicKeyValidityQuery {
    pub public_key: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicKeyValidityResponse {
    pub valid: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicKeyResponse {
    pub public_key: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct IdentityVersionsResponse {
    pub versions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IdentityServerStatusResponse {}

#[derive(Clone, Debug, Serialize)]
pub struct HashDetailsResponse {
    pub algorithms: Vec<String>,
    pub lookup_pepper: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AssociationLookupRequest {
    pub addresses: Vec<String>,
    pub algorithm: String,
    pub pepper: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct AssociationLookupResponse {
    pub mappings: Vec<AssociationLookupMapping>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AssociationLookupMapping {
    pub address: String,
    pub mxid: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StoreInviteRequest {
    pub medium: String,
    pub address: String,
    pub room_id: String,
    pub sender: String,
    pub room_alias: Option<String>,
    pub room_avatar_url: Option<String>,
    pub room_join_rules: Option<String>,
    pub room_name: Option<String>,
    pub sender_display_name: Option<String>,
    pub sender_avatar_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StoreInviteResponse {
    pub display_name: String,
    pub public_keys: Vec<ThirdPartyInvitePublicKey>,
    pub token: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ThirdPartyInvitePublicKey {
    pub key_validity_url: String,
    pub public_key: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SignEd25519Request {
    pub mxid: String,
    pub token: String,
    pub private_key: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SignEd25519Response {
    pub mxid: String,
    pub sender: String,
    pub signatures: serde_json::Value,
    pub token: String,
}
