CREATE TABLE e2ee_device_keys (
    user_id VARCHAR NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    device_id VARCHAR NOT NULL,
    key_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, device_id),
    FOREIGN KEY (user_id, device_id) REFERENCES devices(user_id, device_id) ON DELETE CASCADE
);

CREATE TABLE e2ee_one_time_keys (
    user_id VARCHAR NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    device_id VARCHAR NOT NULL,
    key_algorithm VARCHAR NOT NULL,
    key_id VARCHAR NOT NULL,
    key_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, device_id, key_algorithm, key_id),
    FOREIGN KEY (user_id, device_id) REFERENCES devices(user_id, device_id) ON DELETE CASCADE
);

CREATE TABLE e2ee_fallback_keys (
    user_id VARCHAR NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    device_id VARCHAR NOT NULL,
    key_algorithm VARCHAR NOT NULL,
    key_id VARCHAR NOT NULL,
    key_data JSONB NOT NULL,
    is_used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, device_id, key_algorithm),
    FOREIGN KEY (user_id, device_id) REFERENCES devices(user_id, device_id) ON DELETE CASCADE
);

CREATE INDEX idx_e2ee_one_time_keys_device_algorithm
    ON e2ee_one_time_keys(user_id, device_id, key_algorithm);

CREATE INDEX idx_e2ee_fallback_keys_device_unused
    ON e2ee_fallback_keys(user_id, device_id, is_used);
