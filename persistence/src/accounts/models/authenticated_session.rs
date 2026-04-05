use diesel::{
    QueryableByName,
    sql_types::{Bool, Nullable, Text},
};

#[derive(Debug, QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AuthenticatedSession {
    #[diesel(sql_type = Text)]
    pub user_id: String,
    #[diesel(sql_type = Text)]
    pub device_id: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub device_display_name: Option<String>,
    #[diesel(sql_type = Text)]
    pub access_token: String,
    #[diesel(sql_type = Bool)]
    pub is_guest: bool,
}
