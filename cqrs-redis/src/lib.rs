extern crate cqrs;
extern crate redis;

use std::marker::PhantomData;

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
    type Error = ::cqrs::error::Never;

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
    use cqrs;
    use redis;
    use redis::{Commands, PipelineCommands};
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

    impl<'a, S, C> cqrs::SnapshotPersist for SnapshotStore<'a, C, S>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
    {
        type AggregateId = str;
        type Snapshot = S::Value;
        type Error = redis::RedisError;

        fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: cqrs::StateSnapshot<Self::Snapshot>) -> Result<(), Self::Error> {
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + agg_id.len() + 1);
            key.push_str("snapshot-");
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str(agg_id);

            let _: () =
                redis::pipe()
                    .hset(&key, "version", snapshot.version.number())
                    .hset(&key, "snapshot", self.serializer.serialize(snapshot.snapshot))
                    .query(self.store.conn)?;
            Ok(())
        }
    }

    impl<'a, S, C> cqrs::SnapshotSource for SnapshotStore<'a, C, S>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
    {
        type AggregateId = str;
        type Snapshot = S::Value;
        type Error = redis::RedisError;

        fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<cqrs::StateSnapshot<Self::Snapshot>>, Self::Error> {
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + agg_id.len() + 10);
            key.push_str("snapshot-");
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str(agg_id);

            let result: (Option<usize>, Option<S::Input>) =
                redis::pipe()
                    .hget(&key, "version")
                    .hget(&key, "snapshot")
                    .query(self.store.conn)?;
            Ok(match result {
                (Some(version), Some(snapshot)) =>
                    Some(cqrs::StateSnapshot {
                        version: cqrs::Version::new(version),
                        snapshot: self.serializer.deserialize(snapshot).expect("the snapshot should have been deserializable"),
                    }),
                _ => None
            })
        }
    }

    pub struct RedisEventIterator<'a, S, C>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
    {
        conn: &'a C,
        serializer: S,
        key: String,
        index: usize,
        cursor: usize,
        first_read: bool,
        buffer: Vec<S::Input>,
    }

    const PAGE_SIZE: usize = 100;

    impl<'a, S, C> Iterator for RedisEventIterator<'a, S, C>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
    {
        type Item = cqrs::VersionedEvent<S::Value>;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(x) = self.buffer.pop() {
                let event = cqrs::VersionedEvent {
                    version: cqrs::Version::new(self.cursor + self.index),
                    event: self.serializer.deserialize(x).unwrap(),
                };
                self.index += 1;
                Some(event)
            } else if !self.first_read && self.index + 1 < PAGE_SIZE {
                None
            } else {
                self.first_read = false;
                self.cursor += self.index;
                self.index = 0;
                let mut values: Vec<Vec<S::Input>> =
                    redis::pipe()
                        .lrange(&self.key, self.cursor as isize, (self.cursor + PAGE_SIZE - 1) as isize)
                        .query(self.conn)
                        .unwrap();
                self.buffer = values.pop().unwrap();
                if let Some(x) = self.buffer.pop() {
                    let event = cqrs::VersionedEvent {
                        version: cqrs::Version::new(self.cursor + self.index),
                        event: self.serializer.deserialize(x).unwrap(),
                    };
                    self.index += 1;
                    Some(event)
                } else {
                    None
                }
            }
        }
    }

    impl<'a, S, C> cqrs::EventSource for SnapshotStore<'a, C, S>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer + Clone,
    {
        type AggregateId = str;
        type Events = RedisEventIterator<'a, S, C>;
        type Event = S::Value;
        type Error = redis::RedisError;

        fn read_events(&self, agg_id: &Self::AggregateId, since: cqrs::Since) -> Result<Option<Self::Events>, Self::Error> {
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + agg_id.len() + 1);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str(agg_id);

            let initial =
                if let cqrs::Since::Version(v) = since {
                    v.number() + 1
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
    impl<'a, S, C> cqrs::EventAppend for SnapshotStore<'a, C, S>
        where
            C: redis::ConnectionLike + 'a,
            S: RedisSerializer,
            S::Value: Clone,
    {
        type AggregateId = str;
        type Event = S::Value;
        type Error = cqrs::error::AppendEventsError<redis::RedisError>;

        fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Option<cqrs::Precondition>) -> Result<(), Self::Error> {
            println!("Appending!");
            let mut key = String::with_capacity(self.store.config.key_prefix.len() + agg_id.len() + 1);
            key.push_str(&self.store.config.key_prefix);
            key.push('-');
            key.push_str(agg_id);

            if let Some(precondition) = precondition {
                let result: Option<()> = redis::transaction(self.store.conn, &[&key], |pipe| {
                    let (exists, len): (bool, usize) =
                        redis::pipe()
                            .exists(&key)
                            .llen(&key)
                            .query(self.store.conn)?;

                    let valid =
                        match precondition {
                            cqrs::Precondition::NewStream => !exists,
                            cqrs::Precondition::EmptyStream => !exists || len == 0,
                            cqrs::Precondition::LastVersion(ver) => len > 0 && len - 1 == ver.number()
                        };

                    if !valid {
                        Ok(None)
                    } else {
                        for e in events.iter() {
                            let e = e.to_owned();
                            pipe.rpush(&key, self.serializer.serialize(e));
                        }
                        pipe.query(self.store.conn)
                    }
                }).map_err(cqrs::error::AppendEventsError::WriteError)?;
                if result.is_none() {
                    return Err(cqrs::error::AppendEventsError::PreconditionFailed(precondition))
                }
            } else {
                let result: () = redis::transaction(self.store.conn, &[&key], |pipe| {
                    for e in events.iter() {
                        let e = e.to_owned();
                        pipe.rpush(&key, self.serializer.serialize(e));
                    }
                    pipe.query(self.store.conn)
                }).map_err(cqrs::error::AppendEventsError::WriteError)?;
            }
            Ok(())
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
