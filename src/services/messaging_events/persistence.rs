#[derive(Clone)]
pub struct MessagingEventsPersistenceService<Connection> {
    connection: Connection,
}

pub struct StoreEventInput {
    pub room_identifier: String,
    pub event_identifier: String,
    pub event_type: String,
}

pub struct StoreEventOutput;

pub struct LoadEventsInput {
    pub room_identifier: String,
}

pub struct LoadEventsOutput {
    pub event_identifiers: Vec<String>,
}

mod room_events;
mod room_state;

impl<Connection> MessagingEventsPersistenceService<Connection> {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub fn store_event(&self, _input: StoreEventInput) -> anyhow::Result<StoreEventOutput> {
        let _ = &self.connection;
        todo!()
    }

    pub fn load_events(&self, _input: LoadEventsInput) -> anyhow::Result<LoadEventsOutput> {
        let _ = &self.connection;
        todo!()
    }
}
