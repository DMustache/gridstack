// @generated automatically by Diesel CLI.

diesel::table! {
    access_tokens (access_token) {
        access_token -> Varchar,
        user_id -> Varchar,
        device_id -> Varchar,
        refresh_token -> Varchar,
        access_token_created_at -> Timestamptz,
        access_token_expires_at -> Timestamptz,
        refresh_token_created_at -> Timestamptz,
        refresh_token_expires_at -> Timestamptz,
    }
}

diesel::table! {
    accounts (id) {
        id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    devices (user_id, device_id) {
        user_id -> Varchar,
        device_id -> Varchar,
        display_name -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    e2ee_device_keys (user_id, device_id) {
        user_id -> Varchar,
        device_id -> Varchar,
        key_data -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    e2ee_fallback_keys (user_id, device_id, key_algorithm) {
        user_id -> Varchar,
        device_id -> Varchar,
        key_algorithm -> Varchar,
        key_id -> Varchar,
        key_data -> Jsonb,
        is_used -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    e2ee_one_time_keys (user_id, device_id, key_algorithm, key_id) {
        user_id -> Varchar,
        device_id -> Varchar,
        key_algorithm -> Varchar,
        key_id -> Varchar,
        key_data -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    media_uploads (media_id) {
        media_id -> Varchar,
        owner_user_id -> Varchar,
        content_type -> Varchar,
        filename -> Nullable<Varchar>,
        content -> Nullable<Bytea>,
        content_length -> Nullable<Int8>,
        unused_expires_at -> Nullable<Timestamptz>,
        uploaded_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    registration_sessions (session_id) {
        session_id -> Varchar,
        account_id -> Uuid,
        completed_stages -> Jsonb,
        expires_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    room_events (event_id) {
        event_id -> Varchar,
        room_id -> Varchar,
        sender_user_id -> Varchar,
        event_type -> Varchar,
        state_key -> Nullable<Varchar>,
        content -> Jsonb,
        unsigned -> Nullable<Jsonb>,
        origin_server_ts -> Timestamptz,
        depth -> Int8,
        stream_ordering -> Int8,
        transaction_id -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    room_memberships (room_id, user_id) {
        room_id -> Varchar,
        user_id -> Varchar,
        membership -> Varchar,
        membership_event_id -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    room_state (room_id, event_type, state_key) {
        room_id -> Varchar,
        event_type -> Varchar,
        state_key -> Varchar,
        event_id -> Varchar,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    rooms (room_id) {
        room_id -> Varchar,
        creator_user_id -> Varchar,
        room_version -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Varchar,
        account_id -> Uuid,
        password_hash -> Nullable<Varchar>,
        is_guest -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(devices -> users (user_id));
diesel::joinable!(e2ee_device_keys -> users (user_id));
diesel::joinable!(e2ee_fallback_keys -> users (user_id));
diesel::joinable!(e2ee_one_time_keys -> users (user_id));
diesel::joinable!(media_uploads -> users (owner_user_id));
diesel::joinable!(registration_sessions -> accounts (account_id));
diesel::joinable!(room_events -> rooms (room_id));
diesel::joinable!(room_events -> users (sender_user_id));
diesel::joinable!(room_memberships -> rooms (room_id));
diesel::joinable!(room_memberships -> users (user_id));
diesel::joinable!(room_state -> room_events (event_id));
diesel::joinable!(room_state -> rooms (room_id));
diesel::joinable!(rooms -> users (creator_user_id));
diesel::joinable!(users -> accounts (account_id));

diesel::allow_tables_to_appear_in_same_query!(
    access_tokens,
    accounts,
    devices,
    e2ee_device_keys,
    e2ee_fallback_keys,
    e2ee_one_time_keys,
    media_uploads,
    registration_sessions,
    room_events,
    room_memberships,
    room_state,
    rooms,
    users,
);
