use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DeviceId(String);

impl DeviceId {
    #[inline]
    pub fn get_or_generate(some_device_id: Option<String>) -> DeviceId {
        Self(some_device_id.unwrap_or_else(|| Uuid::new_v4().to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
