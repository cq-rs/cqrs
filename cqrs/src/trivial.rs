//! Provides a trivial implementation of event/snapshot/entity source/sink/store constructs.

use cqrs_core::{Aggregate, AggregateEvent, AggregateId, EventNumber, EventSink, EventSource, Precondition, Since, SnapshotSink, SnapshotSource, Version, VersionedAggregate, VersionedEvent, View};
use std::{fmt, iter::Empty, marker::PhantomData};
use void::Void;

/// A trivial store that never has any events, and which always succeeds in
/// persisting data (which is immediately dropped).
#[derive(Clone, Copy)]
pub struct NullEventStore<A, E>(PhantomData<*const (A, E)>)
where
    A: Aggregate,
    E: AggregateEvent<A>;

impl<A, E> NullEventStore<A, E>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    /// A constant value representing a trivial event store.
    pub const DEFAULT: Self = NullEventStore(PhantomData);
}

impl<A, E> Default for NullEventStore<A, E>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    #[inline(always)]
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl<A, E> fmt::Debug for NullEventStore<A, E>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("NullEventStore").finish()
    }
}

impl<A, E> EventSource<A, E> for NullEventStore<A, E>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    type Error = Void;
    type Events = Empty<VersionedEvent<E>>;

    #[inline]
    fn read_events<I>(
        &self,
        _id: &I,
        _version: Since,
        _max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<A>,
    {
        Ok(None)
    }
}

impl<A, E, M, V> EventSink<A, E, M, V> for NullEventStore<A, E>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    V: View<E>,
{
    type Error = Void;

    #[inline]
    fn append_events<I>(
        &self,
        _id: &I,
        _events: &[E],
        _expect: Option<Precondition>,
        _metadata: M,
    ) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<A>,
    {
        Ok(EventNumber::MIN_VALUE)
    }
}

/// A trivial store that never has any snapshots, and which always succeeds in
/// persisting data (which is immediately dropped).
#[derive(Clone, Copy)]
pub struct NullSnapshotStore<A>(PhantomData<*const A>)
where
    A: Aggregate;

impl<A> NullSnapshotStore<A>
where
    A: Aggregate,
{
    /// A constant value representing a trivial event store.
    pub const DEFAULT: Self = NullSnapshotStore(PhantomData);
}

impl<A> Default for NullSnapshotStore<A>
where
    A: Aggregate,
{
    #[inline(always)]
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl<A> fmt::Debug for NullSnapshotStore<A>
where
    A: Aggregate,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("NullSnapshotStore").finish()
    }
}

impl<A> SnapshotSource<A> for NullSnapshotStore<A>
where
    A: Aggregate,
{
    type Error = Void;

    #[inline]
    fn get_snapshot<I>(&self, _id: &I) -> Result<Option<VersionedAggregate<A>>, Self::Error>
    where
        I: AggregateId<A>,
        Self: Sized,
    {
        Ok(None)
    }
}

impl<A> SnapshotSink<A> for NullSnapshotStore<A>
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
        last_snapshot_version: Option<Version>,
    ) -> Result<Version, Self::Error>
    where
        I: AggregateId<A>,
        Self: Sized,
    {
        Ok(last_snapshot_version.unwrap_or_default())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
