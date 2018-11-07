extern crate cqrs_core;
extern crate redis;
extern crate void;

use std::marker::PhantomData;
use void::Void;

pub use store::{Store, SnapshotStore};

pub trait RedisSerializer {
    type Value;
    type Output: redis::ToRedisArgs;
    type Input: redis::FromRedisValue;
    type Error: ::std::error::Error;

    fn serialize(&self, value: Self::Value) -> Self::Output;
    fn deserialize(&self, value: Self::Input) -> Result<Self::Value, Self::Error>;
}

pub struct IdentitySerializer<S: redis::ToRedisArgs + redis::FromRedisValue> {
    _phantom: PhantomData<S>,
}

impl<S: redis::ToRedisArgs + redis::FromRedisValue> Default for IdentitySerializer<S> {
    fn default() -> Self {
        IdentitySerializer { _phantom: PhantomData }
    }
}

impl<S: redis::ToRedisArgs + redis::FromRedisValue> RedisSerializer for IdentitySerializer<S> {
    type Value = S;
    type Output = S;
    type Input = S;
    type Error = Void;

    fn serialize(&self, value: Self::Value) -> Self::Output {
        value
    }
    fn deserialize(&self, value: Self::Input) -> Result<Self::Value, Self::Error> {
        Ok(value)
    }
}

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

    pub fn with_connection<'a, C: redis::ConnectionLike + 'a>(&'a self, conn: &'a C) -> Store<'a, C> {
        Store::new(&self, conn)
    }
}

mod store {
    use cqrs_core;
    use redis::{self, PipelineCommands};
    use super::RedisSerializer;


    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    pub struct Store<'a, C: redis::ConnectionLike + 'a> {
        config: &'a super::Config,
        conn: &'a C,
    }

    impl<'a, C: redis::ConnectionLike + 'a> Store<'a, C> {
        pub fn new(config: &'a super::Config, conn: &'a C) -> Self {
            Store {
                config,
                conn,
            }
        }

        pub fn for_snapshot<S: redis::ToRedisArgs + redis::FromRedisValue>(&self) -> SnapshotStore<C, super::IdentitySerializer<S>> {
            SnapshotStore {
                store: &self,
                serializer: super::IdentitySerializer::default(),
            }
        }

        pub fn for_snapshot_with_serializer<S: RedisSerializer>(&self, serializer: S) -> SnapshotStore<C, S> {
            SnapshotStore {
                store: &self,
                serializer,
            }
        }
    }

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    pub struct SnapshotStore<'a, C: redis::ConnectionLike + 'a, S: RedisSerializer> {
        store: &'a Store<'a, C>,
        serializer: S,
    }

    impl<'a, S, C> cqrs_core::SnapshotSink<S::Value> for SnapshotStore<'a, C, S>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
            S::Value: cqrs_core::Aggregate,
    {
        type Error = redis::RedisError;

        fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id, snapshot: cqrs_core::StateSnapshot<S::Value>) -> Result<(), Self::Error> {
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + id.as_ref().len() + 1);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str("snapshot-");
            key.push_str(id.as_ref());

            let snapshot_ver = snapshot.version.get();

            let _: () =
                redis::pipe()
                    .hset(&key, "version", snapshot_ver)
                    .hset(&key, "snapshot", self.serializer.serialize(snapshot.snapshot))
                    .query(self.store.conn)?;
            Ok(())
        }
    }

    impl<'a, S, C> cqrs_core::SnapshotSource<S::Value> for SnapshotStore<'a, C, S>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
            S::Value: cqrs_core::Aggregate,
    {
        type Error = redis::RedisError;

        fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id) -> Result<Option<cqrs_core::StateSnapshot<S::Value>>, Self::Error> {
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + id.as_ref().len() + 10);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str("snapshot-");
            key.push_str(id.as_ref());

            let result: (Option<u64>, Option<S::Input>) =
                redis::pipe()
                    .hget(&key, "version")
                    .hget(&key, "snapshot")
                    .query(self.store.conn)?;
            Ok(match result {
                (Some(snapshot_ver), Some(snapshot)) => {
                    let version = cqrs_core::Version::new(snapshot_ver);

                    Some(cqrs_core::StateSnapshot {
                        version: version,
                        snapshot: self.serializer.deserialize(snapshot).expect("the snapshot should have been deserializable"),
                    })
                },
                _ => None
            })
        }
    }

    pub struct RedisEventIterator<'a, S, C>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
            S::Value: ::std::fmt::Debug,
    {
        conn: &'a C,
        serializer: S,
        key: String,
        index: u64,
        cursor: u64,
        first_read: bool,
        buffer: Vec<S::Input>,
    }

    const PAGE_SIZE: u64 = 100;

    impl<'a, S, C> Iterator for RedisEventIterator<'a, S, C>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
            S::Value: ::std::fmt::Debug,
    {
        type Item = Result<cqrs_core::SequencedEvent<S::Value>, redis::RedisError>;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(x) = self.buffer.pop() {
                let event = cqrs_core::SequencedEvent {
                    sequence: cqrs_core::EventNumber::new(self.cursor + self.index + 1).unwrap(),
                    event: self.serializer.deserialize(x).unwrap(),
                };
                self.index += 1;
                println!("Next event: {:?}", event);
                Some(Ok(event))
            } else if !self.first_read && self.index + 1 < PAGE_SIZE {
                None
            } else {
                self.first_read = false;
                self.cursor += self.index;
                self.index = 0;
                let values: Result<Vec<Vec<S::Input>>, _> =
                    redis::pipe()
                        .lrange(&self.key, self.cursor as isize, (self.cursor + PAGE_SIZE - 1) as isize)
                        .query(self.conn);

                if let Err(e) = values {
                    return Some(Err(e));
                }

                let mut values = values.unwrap();

                self.buffer = values.pop().unwrap();
                self.buffer.reverse();
                if let Some(x) = self.buffer.pop() {
                    let event = cqrs_core::SequencedEvent {
                        sequence: cqrs_core::EventNumber::new(self.cursor + self.index + 1).unwrap(),
                        event: self.serializer.deserialize(x).unwrap(),
                    };
                    self.index += 1;
                    println!("Next event: {:?}", event);
                    Some(Ok(event))
                } else {
                    None
                }
            }
        }
    }

    impl<'a, S, C, A> cqrs_core::EventSource<A> for SnapshotStore<'a, C, S>
        where
            A: cqrs_core::Aggregate<Event=S::Value>,
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer + Clone,
            S::Value: ::std::fmt::Debug,
    {
        type Events = RedisEventIterator<'a, S, C>;
        type Error = redis::RedisError;

        fn read_events<Id: AsRef<str> + Into<String>>(&self, id: Id, since: cqrs_core::Since) -> Result<Option<Self::Events>, Self::Error> {
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + id.as_ref().len() + 1);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str(id.as_ref());

            let initial =
                if let cqrs_core::Since::Event(x) = since {
                    x.get()
                } else {
                    0
                };

            let exists: Vec<bool> = redis::pipe().exists(&key).query(self.store.conn)?;
            if exists.len() == 1 && exists[0] {
                Ok(Some(RedisEventIterator {
                    conn: self.store.conn,
                    serializer: self.serializer.clone(),
                    key,
                    cursor: initial,
                    index: 0,
                    first_read: true,
                    buffer: Vec::default(),
                }))
            } else {
                Ok(None)
            }
        }
    }
    impl<'a, S, C, A> cqrs_core::EventSink<A> for SnapshotStore<'a, C, S>
        where
            A: cqrs_core::Aggregate<Event=S::Value>,
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
            S::Value: Clone + ::std::fmt::Debug,
    {
        type Error = ::error::AppendEventsError;

        fn append_events<Id: AsRef<str> + Into<String>>(&self, id: Id, events: &[S::Value], precondition: Option<cqrs_core::Precondition>) -> Result<cqrs_core::EventNumber, Self::Error> {
            println!("Appending {} events!", events.len());
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + id.as_ref().len() + 1);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str(id.as_ref());

            let mut next_event_number = 0;
            if let Some(precondition) = precondition {
                let result: Option<()> = redis::transaction(self.store.conn, &[&key], |pipe| {
                    let (exists, len): (bool, u64) =
                        redis::pipe()
                            .exists(&key)
                            .llen(&key)
                            .query(self.store.conn)?;
                    next_event_number = len;
                    let current_version = cqrs_core::Version::new(len);

                    if let Err(_) = precondition.verify(if exists { Some(current_version) } else { None }) {
                        Ok(Some(None))
                    } else {
                        for e in events.iter() {
                            let e = e.to_owned();
                            println!("Appending event: {:?}", e);
                            pipe.rpush(&key, self.serializer.serialize(e));
                        }
                        pipe.query(self.store.conn)
                    }
                }).map_err(::error::AppendEventsError::WriteError)?;
                if result.is_none() {
                    return Err(::error::AppendEventsError::PreconditionFailed(precondition))
                }
            } else {
                let _: () = redis::transaction(self.store.conn, &[&key], |pipe| {
                    let len: (u64,) =
                        redis::pipe()
                            .llen(&key)
                            .query(self.store.conn)?;
                    next_event_number = len.0;

                    for e in events.iter() {
                        let e = e.to_owned();
                        pipe.rpush(&key, self.serializer.serialize(e));
                    }
                    pipe.query(self.store.conn)
                }).map_err(::error::AppendEventsError::WriteError)?;
            }
            Ok(cqrs_core::EventNumber::new(next_event_number + 1).unwrap())
        }
    }
}

pub mod error {
    use cqrs_core::Precondition;
    use std::error;
    use std::fmt;
    use redis::RedisError;

    #[derive(Debug, PartialEq)]
    pub enum AppendEventsError {
        PreconditionFailed(Precondition),
        WriteError(RedisError),
    }

    impl fmt::Display for AppendEventsError
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let err = self as &error::Error;
            f.write_str(err.description())?;
            f.write_str(": ")?;
            match *self {
                AppendEventsError::WriteError(ref e) => write!(f, "{}", e),
                AppendEventsError::PreconditionFailed(Precondition::ExpectedVersion(v)) => write!(f, "expected aggregate with version {}", v),
                AppendEventsError::PreconditionFailed(Precondition::New) => f.write_str("expected to create new aggregate"),
                AppendEventsError::PreconditionFailed(Precondition::Exists) => f.write_str("expected existing aggregate"),
            }
        }
    }

    impl error::Error for AppendEventsError
    {
        fn description(&self) -> &str {
            match *self {
                AppendEventsError::PreconditionFailed(_) => "precondition failed",
                AppendEventsError::WriteError(_) => "error appending events",
            }
        }

        fn cause(&self) -> Option<&error::Error> {
            match *self {
                AppendEventsError::WriteError(ref e) => Some(e),
                AppendEventsError::PreconditionFailed(_) => None,
            }
        }
    }
    impl From<Precondition> for AppendEventsError {
        fn from(p: Precondition) -> Self {
            AppendEventsError::PreconditionFailed(p)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
