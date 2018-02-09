use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};
use okazis::{StateStore, PersistedSnapshot};
use std::sync::RwLock;
use super::Never;

#[derive(Default)]
pub struct MemoryStateStore<AggregateId, Version, State, Hasher = RandomState>
    where
        AggregateId: Eq + Hash,
        Hasher: BuildHasher,
{
    data: RwLock<HashMap<AggregateId, PersistedSnapshot<Version, State>, Hasher>>
}

impl<AggregateId, Version, State, Hasher> StateStore for MemoryStateStore<AggregateId, Version, State, Hasher>
    where
        AggregateId: Eq + Hash + Clone,
        Version: Clone,
        State: Clone,
        Hasher: BuildHasher,
{
    type AggregateId = AggregateId;
    type State = State;
    type Version = Version;
    type StateResult = Result<Option<PersistedSnapshot<Version, State>>, Never>;
    type PersistResult = Result<(), Never>;

    fn get_state(&self, agg_id: &Self::AggregateId) -> Self::StateResult {
        let lock = self.data.read().unwrap();
        match lock.get(agg_id) {
            Some(s) => Ok(Some(s.clone())),
            None => Ok(None),
        }
    }

    fn put_state(&self, agg_id: &Self::AggregateId, version: Self::Version, state: Self::State) -> Self::PersistResult {
        let new_val = PersistedSnapshot { version, data: state };
        let mut lock = self.data.write().unwrap();
        lock.insert(agg_id.clone(), new_val);
        Ok(())
    }
}

#[cfg(test)]
#[path = "state_store_tests.rs"]
mod tests;