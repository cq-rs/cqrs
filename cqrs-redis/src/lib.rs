extern crate cqrs_core;
extern crate log;
extern crate redis;

mod error;

pub use error::{LoadError, PersistError};

use std::marker::PhantomData;
use redis::ConnectionLike;

pub use store::{Store, SnapshotStore};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Config {
    key_prefix: String,
}

impl Config {
    pub fn new<S: Into<String>>(key_prefix: S) -> Self {
        Config {
            key_prefix: key_prefix.into(),
        }
    }

    pub fn with_connection<'conn, C: ConnectionLike + 'conn>(&'conn self, conn: &'conn C) -> Store<'conn, C> {
        Store::new(&self, conn)
    }
}

mod store {
    use cqrs_core::{Aggregate, EventNumber, EventSource, EventSink, PersistableAggregate, SnapshotSource, SnapshotSink, VersionedAggregate, VersionedAggregateView, Precondition, VersionedEvent, Since, Version, SerializableEvent, EventDeserializeError};
    use redis::PipelineCommands;
    use super::*;


    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    pub struct Store<'conn, C: ConnectionLike + 'conn> {
        config: &'conn Config,
        conn: &'conn C,
    }

    impl<'conn, C: ConnectionLike + 'conn> Store<'conn, C> {
        pub fn new(config: &'conn Config, conn: &'conn C) -> Self {
            Store {
                config,
                conn,
            }
        }

        pub fn for_aggregate<A: Aggregate>(&self) -> SnapshotStore<C, A>
        {
            SnapshotStore {
                store: &self,
                _phantom: PhantomData,
            }
        }
    }

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    pub struct SnapshotStore<'conn, C: ConnectionLike + 'conn, A: Aggregate> {
        store: &'conn Store<'conn, C>,
        _phantom: PhantomData<A>,
    }

    impl<'conn, A, C> SnapshotSink<A> for SnapshotStore<'conn, C, A>
    where
        C: ConnectionLike + 'conn,
        A: PersistableAggregate,
    {
        type Error = PersistError;

        fn persist_snapshot(&self, id: &str, aggregate: VersionedAggregateView<A>) -> Result<(), Self::Error> {
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + id.len() + 1);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str("snapshot-");
            key.push_str(id);

            let snapshot_ver = aggregate.version.get();
            let raw = aggregate.payload.snapshot();

            let _: () =
                redis::pipe()
                    .hset(&key, "version", snapshot_ver)
                    .hset(&key, "snapshot", raw)
                    .query(self.store.conn)?;
            Ok(())
        }
    }

    impl<'conn, A, C> SnapshotSource<A> for SnapshotStore<'conn, C, A>
    where
        C: ConnectionLike + 'conn,
        A: PersistableAggregate,
    {
        type Error = LoadError<A::SnapshotError>;

        fn get_snapshot(&self, id: &str) -> Result<Option<VersionedAggregate<A>>, Self::Error> {
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + id.len() + 10);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str("snapshot-");
            key.push_str(id);

            let result: (Option<u64>, Option<Vec<u8>>) =
                redis::pipe()
                    .hget(&key, "version")
                    .hget(&key, "snapshot")
                    .query(self.store.conn)?;
            Ok(match result {
                (Some(snapshot_ver), Some(raw)) => {
                    let version = Version::new(snapshot_ver);

                    Some(VersionedAggregate {
                        version: version,
                        payload: A::restore(&raw).map_err(LoadError::Deserialize)?,
                    })
                },
                _ => None
            })
        }
    }

    pub struct RedisEventIterator<'conn, C, E>
    where
        C: ConnectionLike + 'conn,
        E: SerializableEvent,
    {
        conn: &'conn C,
        _event: PhantomData<E>,
        key: String,
        index: u64,
        cursor: u64,
        remaining: u64,
        first_read: bool,
        buffer: Vec<Vec<u8>>,
    }

    const PAGE_SIZE: u64 = 100;

    impl<'conn, C, E> Iterator for RedisEventIterator<'conn, C, E>
    where
        C: ConnectionLike + 'conn,
        E: SerializableEvent + 'static,
    {
        type Item = Result<VersionedEvent<E>, LoadError<EventDeserializeError<E>>>;

        fn next(&mut self) -> Option<Self::Item> {
            use std::io::Read;
            if self.remaining == 0 {
                self.buffer.clear();
                None
            } else if let Some(raw) = self.buffer.pop() {
                let sequence = Version::new(self.cursor + self.index).next_event();
                log::trace!("entity {}: loaded event; sequence: {}", &self.key, sequence);
                let mut raw_ref: &[u8] = raw.as_ref();
                let mut len = [0u8; 1];
                raw_ref.read_exact(&mut len).unwrap();
                let (event_type_raw, payload) = raw_ref.split_at(len[0] as usize);
                let data = E::deserialize(::std::str::from_utf8(event_type_raw).unwrap(), payload);
                let event = data.map(|event| {
                    VersionedEvent {
                        sequence,
                        event,
                    }
                });
                self.index += 1;
                self.remaining -= 1;
                Some(event.map_err(LoadError::Deserialize))
            } else if !self.first_read && self.index + 1 < PAGE_SIZE {
                None
            } else {
                self.first_read = false;
                self.cursor += self.index;
                self.index = 0;
                let values: Result<Vec<Vec<Vec<u8>>>, _> =
                    redis::pipe()
                        .lrange(&self.key, self.cursor as isize, (self.cursor + PAGE_SIZE.min(self.remaining) - 1) as isize)
                        .query(self.conn);

                if let Err(e) = values {
                    return Some(Err(e.into()));
                }

                let mut values = values.unwrap();

                self.buffer = values.pop().unwrap();
                self.buffer.reverse();
                if let Some(mut raw) = self.buffer.pop() {
                    let sequence = EventNumber::new(self.cursor + self.index + 1).unwrap();
                    log::trace!("entity {}: loaded event; sequence: {}", &self.key, sequence);
                    let mut raw_ref: &[u8] = raw.as_ref();
                    let mut len = [0u8; 1];
                    raw_ref.read_exact(&mut len).unwrap();
                    let (event_type_raw, payload) = raw_ref.split_at(len[0] as usize);
                    let data = E::deserialize(::std::str::from_utf8(event_type_raw).unwrap(), payload);
                    let event = data.map(|event| {
                        VersionedEvent {
                            sequence,
                            event,
                        }
                    });
                    self.index += 1;
                    self.remaining -= 1;
                    Some(event.map_err(LoadError::Deserialize))
                } else {
                    None
                }
            }
        }
    }

    impl<'conn, C, A> EventSource<A> for SnapshotStore<'conn, C, A>
    where
        A: Aggregate,
        A::Event: SerializableEvent + 'static,
        C: ConnectionLike + 'conn,
    {
        type Events = RedisEventIterator<'conn, C, A::Event>;
        type Error = LoadError<EventDeserializeError<A::Event>>;

        fn read_events(&self, id: &str, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error> {
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + id.len() + 1);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str(id);

            let initial =
                if let Since::Event(x) = since {
                    x.get()
                } else {
                    0
                };

            let exists: Vec<bool> = redis::pipe().exists(&key).query(self.store.conn)?;
            if exists.len() == 1 && exists[0] {
                Ok(Some(RedisEventIterator {
                    conn: self.store.conn,
                    _event: PhantomData,
                    key,
                    cursor: initial,
                    index: 0,
                    remaining: max_count.unwrap_or(u64::max_value()),
                    first_read: true,
                    buffer: Vec::default(),
                }))
            } else {
                Ok(None)
            }
        }
    }

    impl<'conn, C, A> EventSink<A> for SnapshotStore<'conn, C, A>
    where
        A: Aggregate,
        A::Event: SerializableEvent,
        C: ConnectionLike + 'conn,
    {
        type Error = PersistError;

        fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
            log::trace!("Appending {} events!", events.len());
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + id.len() + 1);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str(id);

            let mut last_event_number = 0;
            if let Some(precondition) = precondition {
                let result: Option<()> = redis::transaction(self.store.conn, &[&key], |pipe| {
                    let (exists, len): (bool, u64) =
                        redis::pipe()
                            .exists(&key)
                            .llen(&key)
                            .query(self.store.conn)?;
                    last_event_number = len;
                    let current_version = Version::new(len);

                    if let Err(_) = precondition.verify(if exists { Some(current_version) } else { None }) {
                        Ok(Some(None))
                    } else {
                        for e in events.iter() {
                            log::trace!("entity {}; appending event", id);
                            let mut serialized = Vec::new();
                            e.serialize_in_place(&mut serialized);
                            let event_type = e.event_type();
                            let len = event_type.len();
                            debug_assert!(len < u8::max_value() as usize);
                            let mut data = Vec::with_capacity(serialized.len() + len + 1);
                            data.push(len as u8);
                            data.extend_from_slice(event_type.as_ref());
                            data.extend_from_slice(&serialized);
                            pipe.rpush(&key, data);
                        }
                        pipe.query(self.store.conn)
                    }
                })?;
                if result.is_none() {
                    return Err(PersistError::PreconditionFailed(precondition))
                }
            } else {
                let _: () = redis::transaction(self.store.conn, &[&key], |pipe| {
                    let len: (u64,) =
                        redis::pipe()
                            .llen(&key)
                            .query(self.store.conn)?;
                    last_event_number = len.0;

                    for e in events.iter() {
                        log::trace!("entity {}; appending event", id);
                        let mut serialized = Vec::new();
                        e.serialize_in_place(&mut serialized);
                        let event_type = e.event_type();
                        let len = event_type.len();
                        debug_assert!(len < u8::max_value() as usize);
                        let mut data = Vec::with_capacity(serialized.len() + len + 1);
                        data.push(len as u8);
                        data.extend_from_slice(event_type.as_ref());
                        data.extend_from_slice(&serialized);
                        pipe.rpush(&key, data);
                    }
                    pipe.query(self.store.conn)
                })?;
            }
            Ok(Version::new(last_event_number).next_event())
        }
    }
}
