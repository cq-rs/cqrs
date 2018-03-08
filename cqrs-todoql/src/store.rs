use cqrs::{EventSource, EventAppend, SnapshotSource, SnapshotPersist};
use cqrs::{Since, Precondition, VersionedEvent, VersionedSnapshot};
use cqrs::trivial::{NullEventStore,NullSnapshotStore};
use cqrs_memory::{MemoryEventStore,MemoryStateStore};
use cqrs::error::{AppendEventsError, Never};
use fnv::FnvBuildHasher;

use cqrs_redis;
use cqrs_todo_core::{Event, TodoAggregate};

use r2d2;
use r2d2_redis::RedisConnectionManager;

pub enum MemoryOrNullEventStore
{
    Memory(MemoryEventStore<Event, String, FnvBuildHasher>),
    Null(NullEventStore<Event, String>),
    Redis(cqrs_redis::Config, r2d2::Pool<RedisConnectionManager>)
}

impl MemoryOrNullEventStore
{
    pub fn new_memory_store() -> Self {
        MemoryOrNullEventStore::Memory(MemoryEventStore::default())
    }

    pub fn new_null_store() -> Self {
        MemoryOrNullEventStore::Null(NullEventStore::default())
    }

    pub fn new_redis_store(config: cqrs_redis::Config, pool: r2d2::Pool<RedisConnectionManager>) -> Self {
        MemoryOrNullEventStore::Redis(config, pool)
    }
}

impl EventSource for MemoryOrNullEventStore
{
    type AggregateId = String;
    type Event = Event;
    type Events = Vec<VersionedEvent<Self::Event>>;
    type Error = Never;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => mem.read_events(agg_id, since),
            MemoryOrNullEventStore::Null(ref nil) => nil.read_events(agg_id, since),
            MemoryOrNullEventStore::Redis(ref config, ref pool) => {
                let conn = pool.get().unwrap();
                let x = config.with_connection(&*conn);
                let y = x
                    .for_snapshot_with_serializer(SerdeSnapshotSerializer::default())
                    .read_events(agg_id, since)
                    .unwrap();
                Ok(y.map(|x| x.collect()))
            }
        }
    }
}

impl EventAppend for MemoryOrNullEventStore
{
    type AggregateId = String;
    type Event = Event;
    type Error = AppendEventsError<Never>;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Option<Precondition>) -> Result<(), Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => mem.append_events(agg_id, events, precondition),
            MemoryOrNullEventStore::Null(ref nil) => nil.append_events(agg_id, events, precondition),
            MemoryOrNullEventStore::Redis(ref config, ref pool) => {
                let conn = pool.get().unwrap();
                config.with_connection(&*conn)
                    .for_snapshot_with_serializer(SerdeSnapshotSerializer::default())
                    .append_events(agg_id, events, precondition)
                    .unwrap();
                Ok(())
            },
        }
    }
}

pub enum MemoryOrNullSnapshotStore
{
    Memory(MemoryStateStore<TodoAggregate, String, FnvBuildHasher>),
    Null(NullSnapshotStore<TodoAggregate, String>),
    Redis(cqrs_redis::Config, r2d2::Pool<RedisConnectionManager>)
}

impl MemoryOrNullSnapshotStore
{
    pub fn new_memory_store() -> Self {
        MemoryOrNullSnapshotStore::Memory(MemoryStateStore::default())
    }

    pub fn new_null_store() -> Self {
        MemoryOrNullSnapshotStore::Null(NullSnapshotStore::default())
    }

    pub fn new_redis_store(config: cqrs_redis::Config, pool: r2d2::Pool<RedisConnectionManager>) -> Self {
        MemoryOrNullSnapshotStore::Redis(config, pool)
    }
}

impl SnapshotSource for MemoryOrNullSnapshotStore
{
    type AggregateId = String;
    type Snapshot = TodoAggregate;
    type Error = Never;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<VersionedSnapshot<Self::Snapshot>>, Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => mem.get_snapshot(agg_id),
            MemoryOrNullSnapshotStore::Null(ref nil) => nil.get_snapshot(agg_id),
            MemoryOrNullSnapshotStore::Redis(ref config, ref mgr) => {
                let x = config.with_connection(&*mgr.get().unwrap())
                    .for_snapshot_with_serializer(SerdeSnapshotSerializer::default())
                    .get_snapshot(agg_id)
                    .unwrap();
                Ok(x)
            },
        }
    }
}

impl SnapshotPersist for MemoryOrNullSnapshotStore
{
    type AggregateId = String;
    type Snapshot = TodoAggregate;
    type Error = Never;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: VersionedSnapshot<Self::Snapshot>) -> Result<(), Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => mem.persist_snapshot(agg_id, snapshot),
            MemoryOrNullSnapshotStore::Null(ref nil) => nil.persist_snapshot(agg_id, snapshot),
            MemoryOrNullSnapshotStore::Redis(ref config, ref mgr) => {
                config.with_connection(&*mgr.get().unwrap())
                    .for_snapshot_with_serializer(SerdeSnapshotSerializer::default())
                    .persist_snapshot(agg_id, snapshot)
                    .unwrap();
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SerdeSnapshotSerializer<S: ::serde::Serialize + ::serde::de::DeserializeOwned> {
    _phantom: ::std::marker::PhantomData<S>,
}

impl<S: ::serde::Serialize + ::serde::de::DeserializeOwned> Default for SerdeSnapshotSerializer<S> {
    fn default() -> Self {
        SerdeSnapshotSerializer {
            _phantom: ::std::marker::PhantomData,
        }
    }
}

impl<S: ::serde::Serialize + ::serde::de::DeserializeOwned> cqrs_redis::RedisSerializer for SerdeSnapshotSerializer<S> {
    type Value = S;
    type Output = Vec<u8>;
    type Input = Vec<u8>;
    type Error = ::serde_json::Error;

    fn serialize(&self, snapshot: Self::Value) -> Self::Output {
        ::serde_json::to_vec(&snapshot).unwrap()
    }

    fn deserialize(&self, snapshot: Self::Input) -> Result<Self::Value, Self::Error> {
        ::serde_json::from_slice(&snapshot)
    }
}