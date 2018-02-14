use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};
use okazis::{StateStore, PersistedSnapshot, PersistResult, ReadStateResult, Version, Never};
use std::sync::RwLock;

#[derive(Debug)]
pub struct MemoryStateStore<State, AggregateId, Hasher = RandomState>
    where
        AggregateId: Eq + Hash,
        Hasher: BuildHasher,
{
    data: RwLock<HashMap<AggregateId, PersistedSnapshot<State>, Hasher>>
}

impl<State, AggregateId, Hasher> Default for MemoryStateStore<State, AggregateId, Hasher>
    where
        AggregateId: Eq + Hash,
        Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        MemoryStateStore {
            data: RwLock::new(HashMap::<_, _, Hasher>::default())
        }
    }
}

impl<State, AggregateId, Hasher> StateStore for MemoryStateStore<State, AggregateId, Hasher>
    where
        AggregateId: Eq + Hash + Clone,
        State: Clone,
        Hasher: BuildHasher,
{
    type AggregateId = AggregateId;
    type State = State;
    type StateResult = ReadStateResult<State, Never>;
    type PersistResult = PersistResult<Never>;

    fn get_state(&self, agg_id: &Self::AggregateId) -> Self::StateResult {
        let lock = self.data.read().unwrap();
        match lock.get(agg_id) {
            Some(s) => Ok(Some(s.clone())),
            None => Ok(None),
        }
    }

    fn put_state(&self, agg_id: &Self::AggregateId, version: Version, state: Self::State) -> Self::PersistResult {
        let new_val = PersistedSnapshot { version, data: state };
        let mut lock = self.data.write().unwrap();
        lock.insert(agg_id.clone(), new_val);
        Ok(())
    }
}

#[cfg(test)]
#[path = "state_store_tests.rs"]
mod tests;