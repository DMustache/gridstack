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
    room_current_state (room_id, event_type, state_key) {
        room_id -> Varchar,
        event_type -> Varchar,
        state_key -> Varchar,
        event_id -> Varchar,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    room_event_auth_edges (room_id, event_id, auth_event_id) {
        room_id -> Varchar,
        event_id -> Varchar,
        auth_event_id -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    room_event_prev_edges (room_id, event_id, prev_event_id) {
        room_id -> Varchar,
        event_id -> Varchar,
        prev_event_id -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    room_events (event_id) {
        event_id -> Varchar,
        room_id -> Varchar,
        room_version -> Varchar,
        sender_user_id -> Varchar,
        event_type -> Varchar,
        state_key -> Nullable<Varchar>,
        depth -> Int8,
        origin_server_ts -> Int8,
        redacts -> Nullable<Varchar>,
        rejected -> Bool,
        soft_failed -> Bool,
        membership -> Nullable<Varchar>,
        join_rule -> Nullable<Varchar>,
        history_visibility -> Nullable<Varchar>,
        guest_access -> Nullable<Varchar>,
        canonical_alias -> Nullable<Varchar>,
        room_name -> Nullable<Varchar>,
        room_topic -> Nullable<Text>,
        is_direct -> Nullable<Bool>,
        content_json -> Jsonb,
        unsigned_json -> Nullable<Jsonb>,
        hashes_json -> Jsonb,
        signatures_json -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    room_forward_extremities (room_id, event_id) {
        room_id -> Varchar,
        event_id -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    room_idempotency_records (room_id, sender_user_id, transaction_id) {
        room_id -> Varchar,
        sender_user_id -> Varchar,
        transaction_id -> Varchar,
        event_id -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    room_membership_projection (room_id, user_id) {
        room_id -> Varchar,
        user_id -> Varchar,
        membership -> Varchar,
        event_id -> Varchar,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    room_outbox_tasks (id) {
        id -> Uuid,
        room_id -> Varchar,
        event_id -> Varchar,
        task_type -> Varchar,
        status -> Varchar,
        payload_json -> Nullable<Jsonb>,
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
    room_summary_projection (room_id) {
        room_id -> Varchar,
        name_event_id -> Nullable<Varchar>,
        topic_event_id -> Nullable<Varchar>,
        canonical_alias_event_id -> Nullable<Varchar>,
        join_rule_event_id -> Nullable<Varchar>,
        history_visibility_event_id -> Nullable<Varchar>,
        guest_access_event_id -> Nullable<Varchar>,
        power_levels_event_id -> Nullable<Varchar>,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    room_sync_stream (stream_position) {
        stream_position -> Int8,
        room_id -> Varchar,
        event_id -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    room_timeline_projection (room_id, stream_position) {
        room_id -> Varchar,
        stream_position -> Int8,
        event_id -> Varchar,
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
        visibility -> Nullable<Varchar>,
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
diesel::joinable!(room_current_state -> room_events (event_id));
diesel::joinable!(room_current_state -> rooms (room_id));
diesel::joinable!(room_event_auth_edges -> room_events (event_id));
diesel::joinable!(room_event_auth_edges -> rooms (room_id));
diesel::joinable!(room_event_prev_edges -> room_events (event_id));
diesel::joinable!(room_event_prev_edges -> rooms (room_id));
diesel::joinable!(room_events -> rooms (room_id));
diesel::joinable!(room_forward_extremities -> room_events (event_id));
diesel::joinable!(room_forward_extremities -> rooms (room_id));
diesel::joinable!(room_idempotency_records -> room_events (event_id));
diesel::joinable!(room_idempotency_records -> rooms (room_id));
diesel::joinable!(room_membership_projection -> room_events (event_id));
diesel::joinable!(room_membership_projection -> rooms (room_id));
diesel::joinable!(room_outbox_tasks -> room_events (event_id));
diesel::joinable!(room_outbox_tasks -> rooms (room_id));
diesel::joinable!(room_state_events -> rooms (room_id));
diesel::joinable!(room_summary_projection -> rooms (room_id));
diesel::joinable!(room_sync_stream -> room_events (event_id));
diesel::joinable!(room_sync_stream -> rooms (room_id));
diesel::joinable!(room_timeline_projection -> room_events (event_id));
diesel::joinable!(room_timeline_projection -> rooms (room_id));
diesel::joinable!(rooms -> users (creator_user_id));
diesel::joinable!(users -> accounts (account_id));

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    room_aliases,
    room_current_state,
    room_event_auth_edges,
    room_event_prev_edges,
    room_events,
    room_forward_extremities,
    room_idempotency_records,
    room_membership_projection,
    room_outbox_tasks,
    room_state_events,
    room_summary_projection,
    room_sync_stream,
    room_timeline_projection,
    rooms,
    users,
);
