use async_trait::async_trait;
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{path::Path, time::Duration};
use tokio::time::sleep;

use crate::{
    application::ports::{RoomRepository, ServerProfileRepository, SessionRepository},
    domain::{
        authorization::{ServerCredentials, SessionRecord},
        error::ClientError,
        rooms::RoomListItem,
    },
};

pub struct SqliteSessionRepository {
    pool: SqlitePool,
}

impl SqliteSessionRepository {
    pub async fn new(database_path: &Path) -> Result<Self, ClientError> {
        let parent_directory = database_path
            .parent()
            .ok_or_else(|| ClientError::Database("invalid database path".to_owned()))?;
        std::fs::create_dir_all(parent_directory)
            .map_err(|error| ClientError::Database(error.to_string()))?;

        let connect_options = SqliteConnectOptions::new()
            .filename(database_path)
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(30));

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(connect_options)
            .await
            .map_err(|error| ClientError::Database(error.to_string()))?;

        initialize_schema_with_retry(&pool).await?;

        Ok(Self { pool })
    }
}

async fn initialize_schema_with_retry(pool: &SqlitePool) -> Result<(), ClientError> {
    let create_session_table = "CREATE TABLE IF NOT EXISTS client_session (
        server_url TEXT PRIMARY KEY,
        access_token TEXT NOT NULL,
        user_id TEXT NOT NULL,
        device_id TEXT,
        refresh_token TEXT,
        expires_in_milliseconds INTEGER
    )";
    let create_server_profile_table = "CREATE TABLE IF NOT EXISTS client_server_account (
        server_url TEXT NOT NULL,
        username TEXT NOT NULL,
        password TEXT NOT NULL,
        PRIMARY KEY (server_url, username)
    )";
    let create_room_table = "CREATE TABLE IF NOT EXISTS client_room (
        server_url TEXT NOT NULL,
        user_id TEXT NOT NULL,
        room_id TEXT NOT NULL,
        name TEXT,
        topic TEXT,
        PRIMARY KEY (server_url, user_id, room_id)
    )";

    let retry_delays = [100_u64, 300, 700, 1500];
    let mut last_error: Option<String> = None;

    for delay_ms in retry_delays {
        let session_result = sqlx::query(create_session_table).execute(pool).await;
        let profile_result = sqlx::query(create_server_profile_table).execute(pool).await;
        let room_result = sqlx::query(create_room_table).execute(pool).await;
        match (session_result, profile_result, room_result) {
            (Ok(_), Ok(_), Ok(_)) => return Ok(()),
            (session_error, profile_error, room_error) => {
                last_error = Some(format!(
                    "session_table={:?}, server_profile_table={:?}, room_table={:?}",
                    session_error.err(),
                    profile_error.err(),
                    room_error.err()
                ));
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    Err(ClientError::Database(
        last_error.unwrap_or_else(|| "unknown schema initialization failure".to_owned()),
    ))
}

#[async_trait]
impl RoomRepository for SqliteSessionRepository {
    async fn add_room(&self, room: &RoomListItem) -> Result<(), ClientError> {
        sqlx::query(
            "INSERT INTO client_room (server_url, user_id, room_id, name, topic)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(server_url, user_id, room_id) DO UPDATE SET
                name = excluded.name,
                topic = excluded.topic",
        )
        .bind(&room.server_url)
        .bind(&room.user_id)
        .bind(&room.room_id)
        .bind(&room.name)
        .bind(&room.topic)
        .execute(&self.pool)
        .await
        .map_err(|error| ClientError::Database(error.to_string()))?;
        Ok(())
    }

    async fn list_rooms_by_server_user(
        &self,
        server_url: &str,
        user_id: &str,
    ) -> Result<Vec<RoomListItem>, ClientError> {
        let rows = sqlx::query(
            "SELECT server_url, user_id, room_id, name, topic
             FROM client_room
             WHERE server_url = ?1 AND user_id = ?2
             ORDER BY COALESCE(name, room_id)",
        )
        .bind(server_url)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| ClientError::Database(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| RoomListItem {
                server_url: row.get::<String, _>("server_url"),
                user_id: row.get::<String, _>("user_id"),
                room_id: row.get::<String, _>("room_id"),
                name: row.get::<Option<String>, _>("name"),
                topic: row.get::<Option<String>, _>("topic"),
            })
            .collect())
    }
}

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
    async fn upsert_session(&self, session: &SessionRecord) -> Result<(), ClientError> {
        sqlx::query(
            "INSERT INTO client_session (server_url, access_token, user_id, device_id, refresh_token, expires_in_milliseconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(server_url) DO UPDATE SET
                access_token = excluded.access_token,
                user_id = excluded.user_id,
                device_id = excluded.device_id,
                refresh_token = excluded.refresh_token,
                expires_in_milliseconds = excluded.expires_in_milliseconds",
        )
        .bind(&session.server_url)
        .bind(&session.access_token)
        .bind(&session.user_id)
        .bind(&session.device_id)
        .bind(&session.refresh_token)
        .bind(session.expires_in_milliseconds.map(|value| value as i64))
        .execute(&self.pool)
        .await
        .map_err(|error| ClientError::Database(error.to_string()))?;

        Ok(())
    }

    async fn get_session_by_server(
        &self,
        server_url: &str,
    ) -> Result<Option<SessionRecord>, ClientError> {
        let row = sqlx::query(
            "SELECT server_url, access_token, user_id, device_id, refresh_token, expires_in_milliseconds
             FROM client_session
             WHERE server_url = ?1",
        )
        .bind(server_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| ClientError::Database(error.to_string()))?;

        Ok(row.map(|row| SessionRecord {
            server_url: row.get::<String, _>("server_url"),
            access_token: row.get::<String, _>("access_token"),
            user_id: row.get::<String, _>("user_id"),
            device_id: row.get::<Option<String>, _>("device_id"),
            refresh_token: row.get::<Option<String>, _>("refresh_token"),
            expires_in_milliseconds: row
                .get::<Option<i64>, _>("expires_in_milliseconds")
                .map(|value| value as u64),
        }))
    }

    async fn clear_session_by_server(&self, server_url: &str) -> Result<(), ClientError> {
        sqlx::query("DELETE FROM client_session WHERE server_url = ?1")
            .bind(server_url)
            .execute(&self.pool)
            .await
            .map_err(|error| ClientError::Database(error.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl ServerProfileRepository for SqliteSessionRepository {
    async fn upsert_server_credentials(
        &self,
        credentials: &ServerCredentials,
    ) -> Result<(), ClientError> {
        sqlx::query(
            "INSERT INTO client_server_account (server_url, username, password)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(server_url, username) DO UPDATE SET
                password = excluded.password",
        )
        .bind(&credentials.server_url)
        .bind(&credentials.username)
        .bind(&credentials.password)
        .execute(&self.pool)
        .await
        .map_err(|error| ClientError::Database(error.to_string()))?;

        Ok(())
    }

    async fn list_server_credentials(&self) -> Result<Vec<ServerCredentials>, ClientError> {
        let rows = sqlx::query(
            "SELECT server_url, username, password
             FROM client_server_account
             ORDER BY server_url, username",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| ClientError::Database(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| ServerCredentials {
                server_url: row.get::<String, _>("server_url"),
                username: row.get::<String, _>("username"),
                password: row.get::<String, _>("password"),
            })
            .collect())
    }

    async fn get_server_credentials(
        &self,
        server_url: &str,
        username: &str,
    ) -> Result<Option<ServerCredentials>, ClientError> {
        let row = sqlx::query(
            "SELECT server_url, username, password
             FROM client_server_account
             WHERE server_url = ?1 AND username = ?2",
        )
        .bind(server_url)
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| ClientError::Database(error.to_string()))?;

        Ok(row.map(|row| ServerCredentials {
            server_url: row.get::<String, _>("server_url"),
            username: row.get::<String, _>("username"),
            password: row.get::<String, _>("password"),
        }))
    }

    async fn list_server_accounts(&self, server_url: &str) -> Result<Vec<ServerCredentials>, ClientError> {
        let rows = sqlx::query(
            "SELECT server_url, username, password
             FROM client_server_account
             WHERE server_url = ?1
             ORDER BY username",
        )
        .bind(server_url)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| ClientError::Database(error.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| ServerCredentials {
                server_url: row.get::<String, _>("server_url"),
                username: row.get::<String, _>("username"),
                password: row.get::<String, _>("password"),
            })
            .collect())
    }
}
