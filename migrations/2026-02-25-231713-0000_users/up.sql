CREATE TABLE users (
    user_id VARCHAR PRIMARY KEY,
    account_id UUID NOT NULL UNIQUE REFERENCES accounts(id),
    password_hash VARCHAR,
    is_guest BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE devices
    ADD CONSTRAINT devices_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE;
