use std::iter::Empty;
use void::Void;
use super::*;

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct NullStore;

impl<A> EventSource<A> for NullStore
where
    A: Aggregate,
{
    type Events = Empty<Result<SequencedEvent<A::Event>, Void>>;
    type Error = Void;

    #[inline]
    fn read_events(&self, _id: &str, _version: Since) -> Result<Option<Self::Events>, Self::Error> {
        Ok(None)
    }
}

impl<A> EventSink<A> for NullStore
where
    A: Aggregate,
{
    type Error = Void;

    #[inline]
    fn append_events(&self, _id: &str, _events: &[A::Event], _expect: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        Ok(EventNumber::MIN_VALUE)
    }
}

impl<A> SnapshotSource<A> for NullStore
where
    A: Aggregate,
{
    type Error = Void;

    #[inline]
    fn get_snapshot(&self, _id: &str) -> Result<Option<StateSnapshot<A>>, Self::Error>
        where Self: Sized
    {
        Ok(None)
    }
}

impl<A> SnapshotSink<A> for NullStore
where
    A: Aggregate,
{
    type Error = Void;

    #[inline]
    fn persist_snapshot(&self, _id: &str, _snapshot: StateSnapshotView<A>) -> Result<(), Self::Error>
        where Self: Sized
    {
        Ok(())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
