// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (id) {
        id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    room_aliases (alias_localpart) {
        alias_localpart -> Varchar,
        room_id -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    room_state_events (id) {
        id -> Uuid,
        room_id -> Varchar,
        event_type -> Varchar,
        state_key -> Varchar,
        content -> Jsonb,
        ordering -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    rooms (room_id) {
        room_id -> Varchar,
        room_version -> Nullable<Varchar>,
        creator_user_id -> Varchar,
        is_direct -> Nullable<Bool>,
        name -> Nullable<Varchar>,
        topic -> Nullable<Text>,
        visibility -> Varchar,
        preset -> Nullable<Varchar>,
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

diesel::joinable!(room_aliases -> rooms (room_id));
diesel::joinable!(room_state_events -> rooms (room_id));
diesel::joinable!(rooms -> users (creator_user_id));
diesel::joinable!(users -> accounts (account_id));

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    room_aliases,
    room_state_events,
    rooms,
    users,
);
