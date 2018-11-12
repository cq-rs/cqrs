use cqrs_core::CqrsError;
use std::fmt;

#[derive(Debug)]
pub enum PersistError {
    Redis(redis::RedisError),
    PreconditionFailed(cqrs_core::Precondition),
}

impl fmt::Display for PersistError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PersistError::Redis(ref e) => write!(f, "redis error: {}", e),
            PersistError::PreconditionFailed(ref e) => write!(f, "precondition error: {}", e),
        }
    }
}

impl From<redis::RedisError> for PersistError {
    fn from(err: redis::RedisError) -> Self {
        PersistError::Redis(err)
    }
}

impl From<cqrs_core::Precondition> for PersistError {
    fn from(precondition: cqrs_core::Precondition) -> Self {
        PersistError::PreconditionFailed(precondition)
    }
}

#[derive(Debug)]
pub enum LoadError<E>
where E: CqrsError
{
    Redis(redis::RedisError),
    Deserialize(E),
}

impl<E> fmt::Display for LoadError<E>
where E: CqrsError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LoadError::Redis(ref e) => write!(f, "redis error: {}", e),
            LoadError::Deserialize(ref e) => write!(f, "deserialization error: {}", e),
        }
    }
}

impl<E> From<redis::RedisError> for LoadError<E>
where E: CqrsError
{
    fn from(err: redis::RedisError) -> Self {
        LoadError::Redis(err)
    }
}
