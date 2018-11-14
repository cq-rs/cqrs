use std::fmt;

#[derive(Debug)]
pub enum PersistError {
    Redis(redis::RedisError),
    Serialization(rmp_serde::encode::Error),
    PreconditionFailed(cqrs_core::Precondition),
}

impl fmt::Display for PersistError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PersistError::Redis(ref e) => write!(f, "redis error: {}", e),
            PersistError::Serialization(ref e) => write!(f, "serialization error: {}", e),
            PersistError::PreconditionFailed(ref e) => write!(f, "precondition error: {}", e),
        }
    }
}

impl From<redis::RedisError> for PersistError {
    fn from(err: redis::RedisError) -> Self {
        PersistError::Redis(err)
    }
}

impl From<rmp_serde::encode::Error> for PersistError {
    fn from(err: rmp_serde::encode::Error) -> Self {
        PersistError::Serialization(err)
    }
}

impl From<cqrs_core::Precondition> for PersistError {
    fn from(precondition: cqrs_core::Precondition) -> Self {
        PersistError::PreconditionFailed(precondition)
    }
}

#[derive(Debug)]
pub enum LoadError
{
    Redis(redis::RedisError),
    Deserialization(rmp_serde::decode::Error),
}

impl fmt::Display for LoadError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LoadError::Redis(ref e) => write!(f, "redis error: {}", e),
            LoadError::Deserialization(ref e) => write!(f, "deserialization error: {}", e),
        }
    }
}

impl From<redis::RedisError> for LoadError
{
    fn from(err: redis::RedisError) -> Self {
        LoadError::Redis(err)
    }
}

impl From<rmp_serde::decode::Error> for LoadError {
    fn from(err: rmp_serde::decode::Error) -> Self {
        LoadError::Deserialization(err)
    }
}
