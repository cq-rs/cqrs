use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};
use cqrs::{SnapshotSource, SnapshotPersist, VersionedSnapshot};
use cqrs::error::Never;
use std::sync::RwLock;

#[derive(Debug)]
pub struct MemoryStateStore<State, AggId, Hasher = RandomState>
    where
        AggId: Eq + Hash,
        Hasher: BuildHasher,
{
    data: RwLock<HashMap<AggId, VersionedSnapshot<State>, Hasher>>
}

impl<State, AggId, Hasher> Default for MemoryStateStore<State, AggId, Hasher>
    where
        AggId: Eq + Hash,
        Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        MemoryStateStore {
            data: RwLock::new(HashMap::<_, _, Hasher>::default())
        }
    }
}

impl<Snapshot, AggId, Hasher> SnapshotSource for MemoryStateStore<Snapshot, AggId, Hasher>
    where
        AggId: Eq + Hash + Clone,
        Snapshot: Clone,
        Hasher: BuildHasher,
{
    type AggregateId = AggId;
    type Snapshot = Snapshot;
    type Error = Never;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<VersionedSnapshot<Self::Snapshot>>, Self::Error> {
        let lock = self.data.read().unwrap();
        match lock.get(agg_id) {
            Some(s) => Ok(Some(s.clone())),
            None => Ok(None),
        }
    }
}

impl<Snapshot, AggId, Hasher> SnapshotPersist for MemoryStateStore<Snapshot, AggId, Hasher>
    where
        AggId: Eq + Hash + Clone,
        Snapshot: Clone,
        Hasher: BuildHasher,
{
    type AggregateId = AggId;
    type Snapshot = Snapshot;
    type Error = Never;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: VersionedSnapshot<Self::Snapshot>) -> Result<(), Never> {
        self.data.write().unwrap()
            .insert(agg_id.clone(), snapshot);
        Ok(())
    }
}

#[cfg(test)]
#[path = "state_store_tests.rs"]
mod tests;