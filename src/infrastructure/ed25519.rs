//! Ed25519 key generation and derivation shared by long-term and ephemeral identity keys.
//! Keys are encoded as unpadded standard base64, as Matrix does.

use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

#[derive(Clone, Debug)]
pub struct Ed25519KeyPair {
    pub private_seed: String,
    pub public_key: String,
}

#[must_use]
pub fn generate_key_pair() -> Ed25519KeyPair {
    let signing_key = SigningKey::generate(&mut OsRng);
    Ed25519KeyPair {
        private_seed: STANDARD_NO_PAD.encode(signing_key.to_bytes()),
        public_key: STANDARD_NO_PAD.encode(signing_key.verifying_key().to_bytes()),
    }
}

fn signing_key_from_seed(private_seed: &str) -> anyhow::Result<SigningKey> {
    let seed_bytes = STANDARD_NO_PAD
        .decode(private_seed.trim())
        .map_err(|_| anyhow::anyhow!("ed25519 seed must be unpadded standard base64"))?;
    let seed: [u8; 32] = seed_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("ed25519 seed must be exactly 32 bytes"))?;
    Ok(SigningKey::from_bytes(&seed))
}

pub fn derive_public_key(private_seed: &str) -> anyhow::Result<String> {
    Ok(STANDARD_NO_PAD.encode(
        signing_key_from_seed(private_seed)?
            .verifying_key()
            .to_bytes(),
    ))
}

/// True when `public_key` is exactly the key derived from `private_seed`.
#[must_use]
pub fn is_matching_key_pair(private_seed: &str, public_key: &str) -> bool {
    derive_public_key(private_seed).is_ok_and(|derived| derived == public_key)
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, Verifier, VerifyingKey};

    use super::*;

    const MESSAGE: &[u8] = b"fixed message";

    fn assert_signature_verifies(private_seed: &str, public_key: &str) {
        let signature = signing_key_from_seed(private_seed).unwrap().sign(MESSAGE);
        let public_bytes: [u8; 32] = STANDARD_NO_PAD
            .decode(public_key)
            .unwrap()
            .try_into()
            .unwrap();
        VerifyingKey::from_bytes(&public_bytes)
            .unwrap()
            .verify(MESSAGE, &signature)
            .unwrap();
    }

    #[test]
    fn long_term_signature_verifies_with_published_public_key() {
        let key_pair = generate_key_pair();
        assert_signature_verifies(&key_pair.private_seed, &key_pair.public_key);
        let derived = derive_public_key(&key_pair.private_seed).unwrap();
        assert_eq!(derived, key_pair.public_key);
    }

    #[test]
    fn ephemeral_signature_verifies_with_published_public_key() {
        let key_pair = generate_key_pair();
        assert_signature_verifies(&key_pair.private_seed, &key_pair.public_key);
        assert!(is_matching_key_pair(
            &key_pair.private_seed,
            &key_pair.public_key
        ));
    }

    #[test]
    fn invalid_seed_length_is_rejected() {
        let short_seed = STANDARD_NO_PAD.encode([7_u8; 16]);
        assert!(derive_public_key(&short_seed).is_err());
        assert!(derive_public_key("not base64!!").is_err());
    }

    #[test]
    fn mismatched_pair_is_detected() {
        let first = generate_key_pair();
        let second = generate_key_pair();
        assert!(!is_matching_key_pair(
            &first.private_seed,
            &second.public_key
        ));
    }
}
