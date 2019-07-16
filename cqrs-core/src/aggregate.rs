use std::marker::PhantomData;

use crate::{
    future::IntoTryFuture,
    types::{CqrsError, EventVersion},
};

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

/// A command that is addressed to an [`Aggregate`].
pub trait Command {
    type Aggregate: Aggregate;

    /// Returns ID of the [`Aggregate`] that this [`Command`] is addressed to.
    /// `None` means that a new [`Aggregate`] should be initialized for
    /// this [`Command`].
    fn aggregate_id(&self) -> Option<&<Self::Aggregate as Aggregate>::Id>;
}

/// A handler of a specific [`Command`] for its [`Aggregate`].
pub trait CommandHandler<C: Command + ?Sized> {
    /// A context required by this [`CommandHandler`] for performing operation.
    type Context: ?Sized;
    type Event: Event;
    type Err;
    type Ok: IntoEvents<Self::Event>;
    /// A result that this [`CommandHandler`] will return.
    type Result: IntoTryFuture<Self::Ok, Self::Err>;

    /// Handles and processes given [`Command`] for its [`Aggregate`].
    fn handle_command(&self, cmd: &C, ctx: &Self::Context) -> Self::Result;
}

/// A command that can be executed against an aggregate.
#[deprecated]
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

/// A different version of the same [`Event`].
pub trait VersionedEvent: Event {
    /// A static version of the [`Event`].
    fn event_version(&self) -> &'static EventVersion;
}

/// A state that can be calculated by applying specified [`Event`].
pub trait EventSourced<E: Event> {
    /// Applies given [`Event`] to the current state.
    fn apply_event(&mut self, event: &E);
}

/// An event that can be applied to an aggregate.
#[deprecated]
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

pub trait IntoEvents<E> {
    type Iter: AsRef<[E]>;

    fn into_events(self) -> Self::Iter;
}

impl<E> IntoEvents<E> for () {
    type Iter = [E; 0];

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        []
    }
}

impl<E, A> IntoEvents<E> for (A,)
where
    E: From<A>,
{
    type Iter = [E; 1];

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        [self.0.into()]
    }
}

impl<E, A, B> IntoEvents<E> for (A, B)
where
    E: From<A> + From<B>,
{
    type Iter = [E; 2];

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        [self.0.into(), self.1.into()]
    }
}

impl<E, A, B, C> IntoEvents<E> for (A, B, C)
where
    E: From<A> + From<B> + From<C>,
{
    type Iter = [E; 3];

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        [self.0.into(), self.1.into(), self.2.into()]
    }
}

impl<E, A, B, C, D> IntoEvents<E> for (A, B, C, D)
where
    E: From<A> + From<B> + From<C> + From<D>,
{
    type Iter = [E; 4];

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        [self.0.into(), self.1.into(), self.2.into(), self.3.into()]
    }
}

impl<E> IntoEvents<E> for Vec<E> {
    type Iter = Self;

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<E> IntoEvents<E> for [E; 0] {
    type Iter = Self;

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<E> IntoEvents<E> for [E; 1] {
    type Iter = Self;

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<E> IntoEvents<E> for [E; 2] {
    type Iter = Self;

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<E> IntoEvents<E> for [E; 3] {
    type Iter = Self;

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<E> IntoEvents<E> for [E; 4] {
    type Iter = Self;

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<E, T> IntoEvents<E> for EventsRef<T>
where
    T: AsRef<[E]>,
{
    type Iter = T;

    #[inline(always)]
    fn into_events(self) -> Self::Iter {
        self.0
    }
}

pub struct EventsRef<T>(pub T);

impl<T> From<T> for EventsRef<T> {
    #[inline(always)]
    fn from(v: T) -> Self {
        Self(v)
    }
}
