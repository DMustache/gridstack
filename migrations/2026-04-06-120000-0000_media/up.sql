CREATE TABLE media_uploads (
    media_id VARCHAR PRIMARY KEY,
    owner_user_id VARCHAR NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    content_type VARCHAR NOT NULL DEFAULT 'application/octet-stream',
    filename VARCHAR,
    content BYTEA,
    content_length BIGINT,
    unused_expires_at TIMESTAMPTZ,
    uploaded_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_media_uploads_owner_pending
    ON media_uploads(owner_user_id, unused_expires_at)
    WHERE uploaded_at IS NULL;
