CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE registration_sessions (
    session_id VARCHAR PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id),
    completed_stages JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE devices (
    user_id VARCHAR NOT NULL,
    device_id VARCHAR NOT NULL,
    display_name VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, device_id)
);

CREATE TABLE access_tokens (
    access_token VARCHAR PRIMARY KEY,
    user_id VARCHAR NOT NULL,
    device_id VARCHAR NOT NULL,
    refresh_token VARCHAR NOT NULL UNIQUE,
    access_token_created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    access_token_expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    refresh_token_created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    refresh_token_expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (user_id, device_id) REFERENCES devices(user_id, device_id) ON DELETE CASCADE
);
