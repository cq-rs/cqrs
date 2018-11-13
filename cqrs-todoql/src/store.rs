use cqrs::{EventNumber, Precondition, VersionedEvent, VersionedAggregate, VersionedAggregateView};
use cqrs::Since;
use cqrs::{EventSink, EventSource, SnapshotSink, SnapshotSource, SerializableEvent, EventDeserializeError};
use cqrs::trivial::NullStore;
use cqrs::memory::{EventStore,StateStore};

use cqrs_redis;
use cqrs_redis::{LoadError, PersistError};
use cqrs_todo_core::{Event, TodoAggregate};

use r2d2;
use r2d2_redis::RedisConnectionManager;

use void::ResultVoidExt;

#[derive(Debug)]
pub enum MemoryOrNullEventStore {
    Memory(EventStore<TodoAggregate>),
    Null,
    Redis(cqrs_redis::Config, r2d2::Pool<RedisConnectionManager>)
}

impl MemoryOrNullEventStore {
    pub fn new_memory_store() -> Self {
        MemoryOrNullEventStore::Memory(EventStore::default())
    }

    pub fn new_null_store() -> Self {
        MemoryOrNullEventStore::Null
    }

    pub fn new_redis_store(config: cqrs_redis::Config, pool: r2d2::Pool<RedisConnectionManager>) -> Self {
        MemoryOrNullEventStore::Redis(config, pool)
    }
}

impl EventSource<TodoAggregate> for MemoryOrNullEventStore {
    type Events = Vec<Result<VersionedEvent<Event>, Self::Error>>;
    type Error = LoadError<EventDeserializeError<Event>>;

    fn read_events(&self, id: &str, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => Ok(mem.read_events(id, since, max_count).void_unwrap().map(|es| es.into_iter().map(|e| Ok(e.void_unwrap())).collect())),
            MemoryOrNullEventStore::Null => Ok(EventSource::<TodoAggregate>::read_events(&NullStore, id, since, max_count).void_unwrap().map(|es| es.into_iter().map(|r| Ok(r.void_unwrap())).collect())),
            MemoryOrNullEventStore::Redis(ref config, ref pool) => {
                let conn = pool.get().unwrap();
                let c = config.with_connection(&*conn);
                let store = c.for_aggregate::<TodoAggregate>();
                let y = EventSource::<TodoAggregate>::read_events(&store, id, since, max_count)?;
                Ok(y.map(|x| x.collect()))
            }
        }
    }
}

impl EventSink<TodoAggregate> for MemoryOrNullEventStore {
    type Error = PersistError;

    fn append_events(&self, id: &str, events: &[Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        match *self {
            MemoryOrNullEventStore::Memory(ref mem) => Ok(mem.append_events(id, events, precondition).map_err(|::cqrs::memory::PreconditionFailed(p)| PersistError::PreconditionFailed(p))?),
            MemoryOrNullEventStore::Null => Ok(EventSink::<TodoAggregate>::append_events(&NullStore, id, events, precondition).void_unwrap()),
            MemoryOrNullEventStore::Redis(ref config, ref pool) => {
                let conn = pool.get().unwrap();
                let c = config.with_connection(&*conn);
                let store = c.for_aggregate::<TodoAggregate>();
                let e = EventSink::<TodoAggregate>::append_events(&store, id, events, precondition)?;
                Ok(e)
            },
        }
    }
}

#[derive(Debug)]
pub enum MemoryOrNullSnapshotStore {
    Memory(StateStore<TodoAggregate>),
    Null,
    Redis(cqrs_redis::Config, r2d2::Pool<RedisConnectionManager>)
}

impl MemoryOrNullSnapshotStore {
    pub fn new_memory_store() -> Self {
        MemoryOrNullSnapshotStore::Memory(StateStore::default())
    }

    pub fn new_null_store() -> Self {
        MemoryOrNullSnapshotStore::Null
    }

    pub fn new_redis_store(config: cqrs_redis::Config, pool: r2d2::Pool<RedisConnectionManager>) -> Self {
        MemoryOrNullSnapshotStore::Redis(config, pool)
    }
}

impl SnapshotSource<TodoAggregate> for MemoryOrNullSnapshotStore {
    type Error = LoadError<<Event as SerializableEvent>::PayloadError>;

    fn get_snapshot(&self, id: &str) -> Result<Option<VersionedAggregate<TodoAggregate>>, Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => Ok(mem.get_snapshot(id).void_unwrap()),
            MemoryOrNullSnapshotStore::Null => Ok(NullStore.get_snapshot(id).void_unwrap()),
            MemoryOrNullSnapshotStore::Redis(ref config, ref mgr) => {
                let x = config.with_connection(&*mgr.get().unwrap())
                    .for_aggregate::<TodoAggregate>()
                    .get_snapshot(id)?;
                Ok(x)
            },
        }
    }
}

impl SnapshotSink<TodoAggregate> for MemoryOrNullSnapshotStore {
    type Error = PersistError;

    fn persist_snapshot(&self, id: &str, snapshot: VersionedAggregateView<TodoAggregate>) -> Result<(), Self::Error> {
        match *self {
            MemoryOrNullSnapshotStore::Memory(ref mem) => Ok(mem.persist_snapshot(id, snapshot).void_unwrap()),
            MemoryOrNullSnapshotStore::Null => Ok(NullStore.persist_snapshot(id, snapshot).void_unwrap()),
            MemoryOrNullSnapshotStore::Redis(ref config, ref mgr) => {
                let data = config.with_connection(&*mgr.get().unwrap())
                    .for_aggregate::<TodoAggregate>()
                    .persist_snapshot(id, snapshot)?;
                Ok(data)
            }
        }
    }
}
