use std::marker::PhantomData;

use crate::{
    future::IntoTryFuture,
    types::{CqrsError, EventVersion},
    Version,
};

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
