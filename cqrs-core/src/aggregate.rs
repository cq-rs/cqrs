use crate::types::CqrsError;

/// A projected state built from a series of events.
pub trait Aggregate: Default {
    /// Type of [`Aggregate`]'s unique identifier (ID).
    type Id;

    /// A static string representing the type of the aggregate.
    ///
    /// Note: This should effectively be a constant value, and should never change.
    fn aggregate_type() -> &'static str;

    /// Returns unique ID of this [`Aggregate`].
    fn id(&self) -> &Self::Id;
}

/// A command that can be executed against an aggregate.
pub trait AggregateCommand<A: Aggregate> {
    /// The type of event that is produced by this command.
    type Event: AggregateEvent<A>;

    /// The type of the sequence of events generated when the command is executed successfully.
    type Events: Events<ProducedEvent<A, Self>>;

    /// The error type.
    type Error: CqrsError;

    /// Consumes a command, attempting to execute it against the aggregate. If the execution is successful, a sequence
    /// of events is generated, which can be applied to the aggregate.
    fn execute_on(self, aggregate: &A) -> Result<Self::Events, Self::Error>;
}

/// The event type produced by this command.
pub type ProducedEvent<A, C> = <C as AggregateCommand<A>>::Event;

/// The event sequence produced by this command.
pub type ProducedEvents<A, C> = <C as AggregateCommand<A>>::Events;

/// The error produced when this command cannot be executed against an aggregate.
pub type CommandError<A, C> = <C as AggregateCommand<A>>::Error;

/// A thing that happened.
pub trait Event {
    /// A static description of the event.
    fn event_type(&self) -> &'static str;
}

/// An event that can be applied to an aggregate.
pub trait AggregateEvent<A: Aggregate>: Event {
    /// Consumes the event, applying its effects to the aggregate.
    fn apply_to(self, aggregate: &mut A);
}

/// An iterable and sliceable list of events.
pub trait Events<E>: IntoIterator<Item = E> + AsRef<[E]>
where
    E: Event,
{
}

impl<T, E> Events<E> for T
where
    T: IntoIterator<Item = E> + AsRef<[E]>,
    E: Event,
{
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
    fn deserialize_event_from_buffer(
        data: &[u8],
        event_type: &str,
    ) -> Result<Option<Self>, Self::Error>;
}
