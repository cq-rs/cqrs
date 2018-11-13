use std::{io, iter::Empty};
use void::Void;
use cqrs_core::{Aggregate, PersistableAggregate, SerializableEvent, EventDeserializeError};

/// A test aggregate with no state
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestAggregate;

/// A test event with no data
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestEvent;

/// A test command with no data
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestCommand;

impl Aggregate for TestAggregate {
    type Event = TestEvent;
    type Events = ::std::iter::Empty<Self::Event>;

    type Command = TestCommand;
    type Error = Void;

    fn apply(&mut self, _event: Self::Event) {}

    fn execute(&self, _command: Self::Command) -> Result<Self::Events, Self::Error> {
        Ok(Empty::default())
    }

    fn entity_type() -> &'static str {
        "test"
    }
}

impl PersistableAggregate for TestAggregate {
    type SnapshotError = &'static str;

    fn snapshot_to_writer<W: io::Write>(&self, _writer: W) -> io::Result<()> { Ok(()) }

    fn restore_from_reader<R: io::Read>(_reader: R) -> Result<Self, Self::SnapshotError> {
        Ok(TestAggregate)
    }
}

const EVENT_TYPE: &str = "test";

impl SerializableEvent for TestEvent {
    type PayloadError = &'static str;

    fn event_type(&self) -> &'static str {
        EVENT_TYPE
    }

    fn serialize_to_writer<W: io::Write>(&self, _writer: W) -> io::Result<()> {Ok(())}

    fn deserialize_from_reader<R: io::Read>(event_type: &str, _reader: R) -> Result<Self, EventDeserializeError<Self>> {
        if event_type != EVENT_TYPE {
            Err(EventDeserializeError::new_unknown_event_type(event_type))
        } else {
            Ok(TestEvent)
        }
    }
}