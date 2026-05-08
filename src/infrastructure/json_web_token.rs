use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode, encode,
    errors::Error as JsonWebTokenLibraryError,
};
use serde::{Deserialize, Serialize};

use crate::services::traits::{Clock, JsonWebTokenAdapter};

#[derive(Debug, thiserror::Error)]
pub enum JsonWebTokenError {
    #[error("jwt error: {0}")]
    Jwt(#[from] JsonWebTokenLibraryError),
}

/// https://www.iana.org/assignments/jwt/jwt.xhtml
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonWebTokenClaims {
    // UNIX timestamp in seconds then token SHOULD NOT BE USED AFTER THIS TIME.
    pub exp: i64,
}

impl JsonWebTokenClaims {
    pub fn new<T: Clock>(expired_at: T) -> Self {
        Self {
            exp: expired_at.as_seconds(),
        }
    }
}

pub struct JsonWebToken;

impl JsonWebTokenAdapter for JsonWebToken {
    fn encode<T: Clock>(&self, expires_at: T, secret: &str) -> Result<String, JsonWebTokenError> {
        Ok(encode(
            &Header::new(jsonwebtoken::Algorithm::HS512),
            &JsonWebTokenClaims::new(expires_at),
            &EncodingKey::from_secret(secret.as_bytes()),
        )?)
    }

    fn is_token_valid(token: &str, secret: &str) -> bool {
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS512);
        validation.set_required_spec_claims(&["exp"]);
        decode::<JsonWebTokenClaims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .is_ok()
    }
}
