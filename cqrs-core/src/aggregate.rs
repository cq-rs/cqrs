use types::CqrsError;

pub trait Aggregate: Default {
    type Event: Event;
    type Events: IntoIterator<Item=Self::Event>;

    type Command;
    type Error: CqrsError;

    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error>;
    fn entity_type() -> &'static str;
}

pub trait Event {
    fn event_type(&self) -> &'static str;
}

pub trait SerializableEvent: Event {
    type Error: CqrsError;

    fn serialize_event_to_buffer(&self, buffer: &mut Vec<u8>) -> Result<(), Self::Error>;
}

pub trait DeserializableEvent: Event + Sized {
    type Error: CqrsError;

    fn deserialize_event_from_buffer(data: &[u8], event_type: &str) -> Result<Option<Self>, Self::Error>;
}