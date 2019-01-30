use types::CqrsError;

/// A projected state built from a series of events.
pub trait Aggregate: Default {
    /// A static string representing the type of the aggregate.
    ///
    /// Note: This should effectively be a constant value, and should never change.
    fn entity_type() -> &'static str;

    /// The event type that can be applied to this aggregate.
    type Event: AggregateEvent<Aggregate = Self>;

    /// Consumes the event, applying its effects to the aggregate.
    fn apply(&mut self, event: Self::Event) {
        event.apply_to(self);
    }

    /// Consumes a command, attempting to execute it against the aggregate. If the execution is successful, a sequence
    /// of events is generated, which can be applied to the aggregate.
    fn execute<C: AggregateCommand<Aggregate = Self>>(
        &self,
        command: C,
    ) -> Result<C::Events, C::Error>
    where
        C: AggregateCommand<Aggregate = Self>,
        C::Events: Events<Self::Event>,
    {
        command.execute_on(self)
    }
}

/// The event type that is applied to a particular aggregate.
pub type EventFor<A> = <A as Aggregate>::Event;

/// An identifier for an aggregate.
pub trait AggregateId: AsRef<str> {
    /// The aggregate type identified.
    type Aggregate: Aggregate;
}

/// The aggregate type identified by a given identifier type.
pub type AggregateIdentifiedBy<I> = <I as AggregateId>::Aggregate;

/// A command that can be executed against an aggregate.
pub trait AggregateCommand {
    /// The aggregate type.
    type Aggregate: Aggregate;

    /// The type of the sequence of events generated when the command is executed successfully.
    type Events: Events<ProducedEvent<Self>>;

    /// The error type.
    type Error: CqrsError;

    /// Consumes a command, attempting to execute it against the aggregate. If the execution is successful, a sequence
    /// of events is generated, which can be applied to the aggregate.
    fn execute_on(self, aggregate: &Self::Aggregate) -> Result<Self::Events, Self::Error>;
}

/// The aggregate type that this command executes against.
pub type ExecuteTarget<C> = <C as AggregateCommand>::Aggregate;

/// The event type produced by this command.
pub type ProducedEvent<C> = <ExecuteTarget<C> as Aggregate>::Event;

/// The event sequence produced by this command.
pub type ProducedEvents<C> = <C as AggregateCommand>::Events;

/// The error produced when this command cannot be executed against an aggregate.
pub type CommandError<C> = <C as AggregateCommand>::Error;

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

/// The aggregate type that this event is applied against.
pub type ApplyTarget<E> = <E as AggregateEvent>::Aggregate;

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
