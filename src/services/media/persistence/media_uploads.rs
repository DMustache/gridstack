use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::infrastructure::schema::media_uploads;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = media_uploads)]
#[diesel(primary_key(media_id))]
pub struct MediaUploadDto {
    pub media_id: String,
    pub owner_user_id: String,
    pub content_type: String,
    pub filename: Option<String>,
    pub content: Option<Vec<u8>>,
    pub content_length: Option<i64>,
    pub unused_expires_at: Option<DateTime<Utc>>,
    pub uploaded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = media_uploads)]
pub struct NewMediaUploadDto {
    pub media_id: String,
    pub owner_user_id: String,
    pub content_type: String,
    pub filename: Option<String>,
    pub content: Option<Vec<u8>>,
    pub content_length: Option<i64>,
    pub unused_expires_at: Option<DateTime<Utc>>,
    pub uploaded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
