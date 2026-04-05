use std::env::var;

use diesel::{Connection, PgConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use models::error::internal_error::DatabaseError;

pub mod accounts;
pub mod rooms;
pub mod users;

const MIGRATION_CONNECTION_STRING: &str = "DATABASE_URL";
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

pub fn migrations() -> Result<(), DatabaseError> {
    PgConnection::establish(&var(MIGRATION_CONNECTION_STRING)?)?
        .run_pending_migrations(MIGRATIONS)
        .map_err(|error| DatabaseError::MigrationError(error))?;

    Ok(())
}

mod schema;
