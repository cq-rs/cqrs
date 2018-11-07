use std::hash::BuildHasher;
use std::fmt;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use void::Void;
use cqrs::{EventNumber, Precondition, SequencedEvent, Version};
use cqrs::StateSnapshot;
use super::*;

#[derive(Debug)]
pub struct EventStore<A, Hasher = DefaultHashBuilder>
where
    A: cqrs::Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, RwLock<Vec<A::Event>>, Hasher>>,
}

impl<A, Hasher> Default for EventStore<A, Hasher>
where
    A: cqrs::Aggregate,
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
    A: cqrs::Aggregate,
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
    A: cqrs::Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    type Events = Vec<Result<SequencedEvent<A::Event>, Void>>;
    type Error = Void;

    fn read_events<Id: AsRef<str> + Into<String>>(&self, id: Id, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        let table = self.inner.read();

        let stream = table.get(id.as_ref());

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
    A: cqrs::Aggregate,
    A::Event: Clone,
    Hasher: BuildHasher,
{
    type Error = PreconditionFailed;

    fn append_events<Id: AsRef<str> + Into<String>>(&self, id: Id, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        let table = self.inner.upgradable_read();

        if table.contains_key(id.as_ref()) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            let stream = table.get(id.as_ref()).unwrap().upgradable_read();

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
    A: cqrs::Aggregate + Clone,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, RwLock<StateSnapshot<A>>, Hasher>>,
}

impl<A, Hasher> Default for StateStore<A, Hasher>
where
    A: cqrs::Aggregate + Clone,
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
    A: cqrs::Aggregate + Clone,
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
    A: cqrs::Aggregate + Clone,
    Hasher: BuildHasher,
{
    type Error = Void;

    fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id) -> Result<Option<StateSnapshot<A>>, Self::Error> where Self: Sized {
        let table = self.inner.read();

        let snapshot = table.get(id.as_ref());

        Ok(snapshot.map(|snapshot| {
            snapshot.read().to_owned()
        }))
    }
}

impl<A, Hasher> SnapshotSink<A> for StateStore<A, Hasher>
where
    A: cqrs::Aggregate + Clone,
    Hasher: BuildHasher,
{
    type Error = Void;

    fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id, snapshot: StateSnapshot<A>) -> Result<(), Self::Error> where Self: Sized {
        let table = self.inner.upgradable_read();

        if table.contains_key(id.as_ref()) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            let mut value = table.get(id.as_ref()).unwrap().write();
            *value = snapshot;
        } else {
            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(id.into(), RwLock::new(snapshot));
        };

        Ok(())
    }
}

#[path = "memory_tests.rs"]
#[cfg(test)]
mod tests;