use okazis::{StateStore, PersistedState};
use std::sync::RwLock;

#[derive(PartialEq, Debug, Clone, Copy, Hash)]
pub enum Never {}

#[derive(Default)]
pub struct MemoryStateStore<Offset, State> {
    value: RwLock<Option<PersistedState<Offset, State>>>
}

impl<Offset, State> StateStore for MemoryStateStore<Offset, State>
    where
        Offset: Clone,
        State: Clone,
{
    type StateId = usize;
    type State = State;
    type Offset = Offset;
    type StateResult = Result<Option<PersistedState<Offset, State>>, Never>;

    fn get_state(&self, stream_id: Self::StateId) -> Self::StateResult {
        Ok(self.value.read().unwrap().clone())
    }

    fn put_state(&self, stream_id: Self::StateId, offset: Self::Offset, state: Self::State) {
        let new_val = PersistedState { offset, state };
        let mut lock = self.value.write().unwrap();
        *lock = Some(new_val);
    }
}

#[cfg(test)]
#[path = "state_store_tests.rs"]
mod tests;