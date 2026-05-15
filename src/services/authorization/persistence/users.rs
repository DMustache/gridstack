use chrono::NaiveDateTime;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use uuid::Uuid;

use crate::{
    infrastructure::{schema::users, user_identifier::UserIdentifier},
    services::authorization::entities::UserAccount,
};

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, AsChangeset)]
#[diesel(table_name = users)]
#[diesel(primary_key(user_id))]
pub(super) struct UserModel {
    pub user_id: String,
    pub account_id: Uuid,
    pub password_hash: Option<String>,
    pub is_guest: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = users)]
pub(super) struct CreateUserModel {
    pub user_id: String,
    pub account_id: Uuid,
    pub password_hash: Option<String>,
    pub is_guest: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl TryFrom<UserModel> for UserAccount {
    type Error = ();

    fn try_from(value: UserModel) -> Result<Self, Self::Error> {
        Ok(Self {
            user_identifier: UserIdentifier::try_from(value.user_id).map_err(|_| ())?,
            password_hash: value.password_hash.unwrap_or_default(),
            display_name: String::new(),
            is_guest: value.is_guest,
        })
    }
}
