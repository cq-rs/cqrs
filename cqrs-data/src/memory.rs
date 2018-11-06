use hashbrown::HashMap;
use parking_lot::{RwLock, RwLockUpgradableReadGuard, RwLockWriteGuard};
use std::hash::Hash;
use void::Void;
use ::event;
use ::state;
use ::types::Since;
use cqrs::{EventNumber, Precondition, SequencedEvent, Version};
use cqrs::StateSnapshot;

#[derive(Debug)]
pub struct EventStore<K: Clone + Eq + Hash, E: Clone> {
    inner: RwLock<HashMap<K, RwLock<Vec<E>>>>,
}

impl<K: Clone + Eq + Hash, E: Clone> Default for EventStore<K, E> {
    fn default() -> Self {
        EventStore {
            inner: RwLock::new(HashMap::new())
        }
    }
}

impl<K: Clone + Eq + Hash, E: Clone> event::Source<E> for EventStore<K, E> {
    type AggregateId = K;
    type Events = Vec<Result<SequencedEvent<E>, Void>>;
    type Error = Void;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        let table = self.inner.read();

        let stream = table.get(agg_id);

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
                        stream.iter().skip(next_event_number.get()).map(|e| {
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

impl<K: Clone + Eq + Hash, E: Clone> event::Store<E> for EventStore<K, E> {
    type AggregateId = K;
    type Error = PreconditionFailed;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[E], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        let table = self.inner.upgradable_read();

        let table =
            if table.contains_key(agg_id) {
                RwLockUpgradableReadGuard::downgrade(table)
            } else {
                let mut table = RwLockUpgradableReadGuard::upgrade(table);
                table.insert(agg_id.to_owned(), Default::default());
                RwLockWriteGuard::downgrade(table)
            };

        let stream = table.get(agg_id).unwrap().upgradable_read();

        let latest_event_number = Version::new(stream.len());

        match precondition {
            None => {}
            Some(Precondition::ExpectedVersion(Version::Initial)) if stream.is_empty() => {}
            Some(Precondition::ExpectedVersion(v)) if !stream.is_empty() && latest_event_number == v => {}
            Some(Precondition::New) if !stream.is_empty() => {}
            Some(Precondition::Exists) if stream.is_empty() => {}
            Some(precondition) => return Err(PreconditionFailed(precondition)),
        }

        let stream = &mut RwLockUpgradableReadGuard::upgrade(stream);

        stream.extend_from_slice(events);

        Ok(latest_event_number.incr().event_number().unwrap())
    }
}

#[derive(Debug)]
pub struct StateStore<K: Clone + Eq + Hash, S: Clone> {
    inner: RwLock<HashMap<K, RwLock<StateSnapshot<S>>>>,
}

impl<K: Clone + Eq + Hash, S: Clone> Default for StateStore<K, S> {
    fn default() -> Self {
        StateStore {
            inner: RwLock::new(HashMap::new())
        }
    }
}

impl<K: Clone + Eq + Hash, S: Clone> state::Source<S> for StateStore<K, S> {
    type AggregateId = K;
    type Error = Void;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<StateSnapshot<S>>, Self::Error> {
        let table = self.inner.read();

        let snapshot = table.get(agg_id);

        Ok(snapshot.map(|snapshot| {
            snapshot.read().to_owned()
        }))
    }
}

impl<K: Clone + Eq + Hash, S: Clone> state::Store<S> for StateStore<K, S> {
    type AggregateId = K;
    type Error = Void;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: StateSnapshot<S>) -> Result<(), Self::Error> {
        let table = self.inner.upgradable_read();

        if table.contains_key(agg_id) {
            let table = RwLockUpgradableReadGuard::downgrade(table);
            let mut value = table.get(agg_id).unwrap().write();
            *value = snapshot;
        } else {
            let mut table = RwLockUpgradableReadGuard::upgrade(table);
            table.insert(agg_id.to_owned(), RwLock::new(snapshot));
        };

        Ok(())
    }
}

#[path = "memory_tests.rs"]
#[cfg(test)]
mod tests;