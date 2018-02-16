use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};
use cqrs::{SnapshotSource, SnapshotPersist, VersionedSnapshot, Version};
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

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, version: Version, snapshot: Self::Snapshot) -> Result<(), Never> {
        let new_val = VersionedSnapshot { version, snapshot };
        let mut lock = self.data.write().unwrap();
        lock.insert(agg_id.clone(), new_val);
        Ok(())
    }
}

#[cfg(test)]
#[path = "state_store_tests.rs"]
mod tests;