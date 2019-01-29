//! Provides a trivial implementation of event/snapshot/entity source/sink/store constructs.

use std::iter::Empty;
use void::Void;
use cqrs_core::{Aggregate, AggregateId, EventSource, EventSink, SnapshotSource, SnapshotSink, EventNumber, VersionedEvent, Since, VersionedAggregate, VersionedAggregateView, Precondition};

/// A trivial store that never has any events or snapshots, and which always succeeds in
/// persisting data (which is immediately dropped).
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct NullStore;

impl<A> EventSource<A> for NullStore
where
    A: Aggregate,
{
    type Events = Empty<Result<VersionedEvent<A::Event>, Void>>;
    type Error = Void;

    #[inline]
    fn read_events<I>(&self, _id: &I, _version: Since, _max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        Ok(None)
    }
}

impl<A, M> EventSink<A, M> for NullStore
where
    A: Aggregate,
{
    type Error = Void;

    #[inline]
    fn append_events<I>(&self, _id: &I, _events: &[A::Event], _expect: Option<Precondition>, _metadata: M) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        Ok(EventNumber::MIN_VALUE)
    }
}

impl<A> SnapshotSource<A> for NullStore
where
    A: Aggregate,
{
    type Error = Void;

    #[inline]
    fn get_snapshot<I>(&self, _id: &I) -> Result<Option<VersionedAggregate<A>>, Self::Error>
    where
        I: AggregateId<Aggregate=A>,
        Self: Sized,
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
    fn persist_snapshot<I>(&self, _id: &I, _aggregate: VersionedAggregateView<A>) -> Result<(), Self::Error>
    where
        I: AggregateId<Aggregate=A>,
        Self: Sized,
    {
        Ok(())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
