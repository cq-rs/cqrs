use std::hash::BuildHasher;
use std::fmt;
use std::marker::PhantomData;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use void::Void;
use super::*;

type EventStream<Event> = RwLock<Vec<Event>>;

#[derive(Debug)]
pub struct EventStore<A, Hasher = DefaultHashBuilder>
where
    A: Aggregate,
    A::Event: SerializableEvent,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, EventStream<(String, Vec<u8>)>, Hasher>>,
    _phantom: PhantomData<A>,
}

impl<A, Hasher> Default for EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: SerializableEvent,
    Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        EventStore {
            inner: RwLock::new(HashMap::default()),
            _phantom: PhantomData,
        }
    }
}

impl<A, Hasher> EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: SerializableEvent,
    Hasher: BuildHasher,
{
    pub fn with_hasher(hasher: Hasher) -> Self {
        EventStore {
            inner: RwLock::new(HashMap::with_hasher(hasher)),
            _phantom: PhantomData,
        }
    }
}

impl<A, Hasher> EventSource<A> for EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: SerializableEvent + 'static,
    Hasher: BuildHasher,
{
    type Events = Vec<Result<VersionedEvent<A::Event>, EventDeserializeError<A::Event>>>;
    type Error = EventDeserializeError<A::Event>;

    fn read_events(&self, id: &str, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error> {
        let table = self.inner.read();

        let stream = table.get(id);

        let result =
            stream.map(|stream| {
                let stream = stream.read();
                match (since, max_count) {
                    (Since::BeginningOfStream, None) => {
                        let mut next_event_number = EventNumber::MIN_VALUE;
                        stream.iter()
                            .map(|data| {
                                let sequence = next_event_number;
                                next_event_number = next_event_number.incr();
                                let event = <A::Event as SerializableEvent>::deserialize(&data.0, &data.1)?;
                                Ok(VersionedEvent { sequence, event })
                            }).collect()
                    },
                    (Since::Event(event_number), None) => {
                        let mut next_event_number = event_number.incr();
                        stream.iter()
                            .skip(next_event_number.get() as usize)
                            .map(|data| {
                                let sequence = next_event_number;
                                next_event_number = next_event_number.incr();
                                let event = <A::Event as SerializableEvent>::deserialize(&data.0, &data.1)?;
                                Ok(VersionedEvent { sequence, event })
                            }).collect()
                    },
                    (Since::BeginningOfStream, Some(max_count)) => {
                        let mut next_event_number = EventNumber::MIN_VALUE;
                        stream.iter()
                            .take(max_count.min(usize::max_value() as u64) as usize)
                            .map(|data| {
                                let sequence = next_event_number;
                                next_event_number = next_event_number.incr();
                                let event = <A::Event as SerializableEvent>::deserialize(&data.0, &data.1)?;
                                Ok(VersionedEvent { sequence, event })
                            }).collect()
                    },
                    (Since::Event(event_number), Some(max_count)) => {
                        let mut next_event_number = event_number.incr();
                        stream.iter()
                            .skip(next_event_number.get() as usize)
                            .take(max_count.min(usize::max_value() as u64) as usize)
                            .map(|data| {
                                let sequence = next_event_number;
                                next_event_number = next_event_number.incr();
                                let event = <A::Event as SerializableEvent>::deserialize(&data.0, &data.1)?;
                                Ok(VersionedEvent { sequence, event })
                            }).collect()
                    },
                }
            });

        Ok(result)
    }
}

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

impl<A, Hasher> EventSink<A> for EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: SerializableEvent,
    Hasher: BuildHasher,
{
    type Error = PreconditionFailed;

    fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        let table = self.inner.upgradable_read();

        if table.contains_key(id) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            let stream = table.get(id).unwrap().upgradable_read();

            let current_version = Version::new(stream.len() as u64);

            if let Some(precondition) = precondition {
                precondition.verify(Some(current_version))?;
            }

            let stream = &mut RwLockUpgradableReadGuard::upgrade(stream);

            stream.extend(events.into_iter().map(|e| {
                (e.event_type().into(), e.serialize())
            }));

            Ok(current_version.incr().event_number().unwrap())
        } else {
            if let Some(precondition) = precondition {
                precondition.verify(None)?;
            }

            let stream = RwLock::new(events.into_iter().map(|e| {
                (e.event_type().into(), e.serialize())
            }).collect());

            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(id.into(), stream);

            Ok(EventNumber::MIN_VALUE)
        }
    }
}

#[derive(Debug)]
pub struct StateStore<A, Hasher = DefaultHashBuilder>
where
    A: PersistableAggregate,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, RwLock<(Version, Vec<u8>)>, Hasher>>,
    _phantom: PhantomData<A>,
}

impl<A, Hasher> Default for StateStore<A, Hasher>
where
    A: PersistableAggregate,
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
    A: PersistableAggregate,
    Hasher: BuildHasher,
{
    pub fn with_hasher(hasher: Hasher) -> Self {
        StateStore {
            inner: RwLock::new(HashMap::with_hasher(hasher)),
            _phantom: PhantomData,
        }
    }
}

impl<A, Hasher> SnapshotSource<A> for StateStore<A, Hasher>
where
    A: PersistableAggregate,
    Hasher: BuildHasher,
{
    type Error = A::SnapshotError;

    fn get_snapshot(&self, id: &str) -> Result<Option<VersionedAggregate<A>>, Self::Error> where Self: Sized {
        let table = self.inner.read();

        let snapshot =
            if let Some(raw) = table.get(id) {
                let lock = raw.read();
                Some(VersionedAggregate {
                    version: lock.0,
                    payload: A::restore(&lock.1)?
                })
            } else {
                None
            };

        Ok(snapshot)
    }
}

impl<A, Hasher> SnapshotSink<A> for StateStore<A, Hasher>
where
    A: PersistableAggregate,
    Hasher: BuildHasher,
{
    type Error = Void;

    fn persist_snapshot(&self, id: &str, aggregate: VersionedAggregateView<A>) -> Result<(), Self::Error> where Self: Sized {
        let table = self.inner.upgradable_read();

        if table.contains_key(id) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            let mut raw = table.get(id).unwrap().write();
            raw.1.clear();
            aggregate.payload.snapshot_in_place(&mut raw.1);
            raw.0 = aggregate.version;
        } else {
            let raw = aggregate.payload.snapshot();
            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(id.into(), RwLock::new((aggregate.version, raw)));
        };

        Ok(())
    }
}

#[path = "memory_tests.rs"]
#[cfg(test)]
pub(crate) mod tests;