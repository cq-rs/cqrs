#![warn(
    unused_import_braces,
    unused_imports,
    unused_qualifications,
    missing_docs,
)]

#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
)]

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
    use std::collections::VecDeque;
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

    impl<'conn, C, A> SnapshotStore<'conn, C, A>
    where
        A: Aggregate,
        A::Event: SerializableEvent,
        C: ConnectionLike + 'conn,
    {
        fn serialize_event(event: &A::Event) -> Result<Vec<u8>, redis::RedisError> {
            let mut buffer = Vec::new();
            let event_type = event.event_type();
            let len = event_type.len();
            debug_assert!(len < u8::max_value() as usize);
            buffer.push(len as u8);
            buffer.extend_from_slice(event_type.as_ref());

            {
                use std::io::{Cursor, Seek, SeekFrom};
                let mut cursor = Cursor::new(&mut buffer);
                cursor.seek(SeekFrom::End(0)).map_err(redis::RedisError::from)?;
                event.serialize_to_writer(&mut cursor)?;
            }

            Ok(buffer)
        }
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
            let raw = aggregate.payload.snapshot().map_err(redis::RedisError::from)?;

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

    #[derive(Debug)]
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
        buffer: VecDeque<Vec<u8>>,
    }

    impl<'conn, C, E> RedisEventIterator<'conn, C, E>
    where
        C: ConnectionLike + 'conn,
        E: SerializableEvent + 'static,
    {
        fn read_event_from_buffer(&mut self, buffer: &[u8]) -> Result<VersionedEvent<E>, EventDeserializeError<E>> {
            use std::io::Read;
            use std::str;

            let sequence = Version::new(self.cursor + self.index).next_event();
            let mut buffer_ref: &[u8] = buffer.as_ref();
            let mut len = [0u8; 1];
            buffer_ref.read_exact(&mut len).unwrap();
            let (event_type_raw, payload) = buffer_ref.split_at(len[0] as usize);
            let event_type =
                match str::from_utf8(event_type_raw) {
                    Ok(e) => e,
                    Err(_) =>
                        return Err(EventDeserializeError::UnknownEventType(String::from("<invalid utf-8>"))),
                };
            let data = E::deserialize(event_type, payload);
            let event = data.map(|event| {
                VersionedEvent {
                    sequence,
                    event,
                }
            });

            log::trace!("entity {}: loaded event; sequence: {}", &self.key, sequence);
            self.index += 1;
            self.remaining -= 1;
            event
        }

        fn get_next_buffer(&mut self) -> Result<Option<Vec<u8>>, redis::RedisError> {
            if let Some(buffer) = self.buffer.pop_front() {
                Ok(Some(buffer))
            } else if !self.first_read && self.index + 1 < PAGE_SIZE {
                Ok(None)
            } else {
                self.load_page()?;
                if let Some(buffer) = self.buffer.pop_front() {
                    Ok(Some(buffer))
                } else {
                    Ok(None)
                }
            }
        }

        fn load_page(&mut self) -> Result<(), redis::RedisError> {
            self.first_read = false;
            self.cursor += self.index;
            self.index = 0;
            let mut values: Vec<Vec<Vec<u8>>> =
                redis::pipe()
                    .lrange(&self.key, self.cursor as isize, (self.cursor + PAGE_SIZE.min(self.remaining) - 1) as isize)
                    .query(self.conn)?;

            self.buffer.clear();
            self.buffer.extend(values.pop().unwrap());
            Ok(())
        }
    }

    const PAGE_SIZE: u64 = 100;

    impl<'conn, C, E> Iterator for RedisEventIterator<'conn, C, E>
    where
        C: ConnectionLike + 'conn,
        E: SerializableEvent + 'static,
    {
        type Item = Result<VersionedEvent<E>, LoadError<EventDeserializeError<E>>>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.remaining == 0 {
                self.buffer.clear();
                None
            } else {
                match self.get_next_buffer() {
                    Ok(Some(buffer)) => Some(self.read_event_from_buffer(&buffer).map_err(LoadError::Deserialize)),
                    Ok(None) => None,
                    Err(e) => Some(Err(LoadError::Redis(e))),
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
                    buffer: VecDeque::default(),
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
                            pipe.rpush(&key, Self::serialize_event(e)?);
                            log::trace!("entity {}; appending event", id);
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
                        pipe.rpush(&key, Self::serialize_event(e)?);
                        log::trace!("entity {}; appending event", id);
                    }
                    pipe.query(self.store.conn)
                })?;
            }
            Ok(Version::new(last_event_number).next_event())
        }
    }
}
