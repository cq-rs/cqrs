use super::Precondition;
use std::error;
use std::fmt;

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum LoadAggregateError<EErr, SErr> {
    Snapshot(SErr),
    Events(EErr),
}

impl<EErr, SErr> fmt::Display for LoadAggregateError<EErr, SErr>
    where
        EErr: error::Error,
        SErr: error::Error,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = self as &error::Error;
        f.write_str(err.description())?;
        f.write_str(": ")?;
        write!(f, "{}", err.cause().unwrap())
    }
}

impl<EErr, SErr> error::Error for LoadAggregateError<EErr, SErr>
    where
        EErr: error::Error,
        SErr: error::Error,
{
    fn description(&self) -> &str {
        match *self {
            LoadAggregateError::Snapshot(_) => "loading snapshot",
            LoadAggregateError::Events(_) => "loading events",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            LoadAggregateError::Snapshot(ref e) => Some(e),
            LoadAggregateError::Events(ref e) => Some(e),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum PersistAggregateError<EErr, SErr> {
    Snapshot(SErr),
    Events(EErr),
}

impl<EErr, SErr> fmt::Display for PersistAggregateError<EErr, SErr>
    where
        EErr: error::Error,
        SErr: error::Error,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = self as &error::Error;
        f.write_str(err.description())?;
        f.write_str(": ")?;
        write!(f, "{}", err.cause().unwrap())
    }
}

impl<EErr, SErr> error::Error for PersistAggregateError<EErr, SErr>
    where
        EErr: error::Error,
        SErr: error::Error,
{
    fn description(&self) -> &str {
        match *self {
            PersistAggregateError::Snapshot(_) => "storing snapshot",
            PersistAggregateError::Events(_) => "storing events",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            PersistAggregateError::Snapshot(ref e) => Some(e),
            PersistAggregateError::Events(ref e) => Some(e),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum ExecuteError<CErr, LErr> {
    AggregateNotFound,
    Command(CErr),
    Load(LErr),
}

impl<CErr, LErr> fmt::Display for ExecuteError<CErr, LErr>
    where
        CErr: error::Error,
        LErr: error::Error,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = self as &error::Error;
        f.write_str(err.description())?;
        if let Some(cause) = err.cause() {
            write!(f, ": {}", cause)
        } else {
            Ok(())
        }
    }
}

impl<CErr, LErr> error::Error for ExecuteError<CErr, LErr>
    where
        CErr: error::Error,
        LErr: error::Error,
{
    fn description(&self) -> &str {
        match *self {
            ExecuteError::AggregateNotFound => "aggregate not found",
            ExecuteError::Command(_) => "invalid command",
            ExecuteError::Load(_) => "loading aggregate",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ExecuteError::AggregateNotFound => None,
            ExecuteError::Command(ref e) => Some(e),
            ExecuteError::Load(ref e) => Some(e),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum CommandAggregateError<CErr, LErr, PErr> {
    AggregateNotFound,
    Command(CErr),
    Load(LErr),
    Persist(PErr),
}

impl<CErr, LErr, PErr> fmt::Display for CommandAggregateError<CErr, LErr, PErr>
    where
        CErr: error::Error,
        LErr: error::Error,
        PErr: error::Error,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = self as &error::Error;
        f.write_str(err.description())?;
        if let Some(cause) = err.cause() {
            write!(f, ": {}", cause)
        } else {
            Ok(())
        }
    }
}

impl<CErr, LErr, PErr> error::Error for CommandAggregateError<CErr, LErr, PErr>
    where
        CErr: error::Error,
        LErr: error::Error,
        PErr: error::Error,
{
    fn description(&self) -> &str {
        match *self {
            CommandAggregateError::AggregateNotFound => "aggregate not found",
            CommandAggregateError::Command(_) => "executing command",
            CommandAggregateError::Load(_) => "loading aggregate",
            CommandAggregateError::Persist(_) => "persisting aggregate",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            CommandAggregateError::AggregateNotFound => None,
            CommandAggregateError::Command(ref e) => Some(e),
            CommandAggregateError::Load(ref e) => Some(e),
            CommandAggregateError::Persist(ref e) => Some(e),
        }
    }
}

impl<CErr, PErr, EErr, SErr> From<LoadAggregateError<EErr, SErr>> for CommandAggregateError<CErr, LoadAggregateError<EErr, SErr>, PErr>
    where
        CErr: error::Error,
        EErr: error::Error,
        SErr: error::Error,
        PErr: error::Error,
{
    fn from(err: LoadAggregateError<EErr, SErr>) -> Self {
        CommandAggregateError::Load(err)
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum AppendEventsError<Err> {
    PreconditionFailed(Precondition),
    WriteError(Err),
}

impl<Err> fmt::Display for AppendEventsError<Err>
    where
        Err: error::Error,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = self as &error::Error;
        f.write_str(err.description())?;
        f.write_str(": ")?;
        match *self {
            AppendEventsError::WriteError(ref e) => write!(f, "{}", e),
            AppendEventsError::PreconditionFailed(Precondition::Always) => f.write_str("absurd"),
            AppendEventsError::PreconditionFailed(Precondition::LastVersion(v)) => write!(f, "expected version {}", v),
            AppendEventsError::PreconditionFailed(Precondition::NewStream) => f.write_str("new stream"),
            AppendEventsError::PreconditionFailed(Precondition::EmptyStream) => f.write_str("empty stream"),
        }
    }
}

impl<Err> error::Error for AppendEventsError<Err>
    where
        Err: error::Error,
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
impl<Err> From<Precondition> for AppendEventsError<Err> {
    fn from(p: Precondition) -> Self {
        AppendEventsError::PreconditionFailed(p)
    }
}

#[cfg(not(feature = "never_type"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Never {}

#[cfg(not(feature = "never_type"))]
impl error::Error for Never {
    fn description(&self) -> &str {
        unreachable!()
    }
}

impl fmt::Display for Never {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        unreachable!()
    }
}

#[cfg(feature = "never_type")]
type Never = !;


