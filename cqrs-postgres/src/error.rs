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

pub type LoadError = postgres::Error;
