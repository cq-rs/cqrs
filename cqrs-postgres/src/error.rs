use cqrs_core::CqrsError;
use std::fmt;

#[derive(Debug)]
pub enum PersistError {
    Postgres(postgres::Error),
    PreconditionFailed(cqrs_core::Precondition),
}

impl fmt::Display for PersistError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PersistError::Postgres(ref e) => write!(f, "postgres error: {}", e),
            PersistError::PreconditionFailed(ref e) => write!(f, "precondition error: {}", e),
        }
    }
}

impl From<postgres::Error> for PersistError {
    fn from(err: postgres::Error) -> Self {
        PersistError::Postgres(err)
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
    Postgres(postgres::Error),
    Deserialize(E),
}

impl<E> fmt::Display for LoadError<E>
where E: CqrsError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LoadError::Postgres(ref e) => write!(f, "postgres error: {}", e),
            LoadError::Deserialize(ref e) => write!(f, "deserialization error: {}", e),
        }
    }
}

impl<E> From<postgres::Error> for LoadError<E>
where E: CqrsError
{
    fn from(err: postgres::Error) -> Self {
        LoadError::Postgres(err)
    }
}

