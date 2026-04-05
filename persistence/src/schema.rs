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
    users (user_id) {
        user_id -> Varchar,
        account_id -> Uuid,
        password_hash -> Nullable<Varchar>,
        is_guest -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(registration_sessions -> accounts (account_id));
diesel::joinable!(devices -> users (user_id));
diesel::joinable!(room_memberships -> rooms (room_id));
diesel::joinable!(room_memberships -> users (user_id));
diesel::joinable!(room_events -> rooms (room_id));
diesel::joinable!(room_events -> users (sender_user_id));
diesel::joinable!(room_state -> room_events (event_id));
diesel::joinable!(room_state -> rooms (room_id));
diesel::joinable!(rooms -> users (creator_user_id));
diesel::joinable!(users -> accounts (account_id));

diesel::allow_tables_to_appear_in_same_query!(
    access_tokens,
    accounts,
    devices,
    room_events,
    room_memberships,
    room_state,
    rooms,
    registration_sessions,
    users,
);
