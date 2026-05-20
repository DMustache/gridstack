use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use diesel::{
    Connection, OptionalExtension, QueryableByName, RunQueryDsl,
    pg::PgConnection,
    r2d2::{self, ConnectionManager},
    sql_query,
    sql_types::{Bool, Nullable, Text},
};
use uuid::Uuid;

use crate::{
    infrastructure::configuration::IdentityConfiguration,
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

#[derive(QueryableByName)]
struct LongTermKeyRow {
    #[diesel(sql_type = Text)]
    key_id: String,
    #[diesel(sql_type = Text)]
    public_key: String,
}

#[derive(QueryableByName)]
struct ExistsRow {
    #[diesel(sql_type = Bool)]
    is_present: bool,
}

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
        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;

        let row = sql_query(
            "SELECT key_id, public_key
             FROM public.identity_signing_keys
             WHERE usage = 'long_term'
               AND (expires_at IS NULL OR expires_at > NOW())
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .get_result::<LongTermKeyRow>(&mut connection)
        .optional()
        .map_err(database_error_to_identity_error)?;

        row.map(long_term_key_from_row).transpose()
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
        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;

        sql_query(
            "INSERT INTO public.identity_signing_keys
             (usage, key_id, public_key, private_key, expires_at)
             VALUES ('long_term', $1, $2, $3, NULL)
             ON CONFLICT (key_id)
             DO UPDATE
                SET usage = EXCLUDED.usage,
                    public_key = EXCLUDED.public_key,
                    private_key = EXCLUDED.private_key,
                    expires_at = NULL,
                    updated_at = NOW()",
        )
        .bind::<Text, _>(key_material.key_id.as_str())
        .bind::<Text, _>(key_material.public_key.as_str())
        .bind::<Text, _>(key_material.private_key.as_str())
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
        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;

        let row = sql_query(
            "SELECT key_id, public_key
             FROM public.identity_signing_keys
             WHERE usage = 'long_term'
               AND key_id = $1
               AND (expires_at IS NULL OR expires_at > NOW())
             LIMIT 1",
        )
        .bind::<Text, _>(key_id.as_str())
        .get_result::<LongTermKeyRow>(&mut connection)
        .optional()
        .map_err(database_error_to_identity_error)?;

        row.map(long_term_key_from_row).transpose()
    }

    fn is_long_term_public_key_valid(
        &self,
        public_key: &EncodedPublicKey,
    ) -> Result<bool, IdentityServiceError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;

        let row = sql_query(
            "SELECT EXISTS (
                SELECT 1
                FROM public.identity_signing_keys
                WHERE usage = 'long_term'
                  AND public_key = $1
                  AND (expires_at IS NULL OR expires_at > NOW())
            ) AS is_present",
        )
        .bind::<Text, _>(public_key.as_str())
        .get_result::<ExistsRow>(&mut connection)
        .map_err(database_error_to_identity_error)?;

        Ok(row.is_present)
    }

    fn is_ephemeral_public_key_valid(
        &self,
        public_key: &EncodedPublicKey,
    ) -> Result<bool, IdentityServiceError> {
        let mut connection = self
            .connection_pool
            .get()
            .map_err(connection_error_to_identity_error)?;

        let row = sql_query(
            "SELECT EXISTS (
                SELECT 1
                FROM public.identity_signing_keys
                WHERE usage = 'ephemeral_invite'
                  AND public_key = $1
                  AND (expires_at IS NULL OR expires_at > NOW())
            ) AS is_present",
        )
        .bind::<Text, _>(public_key.as_str())
        .get_result::<ExistsRow>(&mut connection)
        .map_err(database_error_to_identity_error)?;

        Ok(row.is_present)
    }
}

impl IdentityInvitationRepository for IdentityPersistence {
    fn store_invite(
        &self,
        command: &StoreThirdPartyInviteCommand,
        public_keys: Vec<IdentitySigningKey>,
    ) -> Result<StoredThirdPartyInvite, IdentityServiceError> {
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
                sql_query(
                    "INSERT INTO public.identity_third_party_invites
                     (token, medium, address, room_id, sender, display_name, expires_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind::<Text, _>(token.as_str())
                .bind::<Text, _>(command.medium.as_str())
                .bind::<Text, _>(command.address.as_str())
                .bind::<Text, _>(command.room_id.as_str())
                .bind::<Text, _>(command.sender.as_str())
                .bind::<Text, _>(display_name.as_str())
                .bind::<Nullable<diesel::sql_types::Timestamptz>, _>(Some(expires_at))
                .execute(connection)?;

                for public_key in &public_keys {
                    sql_query(
                        "INSERT INTO public.identity_signing_keys
                         (usage, key_id, public_key, private_key, expires_at)
                         VALUES ('ephemeral_invite', $1, $2, NULL, $3)",
                    )
                    .bind::<Nullable<Text>, _>(Some(public_key.key_id.as_str()))
                    .bind::<Text, _>(public_key.public_key.as_str())
                    .bind::<Nullable<diesel::sql_types::Timestamptz>, _>(Some(expires_at))
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

fn long_term_key_from_row(row: LongTermKeyRow) -> Result<IdentitySigningKey, IdentityServiceError> {
    Ok(IdentitySigningKey {
        key_id: IdentitySigningKeyId::parse(row.key_id)?,
        public_key: EncodedPublicKey::parse(row.public_key)?,
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
