CREATE TABLE rooms (
    room_id VARCHAR PRIMARY KEY,
    creator_user_id VARCHAR NOT NULL REFERENCES users(user_id),
    room_version VARCHAR NOT NULL DEFAULT '11',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE room_memberships (
    room_id VARCHAR NOT NULL REFERENCES rooms(room_id) ON DELETE CASCADE,
    user_id VARCHAR NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    membership VARCHAR NOT NULL,
    membership_event_id VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (room_id, user_id)
);

CREATE TABLE room_events (
    event_id VARCHAR PRIMARY KEY,
    room_id VARCHAR NOT NULL REFERENCES rooms(room_id) ON DELETE CASCADE,
    sender_user_id VARCHAR NOT NULL REFERENCES users(user_id),
    event_type VARCHAR NOT NULL,
    state_key VARCHAR,
    content JSONB NOT NULL,
    unsigned JSONB,
    origin_server_ts TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    depth BIGINT NOT NULL,
    stream_ordering BIGSERIAL NOT NULL UNIQUE,
    transaction_id VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE room_state (
    room_id VARCHAR NOT NULL REFERENCES rooms(room_id) ON DELETE CASCADE,
    event_type VARCHAR NOT NULL,
    state_key VARCHAR NOT NULL,
    event_id VARCHAR NOT NULL REFERENCES room_events(event_id) ON DELETE CASCADE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (room_id, event_type, state_key)
);

CREATE INDEX idx_room_events_room_stream_ordering
    ON room_events(room_id, stream_ordering);

CREATE INDEX idx_room_events_room_type_state_key
    ON room_events(room_id, event_type, state_key);
