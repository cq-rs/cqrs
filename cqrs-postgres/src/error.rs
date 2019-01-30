use cqrs_core::CqrsError;
use std::fmt;

/// An error while attempting to persist an event or snapshot.
#[derive(Debug)]
pub enum PersistError<E: CqrsError> {
    /// An error from the PostgreSQL backend.
    Postgres(postgres::Error),

    /// The operation failed because a specified precondition failed.
    PreconditionFailed(cqrs_core::Precondition),

    /// The operation failed because there was a serialization error.
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

/// An error while attempting to load an event or snapshot.
#[derive(Debug)]
pub enum LoadError<E: CqrsError> {
    /// An error from the PostgreSQL backend.
    Postgres(postgres::Error),

    /// The event type from the event stream is not one that can be deserialized.
    UnknownEventType(String),

    /// The operation failed because there was a deserialization error.
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
