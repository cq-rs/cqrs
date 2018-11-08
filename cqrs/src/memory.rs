use std::hash::BuildHasher;
use std::fmt;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use void::Void;
use super::*;

type EventStream<Event> = RwLock<Vec<Event>>;

#[derive(Debug)]
pub struct EventStore<A, Hasher = DefaultHashBuilder>
where
    A: Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, EventStream<A::Event>, Hasher>>,
}

impl<A, Hasher> Default for EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        EventStore {
            inner: RwLock::new(HashMap::default())
        }
    }
}

impl<A, Hasher> EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    pub fn with_hasher(hasher: Hasher) -> Self {
        EventStore {
            inner: RwLock::new(HashMap::with_hasher(hasher))
        }
    }
}

impl<A, Hasher> EventSource<A> for EventStore<A, Hasher>
where
    A: Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    type Events = Vec<Result<SequencedEvent<A::Event>, Void>>;
    type Error = Void;

    fn read_events(&self, id: &str, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        let table = self.inner.read();

        let stream = table.get(id);

        let result =
            stream.map(|stream| {
                let stream = stream.read();
                match since {
                    Since::BeginningOfStream => {
                        let mut next_event_number = EventNumber::MIN_VALUE;
                        stream.iter().map(|e| {
                            let sequence = next_event_number;
                            next_event_number = next_event_number.incr();
                            Ok(SequencedEvent{ sequence, event: e.to_owned()})
                        }).collect()
                    },
                    Since::Event(event_number) => {
                        let mut next_event_number = event_number.incr();
                        stream.iter().skip(next_event_number.get() as usize).map(|e| {
                            let sequence = next_event_number;
                            next_event_number = next_event_number.incr();
                            Ok(SequencedEvent{ sequence, event: e.to_owned()})
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
    A::Event: Clone,
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

            stream.extend_from_slice(events);

            Ok(current_version.incr().event_number().unwrap())
        } else {
            if let Some(precondition) = precondition {
                precondition.verify(None)?;
            }

            let stream = RwLock::new(events.into());

            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(id.into(), stream);

            Ok(EventNumber::MIN_VALUE)
        }
    }
}

#[derive(Debug)]
pub struct StateStore<A, Hasher = DefaultHashBuilder>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, RwLock<StateSnapshot<A>>, Hasher>>,
}

impl<A, Hasher> Default for StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        StateStore {
            inner: RwLock::new(HashMap::default())
        }
    }
}

impl<A, Hasher> StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    pub fn with_hasher(hasher: Hasher) -> Self {
        StateStore {
            inner: RwLock::new(HashMap::with_hasher(hasher))
        }
    }
}

impl<A, Hasher> SnapshotSource<A> for StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    type Error = Void;

    fn get_snapshot(&self, id: &str) -> Result<Option<StateSnapshot<A>>, Self::Error> where Self: Sized {
        let table = self.inner.read();

        let snapshot = table.get(id);

        Ok(snapshot.map(|snapshot| {
            snapshot.read().to_owned()
        }))
    }
}

impl<A, Hasher> SnapshotSink<A> for StateStore<A, Hasher>
where
    A: Aggregate + Clone,
    Hasher: BuildHasher,
{
    type Error = Void;

    fn persist_snapshot(&self, id: &str, snapshot: StateSnapshotView<A>) -> Result<(), Self::Error> where Self: Sized {
        let table = self.inner.upgradable_read();

        if table.contains_key(id) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            let mut value = table.get(id).unwrap().write();
            *value = StateSnapshot {
                version: snapshot.version,
                snapshot: snapshot.snapshot.to_owned(),
            };
        } else {
            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(id.into(), RwLock::new(StateSnapshot {
                version: snapshot.version,
                snapshot: snapshot.snapshot.to_owned(),
            }));
        };

        Ok(())
    }
}

#[path = "memory_tests.rs"]
#[cfg(test)]
pub(crate) mod tests;