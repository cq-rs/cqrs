use types::{CqrsError, EventDeserializeError};

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

    fn snapshot(&self) -> Vec<u8> {
        let mut snapshot = Vec::new();
        self.snapshot_in_place(&mut snapshot);
        snapshot
    }
    fn snapshot_in_place(&self, snapshot: &mut Vec<u8>);
    fn restore(snapshot: &[u8]) -> Result<Self, Self::SnapshotError>;
}

pub trait SerializableEvent: Sized {
    type PayloadError: CqrsError;

    fn event_type(&self) -> &'static str;
    fn deserialize(event_type: &str, payload: &[u8]) -> Result<Self, EventDeserializeError<Self>>;
    fn serialize_in_place(&self, payload: &mut Vec<u8>);
    fn serialize(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        self.serialize_in_place(&mut payload);
        payload
    }
}

#[cfg(test)]
pub mod testing {
    use std::iter::Empty;
    use void::Void;
    use super::*;

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

        fn snapshot(&self) -> Vec<u8> {
            Default::default()
        }

        fn snapshot_in_place(&self, snapshot: &mut Vec<u8>) {
            debug_assert!(snapshot.is_empty());
        }

        fn restore(snapshot: &[u8]) -> Result<Self, Self::SnapshotError> {
            if !snapshot.is_empty() {
                Err("invalid snapshot")
            } else {
                Ok(TestAggregate)
            }
        }
    }

    const EVENT_TYPE: &'static str = "test";

    impl SerializableEvent for TestEvent {
        type PayloadError = &'static str;

        fn event_type(&self) -> &'static str {
            EVENT_TYPE
        }

        fn deserialize(event_type: &str, payload: &[u8]) -> Result<Self, EventDeserializeError<Self>> {
            if event_type != EVENT_TYPE {
                Err(EventDeserializeError::new_unknown_event_type(event_type))
            } else if !payload.is_empty() {
                Err(EventDeserializeError::InvalidPayload("invalid payload"))
            } else {
                Ok(TestEvent)
            }
        }

        fn serialize_in_place(&self, payload: &mut Vec<u8>) {
            debug_assert!(payload.is_empty());
        }
    }
}
