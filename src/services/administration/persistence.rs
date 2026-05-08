#[derive(Clone)]
pub struct PersistenceService<Connection> {
    connection: Connection,
}

pub struct OperationInput;
pub struct OperationOutput;

mod access_tokens;

impl<Connection> PersistenceService<Connection> {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub fn execute(&self, _input: OperationInput) -> anyhow::Result<OperationOutput> {
        let _ = &self.connection;
        todo!()
    }
}
