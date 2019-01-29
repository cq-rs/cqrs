use types::CqrsError;

/// A projected state built from a series of events.
pub trait Aggregate: Default {
    /// A static string representing the type of the aggregate.
    ///
    /// Note: This should effectively be a constant value, and should never change.
    fn entity_type() -> &'static str;

    /// The event type that can be applied to this aggregate.
    type Event: AggregateEvent<Aggregate=Self>;

    /// Consumes the event, applying its effects to the aggregate.
    fn apply(&mut self, event: Self::Event) {
        event.apply_to(self);
    }

    /// Consumes a command, attempting to execute it against the aggregate. If the execution is successful, a sequence
    /// of events is generated, which can be applied to the aggregate.
    fn execute<C: AggregateCommand<Aggregate=Self>>(&self, command: C) -> Result<C::Events, C::Error> {
        command.execute_on(self)
    }
}

/// A command that can be executed against an aggregate.
pub trait AggregateCommand {
    /// The aggregate type.
    type Aggregate: Aggregate;

    /// The type of the sequence of events generated when the command is executed successfully.
    type Events: IntoIterator<Item=<Self::Aggregate as Aggregate>::Event>;

    /// The error type.
    type Error: CqrsError;

    /// Consumes a command, attempting to execute it against the aggregate. If the execution is successful, a sequence
    /// of events is generated, which can be applied to the aggregate.
    fn execute_on(self, aggregate: &Self::Aggregate) -> Result<Self::Events, Self::Error>;
}

/// A thing that happened.
pub trait Event {
    /// A static description of the event.
    fn event_type(&self) -> &'static str;
}

/// An event that can be applied to an aggregate.
pub trait AggregateEvent: Event {
    /// The aggregate type.
    type Aggregate: Aggregate;

    /// Consumes the event, applying its effects to the aggregate.
    fn apply_to(self, aggregate: &mut Self::Aggregate);
}

/// An event that can be serialized to a buffer.
pub trait SerializableEvent: Event {
    /// The error type.
    type Error: CqrsError;

    /// Serializes the event to the given buffer.
    fn serialize_event_to_buffer(&self, buffer: &mut Vec<u8>) -> Result<(), Self::Error>;
}

/// An event that can be deserialized from a buffer.
pub trait DeserializableEvent: Event + Sized {
    /// The error type.
    type Error: CqrsError;

    /// Deserializes an event from the provided buffer, with prior knowledge about the event's type.
    fn deserialize_event_from_buffer(data: &[u8], event_type: &str) -> Result<Option<Self>, Self::Error>;
}
