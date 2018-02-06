use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};
use okazis::{StateStore, PersistedState};
use std::sync::RwLock;

#[derive(PartialEq, Debug, Clone, Copy, Hash)]
pub enum Never {}

#[derive(Default)]
pub struct MemoryStateStore<StateId, Offset, State, Hasher = RandomState>
    where
        StateId: Eq + Hash,
        Hasher: BuildHasher,
{
    data: RwLock<HashMap<StateId, PersistedState<Offset, State>, Hasher>>
}

impl<StateId, Offset, State, Hasher> StateStore for MemoryStateStore<StateId, Offset, State, Hasher>
    where
        StateId: Eq + Hash,
        Offset: Clone,
        State: Clone,
        Hasher: BuildHasher,
{
    type StateId = StateId;
    type State = State;
    type Offset = Offset;
    type StateResult = Result<Option<PersistedState<Offset, State>>, Never>;

    fn get_state(&self, state_id: Self::StateId) -> Self::StateResult {
        let lock = self.data.read().unwrap();
        match lock.get(&state_id) {
            Some(s) => Ok(Some(s.clone())),
            None => Ok(None),
        }
    }

    fn put_state(&self, state_id: Self::StateId, offset: Self::Offset, state: Self::State) {
        let new_val = PersistedState { offset, state };
        let mut lock = self.data.write().unwrap();
        lock.insert(state_id, new_val);
    }
}

#[cfg(test)]
#[path = "state_store_tests.rs"]
mod tests;