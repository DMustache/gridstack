use crate::services::traits::Id;

pub struct SessionId(String);

impl Id for SessionId {
    fn new(&self) -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}
