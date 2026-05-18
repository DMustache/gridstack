CREATE TABLE public.room_events (
    event_id VARCHAR PRIMARY KEY,
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    room_version VARCHAR NOT NULL,
    sender_user_id VARCHAR NOT NULL,
    event_type VARCHAR NOT NULL,
    state_key VARCHAR NULL,
    depth BIGINT NOT NULL,
    origin_server_ts BIGINT NOT NULL,
    redacts VARCHAR NULL,
    rejected BOOLEAN NOT NULL DEFAULT FALSE,
    soft_failed BOOLEAN NOT NULL DEFAULT FALSE,

    -- normalized payload fields for known events
    membership VARCHAR NULL,
    join_rule VARCHAR NULL,
    history_visibility VARCHAR NULL,
    guest_access VARCHAR NULL,
    canonical_alias VARCHAR NULL,
    room_name VARCHAR NULL,
    room_topic TEXT NULL,
    is_direct BOOLEAN NULL,

    -- parallel archival payload for forward compatibility
    content_json JSONB NOT NULL,
    unsigned_json JSONB NULL,
    hashes_json JSONB NOT NULL,
    signatures_json JSONB NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX room_events_room_id_created_at_idx ON public.room_events(room_id, created_at);
CREATE INDEX room_events_room_id_event_type_idx ON public.room_events(room_id, event_type);
CREATE INDEX room_events_room_id_state_key_idx ON public.room_events(room_id, state_key);

CREATE TABLE public.room_event_prev_edges (
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    prev_event_id VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (room_id, event_id, prev_event_id)
);

CREATE TABLE public.room_event_auth_edges (
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    auth_event_id VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (room_id, event_id, auth_event_id)
);

CREATE TABLE public.room_forward_extremities (
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (room_id, event_id)
);

CREATE TABLE public.room_current_state (
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    event_type VARCHAR NOT NULL,
    state_key VARCHAR NOT NULL,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (room_id, event_type, state_key)
);

CREATE TABLE public.room_membership_projection (
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    user_id VARCHAR NOT NULL,
    membership VARCHAR NOT NULL,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (room_id, user_id)
);

CREATE TABLE public.room_timeline_projection (
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    stream_position BIGINT NOT NULL,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    PRIMARY KEY (room_id, stream_position),
    UNIQUE(event_id)
);

CREATE TABLE public.room_summary_projection (
    room_id VARCHAR PRIMARY KEY REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    name_event_id VARCHAR NULL REFERENCES public.room_events(event_id) ON DELETE SET NULL,
    topic_event_id VARCHAR NULL REFERENCES public.room_events(event_id) ON DELETE SET NULL,
    canonical_alias_event_id VARCHAR NULL REFERENCES public.room_events(event_id) ON DELETE SET NULL,
    join_rule_event_id VARCHAR NULL REFERENCES public.room_events(event_id) ON DELETE SET NULL,
    history_visibility_event_id VARCHAR NULL REFERENCES public.room_events(event_id) ON DELETE SET NULL,
    guest_access_event_id VARCHAR NULL REFERENCES public.room_events(event_id) ON DELETE SET NULL,
    power_levels_event_id VARCHAR NULL REFERENCES public.room_events(event_id) ON DELETE SET NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE public.room_sync_stream (
    stream_position BIGSERIAL PRIMARY KEY,
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(event_id)
);

CREATE TABLE public.room_outbox_tasks (
    id UUID PRIMARY KEY DEFAULT public.uuid_generate_v4(),
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    task_type VARCHAR NOT NULL,
    status VARCHAR NOT NULL DEFAULT 'pending',
    payload_json JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE public.room_idempotency_records (
    room_id VARCHAR NOT NULL REFERENCES public.rooms(room_id) ON DELETE CASCADE,
    sender_user_id VARCHAR NOT NULL,
    transaction_id VARCHAR NOT NULL,
    event_id VARCHAR NOT NULL REFERENCES public.room_events(event_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY(room_id, sender_user_id, transaction_id)
);
