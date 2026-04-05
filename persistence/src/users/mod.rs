pub mod models;
pub use models::{NewUser, User};

use ::models::error::internal_error::DatabaseError;
use deadpool_diesel::{
    Runtime,
    postgres::{Manager, Pool},
};
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper, dsl::exists,
    insert_into, select,
};

use crate::schema::users;

use std::sync::LazyLock;

static CONNECTION_STRING: LazyLock<String> =
    LazyLock::new(|| std::env::var("DATABASE_URL").expect("DATABASE_URL must be set at runtime"));

pub struct Users {
    pool: Pool,
}

impl Users {
    pub fn try_new() -> Result<Self, DatabaseError> {
        let manager = Manager::new(&*CONNECTION_STRING, Runtime::Tokio1);
        let pool = Pool::builder(manager).build()?;

        Ok(Self { pool })
    }

    pub async fn exists(&self, user_id: String) -> Result<bool, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let exists = select(exists(users::table.filter(users::user_id.eq(user_id))))
                    .get_result::<bool>(connection)?;

                Ok::<bool, DatabaseError>(exists)
            })
            .await??)
    }

    pub async fn create_user(&self, new_user: NewUser) -> Result<(), DatabaseError> {
        self.pool
            .get()
            .await?
            .interact(move |connection| {
                insert_into(users::table)
                    .values(new_user)
                    .execute(connection)?;

                Ok::<(), DatabaseError>(())
            })
            .await?
    }

    pub async fn get_by_user_id(&self, user_id: String) -> Result<Option<User>, DatabaseError> {
        Ok(self
            .pool
            .get()
            .await?
            .interact(move |connection| {
                let user = users::table
                    .filter(users::user_id.eq(user_id))
                    .select(User::as_select())
                    .first::<User>(connection)
                    .optional()?;

                Ok::<Option<User>, DatabaseError>(user)
            })
            .await??)
    }
}
