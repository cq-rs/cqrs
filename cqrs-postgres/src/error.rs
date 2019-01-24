use cqrs_core::CqrsError;
use std::fmt;

#[derive(Debug)]
pub enum PersistError<E: CqrsError> {
    Postgres(postgres::Error),
    PreconditionFailed(cqrs_core::Precondition),
    SerializationError(E),
}

impl<E: CqrsError> fmt::Display for PersistError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PersistError::Postgres(ref e) => write!(f, "postgres error: {}", e),
            PersistError::PreconditionFailed(ref e) => write!(f, "precondition error: {}", e),
            PersistError::SerializationError(ref e) => write!(f, "serialization error: {}", e),
        }
    }
}

impl<E: CqrsError> From<postgres::Error> for PersistError<E> {
    fn from(err: postgres::Error) -> Self {
        PersistError::Postgres(err)
    }
}

impl<E: CqrsError> From<cqrs_core::Precondition> for PersistError<E> {
    fn from(precondition: cqrs_core::Precondition) -> Self {
        PersistError::PreconditionFailed(precondition)
    }
}

#[derive(Debug)]
pub enum LoadError<E: CqrsError> {
    Postgres(postgres::Error),
    UnknownEventType(String),
    DeserializationError(E),
}

impl<E: CqrsError> fmt::Display for LoadError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LoadError::Postgres(ref e) => write!(f, "postgres error: {}", e),
            LoadError::DeserializationError(ref e) => write!(f, "deserialization error: {}", e),
            LoadError::UnknownEventType(ref s) => write!(f, "unknown event type: {}", s),
        }
    }
}

impl<E: CqrsError> From<postgres::Error> for LoadError<E> {
    fn from(err: postgres::Error) -> Self {
        LoadError::Postgres(err)
    }
}
