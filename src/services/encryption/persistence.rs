#[derive(Clone)]
pub struct PersistenceService<Connection> {
    connection: Connection,
}

pub struct OperationInput;
pub struct OperationOutput;

mod e2ee_device_keys;
mod e2ee_fallback_keys;
mod e2ee_one_time_keys;

impl<Connection> PersistenceService<Connection> {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub fn execute(&self, _input: OperationInput) -> anyhow::Result<OperationOutput> {
        let _ = &self.connection;
        todo!()
    }
}
