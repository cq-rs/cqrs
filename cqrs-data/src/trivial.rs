use types::Since;
use cqrs::{EventNumber, Precondition, SequencedEvent, StateSnapshot};
use std::iter::Empty;
use void::Void;
use super::*;

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct NullStore;

impl<A> EventSource<A> for NullStore
where
    A: cqrs::Aggregate,
{
    type Events = Empty<Result<SequencedEvent<A::Event>, Void>>;
    type Error = Void;

    #[inline]
    fn read_events<Id: AsRef<str> + Into<String>>(&self, _id: Id, _version: Since) -> Result<Option<Self::Events>, Self::Error> {
        Ok(None)
    }
}

impl<A> EventSink<A> for NullStore
where
    A: cqrs::Aggregate,
{
    type Error = Void;

    #[inline]
    fn append_events<Id: AsRef<str> + Into<String>>(&self, _id: Id, _events: &[A::Event], _expect: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        Ok(EventNumber::MIN_VALUE)
    }
}

impl<A> SnapshotSource<A> for NullStore
where
    A: cqrs::Aggregate,
{
    type Error = Void;

    #[inline]
    fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, _id: Id) -> Result<Option<StateSnapshot<A>>, Self::Error>
        where Self: Sized
    {
        Ok(None)
    }
}

impl<A> SnapshotSink<A> for NullStore
where
    A: cqrs::Aggregate,
{
    type Error = Void;

    #[inline]
    fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, _id: Id, _snapshot: StateSnapshot<A>) -> Result<(), Self::Error>
        where Self: Sized
    {
        Ok(())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
