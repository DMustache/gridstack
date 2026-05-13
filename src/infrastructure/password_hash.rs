use argon2::{
    Argon2, PasswordHash as ParsedPasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::SaltString,
};
use uuid::Uuid;

pub struct PasswordHash;

impl PasswordHash {
    pub fn encode(raw_password: &str, pepper: &str) -> anyhow::Result<String> {
        let salt = SaltString::encode_b64(&Uuid::new_v4().into_bytes())
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let password_with_pepper = format!("{pepper}{raw_password}");
        let encoded = Argon2::default()
            .hash_password(password_with_pepper.as_bytes(), &salt)
            .map_err(|error| anyhow::anyhow!(error.to_string()))?
            .to_string();
        Ok(encoded)
    }

    pub fn verify(raw_password: &str, pepper: &str, encoded_hash: &str) -> bool {
        let Ok(parsed_hash) = ParsedPasswordHash::new(encoded_hash) else {
            return false;
        };
        let password_with_pepper = format!("{pepper}{raw_password}");
        Argon2::default()
            .verify_password(password_with_pepper.as_bytes(), &parsed_hash)
            .is_ok()
    }
}
