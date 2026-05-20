use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use diesel::{
    BoolExpressionMethods, Connection, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
    dsl::{exists, select},
    insert_into,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
    upsert::excluded,
};
use uuid::Uuid;

use crate::{
    infrastructure::{configuration::IdentityConfiguration, schema},
    services::identity::{
        entities::{
            EncodedPublicKey, IdentitySigningAlgorithm, IdentitySigningKey, IdentitySigningKeyId,
            IdentitySigningKeyUsage, StoredThirdPartyInvite,
        },
        errors::IdentityServiceError,
        service::{
            IdentityEphemeralKeyGenerator, IdentityInvitationRepository,
            IdentitySigningKeyRepository, StoreThirdPartyInviteCommand,
        },
    },
};

#[derive(Clone, Debug)]
struct ConfiguredLongTermKeyMaterial {
    key_id: String,
    public_key: String,
    private_key: String,
}

#[derive(Clone)]
pub struct IdentityPersistence {
    connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    configured_long_term_key_material: Option<ConfiguredLongTermKeyMaterial>,
    default_long_term_key_id: String,
    ephemeral_key_validity_seconds: i64,
}

impl IdentityPersistence {
    #[must_use]
    pub fn new(database_url: &str, identity_configuration: &IdentityConfiguration) -> Self {
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let connection_pool = r2d2::Pool::builder().build_unchecked(manager);

        Self {
            connection_pool,
            configured_long_term_key_material: configured_long_term_key_material(
                identity_configuration,
            ),
            default_long_term_key_id: identity_configuration.long_term_key_id.clone(),
            ephemeral_key_validity_seconds: i64::try_from(
                identity_configuration.ephemeral_key_validity_seconds,
            )
            .unwrap_or(7 * 24 * 60 * 60),
        }
    }

    pub fn ensure_long_term_key_material(
        &self,
    ) -> Result<IdentitySigningKey, IdentityServiceError> {
        if let Some(configured_material) = &self.configured_long_term_key_material {
            self.upsert_long_term_key_material(configured_material)?;
            return Ok(IdentitySigningKey {
                key_id: IdentitySigningKeyId::parse(configured_material.key_id.clone())?,
                public_key: EncodedPublicKey::parse(configured_material.public_key.clone())?,
                usage: IdentitySigningKeyUsage::LongTerm,
            });
        }

        if let Some(existing_key) = self.fetch_latest_long_term_key()? {
            return Ok(existing_key);
        }

        let generated_key_material = self.generate_long_term_key_material()?;
        self.upsert_long_term_key_material(&generated_key_material)?;

        Ok(IdentitySigningKey {
            key_id: IdentitySigningKeyId::parse(generated_key_material.key_id)?,
            public_key: EncodedPublicKey::parse(generated_key_material.public_key)?,
            usage: IdentitySigningKeyUsage::LongTerm,
        })
    }

    fn fetch_latest_long_term_key(
        &self,
    ) -> Result<Option<IdentitySigningKey>, IdentityServiceError> {
        use schema::identity_signing_keys;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;
        let now = Utc::now();

        let row = identity_signing_keys::table
            .select((
                identity_signing_keys::key_id,
                identity_signing_keys::public_key,
            ))
            .filter(identity_signing_keys::usage.eq("long_term"))
            .filter(
                identity_signing_keys::expires_at
                    .is_null()
                    .or(identity_signing_keys::expires_at.gt(now)),
            )
            .order(identity_signing_keys::created_at.desc())
            .first::<(Option<String>, String)>(&mut connection)
            .optional()
            .map_err(database_error_to_identity_error)?;

        row.map(long_term_key_from_columns).transpose()
    }

    fn fetch_active_long_term_key(&self) -> Result<IdentitySigningKey, IdentityServiceError> {
        self.fetch_latest_long_term_key()?.ok_or_else(|| {
            IdentityServiceError::Internal(anyhow::anyhow!(
                "missing long-term identity signing key"
            ))
        })
    }

    fn upsert_long_term_key_material(
        &self,
        key_material: &ConfiguredLongTermKeyMaterial,
    ) -> Result<(), IdentityServiceError> {
        use schema::identity_signing_keys;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;

        insert_into(identity_signing_keys::table)
            .values((
                identity_signing_keys::usage.eq("long_term"),
                identity_signing_keys::key_id.eq(Some(key_material.key_id.as_str())),
                identity_signing_keys::public_key.eq(key_material.public_key.as_str()),
                identity_signing_keys::private_key.eq(Some(key_material.private_key.as_str())),
                identity_signing_keys::expires_at.eq::<Option<chrono::DateTime<Utc>>>(None),
            ))
            .on_conflict(identity_signing_keys::key_id)
            .do_update()
            .set((
                identity_signing_keys::usage.eq(excluded(identity_signing_keys::usage)),
                identity_signing_keys::public_key.eq(excluded(identity_signing_keys::public_key)),
                identity_signing_keys::private_key.eq(excluded(identity_signing_keys::private_key)),
                identity_signing_keys::expires_at.eq::<Option<chrono::DateTime<Utc>>>(None),
                identity_signing_keys::updated_at.eq(Utc::now()),
            ))
            .execute(&mut connection)
            .map_err(database_error_to_identity_error)?;

        Ok(())
    }

    fn generate_long_term_key_material(
        &self,
    ) -> Result<ConfiguredLongTermKeyMaterial, IdentityServiceError> {
        let key_id = self.default_long_term_key_id.clone();
        IdentitySigningKeyId::parse(key_id.clone())?;

        let public_key = random_base64_url_string();
        let private_key = random_base64_url_string();

        Ok(ConfiguredLongTermKeyMaterial {
            key_id,
            public_key,
            private_key,
        })
    }
}

impl IdentitySigningKeyRepository for IdentityPersistence {
    fn find_long_term_key(
        &self,
        key_id: &IdentitySigningKeyId,
    ) -> Result<Option<IdentitySigningKey>, IdentityServiceError> {
        use schema::identity_signing_keys;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;
        let now = Utc::now();

        let row = identity_signing_keys::table
            .select((
                identity_signing_keys::key_id,
                identity_signing_keys::public_key,
            ))
            .filter(identity_signing_keys::usage.eq("long_term"))
            .filter(identity_signing_keys::key_id.eq(Some(key_id.as_str())))
            .filter(
                identity_signing_keys::expires_at
                    .is_null()
                    .or(identity_signing_keys::expires_at.gt(now)),
            )
            .first::<(Option<String>, String)>(&mut connection)
            .optional()
            .map_err(database_error_to_identity_error)?;

        row.map(long_term_key_from_columns).transpose()
    }

    fn is_long_term_public_key_valid(
        &self,
        public_key: &EncodedPublicKey,
    ) -> Result<bool, IdentityServiceError> {
        use schema::identity_signing_keys;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;
        let now = Utc::now();

        select(exists(
            identity_signing_keys::table
                .filter(identity_signing_keys::usage.eq("long_term"))
                .filter(identity_signing_keys::public_key.eq(public_key.as_str()))
                .filter(
                    identity_signing_keys::expires_at
                        .is_null()
                        .or(identity_signing_keys::expires_at.gt(now)),
                ),
        ))
        .get_result::<bool>(&mut connection)
        .map_err(database_error_to_identity_error)
    }

    fn is_ephemeral_public_key_valid(
        &self,
        public_key: &EncodedPublicKey,
    ) -> Result<bool, IdentityServiceError> {
        use schema::identity_signing_keys;

        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;
        let now = Utc::now();

        select(exists(
            identity_signing_keys::table
                .filter(identity_signing_keys::usage.eq("ephemeral_invite"))
                .filter(identity_signing_keys::public_key.eq(public_key.as_str()))
                .filter(
                    identity_signing_keys::expires_at
                        .is_null()
                        .or(identity_signing_keys::expires_at.gt(now)),
                ),
        ))
        .get_result::<bool>(&mut connection)
        .map_err(database_error_to_identity_error)
    }
}

impl IdentityInvitationRepository for IdentityPersistence {
    fn store_invite(
        &self,
        command: &StoreThirdPartyInviteCommand,
        public_keys: Vec<IdentitySigningKey>,
    ) -> Result<StoredThirdPartyInvite, IdentityServiceError> {
        use schema::{identity_signing_keys, identity_third_party_invites};

        let long_term_key = self.fetch_active_long_term_key()?;
        let token = Uuid::new_v4().to_string();
        let display_name = redacted_display_name(command.address.as_str());
        let expires_at = Utc::now() + Duration::seconds(self.ephemeral_key_validity_seconds);

        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;

        connection
            .transaction(|connection| {
                insert_into(identity_third_party_invites::table)
                    .values((
                        identity_third_party_invites::token.eq(token.as_str()),
                        identity_third_party_invites::medium.eq(command.medium.as_str()),
                        identity_third_party_invites::address.eq(command.address.as_str()),
                        identity_third_party_invites::room_id.eq(command.room_id.as_str()),
                        identity_third_party_invites::sender.eq(command.sender.as_str()),
                        identity_third_party_invites::display_name.eq(display_name.as_str()),
                        identity_third_party_invites::expires_at.eq(Some(expires_at)),
                    ))
                    .execute(connection)?;

                for invite_key in &public_keys {
                    insert_into(identity_signing_keys::table)
                        .values((
                            identity_signing_keys::usage.eq("ephemeral_invite"),
                            identity_signing_keys::key_id.eq(Some(invite_key.key_id.as_str())),
                            identity_signing_keys::public_key.eq(invite_key.public_key.as_str()),
                            identity_signing_keys::private_key.eq::<Option<&str>>(None),
                            identity_signing_keys::expires_at.eq(Some(expires_at)),
                        ))
                        .execute(connection)?;
                }

                Ok::<(), diesel::result::Error>(())
            })
            .map_err(database_error_to_identity_error)?;

        let mut response_public_keys = Vec::with_capacity(public_keys.len() + 1);
        response_public_keys.push(long_term_key);
        response_public_keys.extend(public_keys);

        Ok(StoredThirdPartyInvite {
            medium: command.medium.clone(),
            address: command.address.clone(),
            room_id: command.room_id.clone(),
            sender: command.sender.clone(),
            display_name,
            token: crate::services::identity::entities::InvitationToken::parse(token)?,
            public_keys: response_public_keys,
        })
    }
}

impl IdentityEphemeralKeyGenerator for IdentityPersistence {
    fn generate_invite_key(&self) -> Result<IdentitySigningKey, IdentityServiceError> {
        let algorithm = IdentitySigningAlgorithm::ed25519();
        let key_identifier = format!("ephemeral_{}", Uuid::new_v4().simple());
        let key_id =
            IdentitySigningKeyId::parse(format!("{}:{}", algorithm.as_str(), key_identifier))?;
        let public_key = EncodedPublicKey::parse(random_base64_url_string())?;

        Ok(IdentitySigningKey {
            key_id,
            public_key,
            usage: IdentitySigningKeyUsage::EphemeralInvite,
        })
    }
}

fn configured_long_term_key_material(
    identity_configuration: &IdentityConfiguration,
) -> Option<ConfiguredLongTermKeyMaterial> {
    let public_key = identity_configuration.long_term_public_key.as_ref()?.trim();
    let private_key = identity_configuration
        .long_term_private_key
        .as_ref()?
        .trim();
    if public_key.is_empty() || private_key.is_empty() {
        return None;
    }

    Some(ConfiguredLongTermKeyMaterial {
        key_id: identity_configuration.long_term_key_id.clone(),
        public_key: public_key.to_owned(),
        private_key: private_key.to_owned(),
    })
}

fn redacted_display_name(address: &str) -> String {
    let (local_part, domain) = address.split_once('@').unwrap_or((address, ""));

    let redacted_local_part = if local_part.len() <= 1 {
        "...".to_owned()
    } else {
        format!("{}...", &local_part[..1])
    };

    let redacted_domain = if domain.is_empty() {
        "".to_owned()
    } else if domain.len() <= 1 {
        "...".to_owned()
    } else {
        format!("{}...", &domain[..1])
    };

    if redacted_domain.is_empty() {
        redacted_local_part
    } else {
        format!("{redacted_local_part}@{redacted_domain}")
    }
}

fn long_term_key_from_columns(
    (key_id, public_key): (Option<String>, String),
) -> Result<IdentitySigningKey, IdentityServiceError> {
    let key_id = key_id.ok_or_else(|| {
        IdentityServiceError::Internal(anyhow::anyhow!(
            "identity long-term signing key is missing key_id"
        ))
    })?;

    Ok(IdentitySigningKey {
        key_id: IdentitySigningKeyId::parse(key_id)?,
        public_key: EncodedPublicKey::parse(public_key)?,
        usage: IdentitySigningKeyUsage::LongTerm,
    })
}

fn connection_error_to_identity_error<E>(error: E) -> IdentityServiceError
where
    E: ToString,
{
    IdentityServiceError::Internal(anyhow::anyhow!(error.to_string()))
}

fn database_error_to_identity_error(error: diesel::result::Error) -> IdentityServiceError {
    IdentityServiceError::Internal(anyhow::anyhow!(error.to_string()))
}

fn random_base64_url_string() -> String {
    let left = Uuid::new_v4();
    let right = Uuid::new_v4();

    let mut bytes = [0_u8; 32];
    bytes[..16].copy_from_slice(left.as_bytes());
    bytes[16..].copy_from_slice(right.as_bytes());

    URL_SAFE_NO_PAD.encode(bytes)
}
