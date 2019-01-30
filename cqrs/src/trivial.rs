//! Provides a trivial implementation of event/snapshot/entity source/sink/store constructs.

use cqrs_core::{
    Aggregate, AggregateId, EventNumber, EventSink, EventSource, Precondition, Since, SnapshotSink,
    SnapshotSource, Version, VersionedAggregate, VersionedEvent,
};
use std::iter::Empty;
use void::Void;

/// A trivial store that never has any events or snapshots, and which always succeeds in
/// persisting data (which is immediately dropped).
#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct NullStore;

impl<A> EventSource<A> for NullStore
where
    A: Aggregate,
{
    type Error = Void;
    type Events = Empty<Result<VersionedEvent<A::Event>, Void>>;

    #[inline]
    fn read_events<I>(
        &self,
        _id: &I,
        _version: Since,
        _max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
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
    fn append_events<I>(
        &self,
        _id: &I,
        _events: &[A::Event],
        _expect: Option<Precondition>,
        _metadata: M,
    ) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
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
        I: AggregateId<Aggregate = A>,
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
    fn persist_snapshot<I>(
        &self,
        _id: &I,
        _aggregate: &A,
        _version: Version,
        last_snapshot_version: Version,
    ) -> Result<Version, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
        Self: Sized,
    {
        Ok(last_snapshot_version)
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
