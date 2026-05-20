use axum_extra::routing::TypedPath;

#[derive(TypedPath)]
#[typed_path("/_matrix/identity/v2/pubkey/ephemeral/isvalid")]
pub struct IdentityV2PublicKeyEphemeralIsValidPath;

#[derive(TypedPath)]
#[typed_path("/_matrix/identity/v2/pubkey/isvalid")]
pub struct IdentityV2PublicKeyIsValidPath;

#[derive(TypedPath)]
#[typed_path("/_matrix/identity/api/v1/pubkey/isvalid")]
pub struct IdentityLegacyV1PublicKeyIsValidPath;
