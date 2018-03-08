use super::{Since};
use cqrs::error::{Never};
use cqrs::{Precondition, SequencedEvent, VersionedSnapshot};
use events;
use snapshots;
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct NullEventStore<Event, AggregateId: ?Sized> {
    _phantom: PhantomData<(Event, AggregateId)>,
}

impl<Event, AggregateId: ?Sized> Default for NullEventStore<Event, AggregateId> {
    fn default() -> Self {
        NullEventStore {
            _phantom: PhantomData,
        }
    }
}

impl<Event, AggregateId: ?Sized> events::Source for NullEventStore<Event, AggregateId> {
    type AggregateId = AggregateId;
    type Result = Result<Option<Vec<SequencedEvent<Event>>>, Never>;

    #[inline]
    fn read_events(&self, _aggregate_id: &Self::AggregateId, _version: Since) -> Self::Result {
        Ok(None)
    }
}

impl<Event, AggregateId: ?Sized> events::Store for NullEventStore<Event, AggregateId> {
    type AggregateId = AggregateId;
    type Event = Event;
    type Result = Result<(), Never>;

    #[inline]
    fn append_events(&self, _aggregate_id: &Self::AggregateId, _events: &[Self::Event], _precondition: Option<Precondition>) -> Self::Result {
        Ok(())
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct NullSnapshotStore<Snapshot, AggregateId: ?Sized> {
    _phantom: PhantomData<(Snapshot, AggregateId)>,
}

impl<Snapshot, AggregateId: ?Sized> Default for NullSnapshotStore<Snapshot, AggregateId> {
    fn default() -> Self {
        NullSnapshotStore {
            _phantom: PhantomData,
        }
    }
}

impl<Snapshot, AggregateId: ?Sized> snapshots::Source for NullSnapshotStore<Snapshot, AggregateId> {
    type AggregateId = AggregateId;
    type Result = Result<Option<VersionedSnapshot<Snapshot>>, Never>;

    #[inline]
    fn get_snapshot(&self, _agg_id: &Self::AggregateId) -> Self::Result {
        Ok(None)
    }
}

impl<Snapshot, AggregateId: ?Sized> snapshots::Store for NullSnapshotStore<Snapshot, AggregateId> {
    type AggregateId = AggregateId;
    type Snapshot = Snapshot;
    type Result = Result<(), Never>;

    #[inline]
    fn persist_snapshot(&self, _agg_id: &Self::AggregateId, _snapshot: VersionedSnapshot<Self::Snapshot>) -> Self::Result {
        Ok(())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
