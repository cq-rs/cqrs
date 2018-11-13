use types::{EventDeserializeError, CqrsError};
use std::io;

pub trait Aggregate: Default {
    type Event;
    type Events: IntoIterator<Item=Self::Event>;

    type Command;
    type Error: CqrsError;

    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error>;
    fn entity_type() -> &'static str;
}

pub trait PersistableAggregate: Aggregate {
    type SnapshotError: CqrsError;

    fn snapshot(&self) -> io::Result<Vec<u8>> {
        let mut snapshot = Vec::new();
        self.snapshot_to_writer(&mut snapshot)?;
        Ok(snapshot)
    }
    fn snapshot_to_writer<W: io::Write>(&self, writer: W) -> io::Result<()>;

    fn restore(snapshot: &[u8]) -> Result<Self, Self::SnapshotError> {
        Self::restore_from_reader(snapshot)
    }
    fn restore_from_reader<R: io::Read>(reader: R) -> Result<Self, Self::SnapshotError>;
}

pub trait SerializableEvent: Sized {
    type PayloadError: CqrsError;

    fn event_type(&self) -> &'static str;
    fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut payload = Vec::new();
        self.serialize_to_writer(&mut payload)?;
        Ok(payload)
    }
    fn serialize_to_writer<W: io::Write>(&self, writer: W) -> io::Result<()>;

    fn deserialize(event_type: &str, payload: &[u8]) -> Result<Self, EventDeserializeError<Self>> {
        Self::deserialize_from_reader(event_type, payload)
    }
    fn deserialize_from_reader<R: io::Read>(event_type: &str, reader: R) -> Result<Self, EventDeserializeError<Self>>;
}