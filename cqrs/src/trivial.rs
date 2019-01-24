use std::iter::Empty;
use void::Void;
use cqrs_core::{Aggregate, EventSource, EventSink, SnapshotSource, SnapshotSink, EventNumber, VersionedEvent, Since, VersionedAggregate, VersionedAggregateView, Precondition};

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct NullStore;

impl<A> EventSource<A> for NullStore
where
    A: Aggregate,
{
    type Events = Empty<Result<VersionedEvent<A::Event>, Void>>;
    type Error = Void;

    #[inline]
    fn read_events(&self, _id: &str, _version: Since, _max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error> {
        Ok(None)
    }
}

impl<A, M> EventSink<A, M> for NullStore
where
    A: Aggregate,
{
    type Error = Void;

    #[inline]
    fn append_events(&self, _id: &str, _events: &[A::Event], _expect: Option<Precondition>, _metadata: M) -> Result<EventNumber, Self::Error> {
        Ok(EventNumber::MIN_VALUE)
    }
}

impl<A> SnapshotSource<A> for NullStore
where
    A: Aggregate,
{
    type Error = Void;

    #[inline]
    fn get_snapshot(&self, _id: &str) -> Result<Option<VersionedAggregate<A>>, Self::Error>
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
    fn persist_snapshot(&self, _id: &str, _aggregate: VersionedAggregateView<A>) -> Result<(), Self::Error>
        where Self: Sized
    {
        Ok(())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
