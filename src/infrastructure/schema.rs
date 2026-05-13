// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (id) {
        id -> Uuid,
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

diesel::joinable!(users -> accounts (account_id));

diesel::allow_tables_to_appear_in_same_query!(accounts, users,);
