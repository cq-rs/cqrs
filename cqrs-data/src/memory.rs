use std::hash::BuildHasher;
use hashbrown::{hash_map::DefaultHashBuilder, HashMap};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use void::Void;
use ::event;
use ::state;
use ::types::Since;
use cqrs::{EventNumber, Precondition, SequencedEvent, Version};
use cqrs::StateSnapshot;

#[derive(Debug)]
pub struct EventStore<E: Clone, Hasher = DefaultHashBuilder>
where
    E: Clone,
    Hasher: BuildHasher,
{
    inner: RwLock<HashMap<String, RwLock<Vec<E>>, Hasher>>,
}

impl<E: Clone, Hasher: BuildHasher + Default> Default for EventStore<E, Hasher> {
    fn default() -> Self {
        EventStore {
            inner: RwLock::new(HashMap::default())
        }
    }
}

impl<E: Clone, Hasher: BuildHasher> EventStore<E, Hasher> {
    pub fn with_hasher(hasher: Hasher) -> Self {
        EventStore {
            inner: RwLock::new(HashMap::with_hasher(hasher))
        }
    }
}

impl<E: Clone, Hasher: BuildHasher> event::Source<E> for EventStore<E, Hasher> {
    type Events = Vec<Result<SequencedEvent<E>, Void>>;
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

#[derive(Debug)]
pub struct PreconditionFailed(pub Precondition);

impl From<Precondition> for PreconditionFailed {
    fn from(p: Precondition) -> Self {
        PreconditionFailed(p)
    }
}

impl<E: Clone, Hasher: BuildHasher> event::Store<E> for EventStore<E, Hasher> {
    type Error = PreconditionFailed;

    fn append_events<Id: AsRef<str> + Into<String>>(&self, id: Id, events: &[E], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
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
pub struct StateStore<S: Clone, Hasher: BuildHasher = DefaultHashBuilder> {
    inner: RwLock<HashMap<String, RwLock<StateSnapshot<S>>, Hasher>>,
}

impl<S: Clone, Hasher: BuildHasher + Default> Default for StateStore<S, Hasher> {
    fn default() -> Self {
        StateStore {
            inner: RwLock::new(HashMap::default())
        }
    }
}

impl<S: Clone, Hasher: BuildHasher> StateStore<S, Hasher> {
    pub fn with_hasher(hasher: Hasher) -> Self {
        StateStore {
            inner: RwLock::new(HashMap::with_hasher(hasher))
        }
    }
}

impl<S: Clone, Hasher: BuildHasher> state::Source<S> for StateStore<S, Hasher> {
    type Error = Void;

    fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id) -> Result<Option<StateSnapshot<S>>, Self::Error> where Self: Sized {
        let table = self.inner.read();

        let snapshot = table.get(id.as_ref());

        Ok(snapshot.map(|snapshot| {
            snapshot.read().to_owned()
        }))
    }
}

impl<S: Clone, Hasher: BuildHasher> state::Store<S> for StateStore<S, Hasher> {
    type Error = Void;

    fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id, snapshot: StateSnapshot<S>) -> Result<(), Self::Error> where Self: Sized {
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