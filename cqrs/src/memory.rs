//! A basic, in-memory event stream.

use cqrs_core::{
    Aggregate, AggregateEvent, AggregateId, EventNumber, EventSink, EventSource, Precondition,
    Since, SnapshotSink, SnapshotSource, Version, VersionedAggregate, VersionedEvent,
};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use std::{
    collections::{hash_map::RandomState, HashMap},
    fmt,
    hash::BuildHasher,
    iter,
    marker::PhantomData,
    sync::Arc,
};
use void::Void;

#[derive(Debug, Default)]
struct EventStream<Event, Metadata> {
    events: Vec<Event>,
    metadata: Vec<Arc<Metadata>>,
}

type LockedHashMap<K, V, H> = RwLock<HashMap<K, V, H>>;
type LockedEventStream<E, M> = RwLock<EventStream<VersionedEvent<E>, M>>;

/// An in-memory event store
#[derive(Debug)]
pub struct EventStore<A, E, M, Hasher = RandomState>
where
    A: Aggregate,
    E: AggregateEvent<A> + Clone,
    Hasher: BuildHasher,
{
    inner: LockedHashMap<String, LockedEventStream<E, M>, Hasher>,
    _phantom: PhantomData<*const A>,
}

impl<A, E, M, Hasher> Default for EventStore<A, E, M, Hasher>
where
    A: Aggregate,
    E: AggregateEvent<A> + Clone,
    Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        EventStore {
            inner: RwLock::new(HashMap::default()),
            _phantom: PhantomData,
        }
    }
}

impl<A, E, M, Hasher> EventStore<A, E, M, Hasher>
where
    A: Aggregate,
    E: AggregateEvent<A> + Clone,
    Hasher: BuildHasher,
{
    /// Constructs a new event store with the specified hasher.
    pub fn with_hasher(hasher: Hasher) -> Self {
        EventStore {
            inner: RwLock::new(HashMap::with_hasher(hasher)),
            _phantom: PhantomData,
        }
    }
}

impl<A, E, M, Hasher> EventSource<A, E> for EventStore<A, E, M, Hasher>
where
    A: Aggregate,
    E: AggregateEvent<A> + Clone,
    Hasher: BuildHasher,
{
    type Error = Void;
    type Events = Vec<VersionedEvent<E>>;

    fn _read_events<I>(
        &self,
        id: Option<&I>,
        since: Since,
        max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<A>,
    {
        let table = self.inner.read();

        let stream = match id {
            Some(id) => {
                let r = vec![table.get(id.as_str())];
                r
            },
            None => {
                let r = table.values().map(|v| Some(v)).collect();
                r
            },
        }.into_iter().collect::<Option<Vec<_>>>();

        match stream {
            Some(stream) => {
                let r = stream.into_iter().map(|stream| {
                    let stream = stream.read();
                    match (since, max_count) {
                        (Since::BeginningOfStream, None) => stream.events.iter().map(ToOwned::to_owned).collect::<Vec<_>>(),
                        (Since::Event(event_number), None) => stream
                            .events
                            .iter()
                            .skip(event_number.get() as usize)
                            .map(ToOwned::to_owned)
                            .collect::<Vec<_>>(),
                        (Since::BeginningOfStream, Some(max_count)) => stream
                            .events
                            .iter()
                            .take(max_count.min(usize::max_value() as u64) as usize)
                            .map(ToOwned::to_owned)
                            .collect::<Vec<_>>(),
                        (Since::Event(event_number), Some(max_count)) => stream
                            .events
                            .iter()
                            .skip(event_number.get() as usize)
                            .take(max_count.min(usize::max_value() as u64) as usize)
                            .map(ToOwned::to_owned)
                            .collect::<Vec<_>>(),
                    }
                }).flatten().collect::<Vec<_>>();
                Ok(Some(r))
            },
            None => Ok(None),
        }
    }
}

/// An error indicating that a precondition has failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreconditionFailed(pub Precondition);

impl From<Precondition> for PreconditionFailed {
    fn from(p: Precondition) -> Self {
        PreconditionFailed(p)
    }
}

impl fmt::Display for PreconditionFailed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "precondition failed: {}", self.0)
    }
}

impl<A, E, M, Hasher> EventSink<A, E, M> for EventStore<A, E, M, Hasher>
where
    A: Aggregate,
    E: AggregateEvent<A> + Clone,
    Hasher: BuildHasher,
{
    type Error = PreconditionFailed;

    fn append_events<I>(
        &self,
        id: &I,
        events: &[E],
        precondition: Option<Precondition>,
        metadata: M,
    ) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<A>,
    {
        let table = self.inner.upgradable_read();

        if table.contains_key(id.as_str()) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            let stream = table.get(id.as_str()).unwrap().upgradable_read();

            let mut sequence = Version::new(stream.events.len() as u64).next_event();
            let first_sequence = sequence;

            if let Some(precondition) = precondition {
                precondition.verify(Some(first_sequence.into()))?;
            }

            let stream = &mut RwLockUpgradableReadGuard::upgrade(stream);

            let metadata = Arc::new(metadata);
            stream
                .metadata
                .extend(iter::repeat(metadata).take(events.len()));

            stream.events.extend(events.iter().map(|event| {
                let versioned_event = VersionedEvent {
                    sequence,
                    event: event.to_owned(),
                };
                sequence.incr();
                versioned_event
            }));

            Ok(first_sequence)
        } else {
            if let Some(precondition) = precondition {
                precondition.verify(None)?;
            }

            let mut sequence = EventNumber::MIN_VALUE;

            let metadata = Arc::new(metadata);
            let metadata_stream = iter::repeat(metadata).take(events.len()).collect();

            let new_stream = EventStream {
                events: events
                    .iter()
                    .map(|event| {
                        let versioned_event = VersionedEvent {
                            sequence,
                            event: event.to_owned(),
                        };
                        sequence.incr();
                        versioned_event
                    })
                    .collect(),
                metadata: metadata_stream,
            };

            let stream = RwLock::new(new_stream);

            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(id.as_str().into(), stream);

            Ok(EventNumber::MIN_VALUE)
        }
    }
}

/// An in-memory store for aggregate snapshots.
#[derive(Debug)]
pub struct StateStore<A, Hasher = RandomState>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, RwLock<VersionedAggregate<A>>, Hasher>>,
    _phantom: PhantomData<A>,
}

impl<A, Hasher> Default for StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        StateStore {
            inner: RwLock::new(HashMap::default()),
            _phantom: PhantomData,
        }
    }
}

impl<A, Hasher> StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    /// Constructs a new snapshot store with a specific hasher.
    pub fn with_hasher(hasher: Hasher) -> Self {
        StateStore {
            inner: RwLock::new(HashMap::with_hasher(hasher)),
            _phantom: PhantomData,
        }
    }
}

impl<A, Hasher> SnapshotSource<A> for StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    type Error = Void;

    fn get_snapshot<I>(&self, id: &I) -> Result<Option<VersionedAggregate<A>>, Self::Error>
    where
        I: AggregateId<A>,
        Self: Sized,
    {
        let table = self.inner.read();

        let snapshot = table.get(id.as_str()).map(|data| data.read().to_owned());

        Ok(snapshot)
    }
}

impl<A, Hasher> SnapshotSink<A> for StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    type Error = Void;

    fn persist_snapshot<I>(
        &self,
        id: &I,
        aggregate: &A,
        version: Version,
        _last_snapshot_version: Option<Version>,
    ) -> Result<Version, Self::Error>
    where
        I: AggregateId<A>,
        Self: Sized,
    {
        let table = self.inner.upgradable_read();

        let owned_aggregate = VersionedAggregate {
            version,
            payload: aggregate.to_owned(),
        };

        if table.contains_key(id.as_str()) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            *table.get(id.as_str()).unwrap().write() = owned_aggregate;
        } else {
            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(id.as_str().into(), RwLock::new(owned_aggregate));
        };

        Ok(version)
    }
}

#[path = "memory_tests.rs"]
#[cfg(test)]
pub(crate) mod tests;
