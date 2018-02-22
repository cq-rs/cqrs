use std::hash::{Hash, BuildHasher};

use cqrs::{EventSource, EventAppend, SnapshotSource, SnapshotPersist};
use cqrs::{Since, Precondition, VersionedEvent, VersionedSnapshot};
use cqrs::trivial::{NullEventStore,NullSnapshotStore};
use cqrs_memory::{MemoryEventStore,MemoryStateStore};
use cqrs::error::{AppendEventsError, Never};
use fnv::FnvBuildHasher;

pub enum MemoryOrNullEventStore<E, I, H = FnvBuildHasher>
    where
        E: Clone,
        I: Hash + Eq + Clone,
        H: BuildHasher + Default,
{
    Memory(MemoryEventStore<E, I, H>),
    Null(NullEventStore<E, I>),
}

impl<E, I, H> MemoryOrNullEventStore<E, I, H>
    where
        E: Clone,
        I: Hash + Eq + Clone,
        H: BuildHasher + Default,
{
    pub fn new_memory_store() -> Self {
        MemoryOrNullEventStore::Memory(MemoryEventStore::<E, I, H>::default())
    }

    pub fn new_null_store() -> Self {
        MemoryOrNullEventStore::Null(NullEventStore::<E, I>::default())
    }
}

impl<E, I, H> EventSource for MemoryOrNullEventStore<E, I, H>
    where
        E: Clone,
        I: Hash + Eq + Clone,
        H: BuildHasher + Default,
{
    type AggregateId = I;
    type Event = E;
    type Events = Vec<VersionedEvent<Self::Event>>;
    type Error = Never;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => mem.read_events(agg_id, since),
            MemoryOrNullEventStore::Null(ref nil) => nil.read_events(agg_id, since),
        }
    }
}

impl<E, I, H> EventAppend for MemoryOrNullEventStore<E, I, H>
    where
        E: Clone,
        I: Hash + Eq + Clone,
        H: BuildHasher + Default,
{
    type AggregateId = I;
    type Event = E;
    type Error = AppendEventsError<Never>;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Option<Precondition>) -> Result<(), Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => mem.append_events(agg_id, events, precondition),
            MemoryOrNullEventStore::Null(ref nil) => nil.append_events(agg_id, events, precondition),
        }
    }
}

pub enum MemoryOrNullSnapshotStore<S, I, H = FnvBuildHasher>
    where
        S: Clone,
        I: Hash + Eq + Clone,
        H: BuildHasher + Default,
{
    Memory(MemoryStateStore<S, I, H>),
    Null(NullSnapshotStore<S, I>),
}

impl<S, I, H> MemoryOrNullSnapshotStore<S, I, H>
    where
        S: Clone,
        I: Hash + Eq + Clone,
        H: BuildHasher + Default,
{
    pub fn new_memory_store() -> Self {
        MemoryOrNullSnapshotStore::Memory(MemoryStateStore::<S, I, H>::default())
    }

    pub fn new_null_store() -> Self {
        MemoryOrNullSnapshotStore::Null(NullSnapshotStore::<S, I>::default())
    }
}

impl<S, I, H> SnapshotSource for MemoryOrNullSnapshotStore<S, I, H>
    where
        S: Clone,
        I: Hash + Eq + Clone,
        H: BuildHasher + Default,
{
    type AggregateId = I;
    type Snapshot = S;
    type Error = Never;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<VersionedSnapshot<S>>, Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => mem.get_snapshot(agg_id),
            MemoryOrNullSnapshotStore::Null(ref nil) => nil.get_snapshot(agg_id),
        }
    }
}

impl<S, I, H> SnapshotPersist for MemoryOrNullSnapshotStore<S, I, H>
    where
        S: Clone,
        I: Hash + Eq + Clone,
        H: BuildHasher + Default,
{
    type AggregateId = I;
    type Snapshot = S;
    type Error = Never;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: VersionedSnapshot<S>) -> Result<(), Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => mem.persist_snapshot(agg_id, snapshot),
            MemoryOrNullSnapshotStore::Null(ref nil) => nil.persist_snapshot(agg_id, snapshot),
        }
    }
}

