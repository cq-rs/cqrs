use types::Precondition;
use std::error;
use std::fmt;

//#[derive(Debug, Clone, Hash, PartialEq)]
//pub enum LoadAggregateError<EErr, SErr> {
//    Snapshot(SErr),
//    Events(EErr),
//}
//
//impl<EErr, SErr> fmt::Display for LoadAggregateError<EErr, SErr>
//    where
//        EErr: error::Error,
//        SErr: error::Error,
//{
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        let err = self as &error::Error;
//        f.write_str(err.description())?;
//        f.write_str(": ")?;
//        write!(f, "{}", err.cause().unwrap())
//    }
//}
//
//impl<EErr, SErr> error::Error for LoadAggregateError<EErr, SErr>
//    where
//        EErr: error::Error,
//        SErr: error::Error,
//{
//    fn description(&self) -> &str {
//        match *self {
//            LoadAggregateError::Snapshot(_) => "loading snapshot",
//            LoadAggregateError::Events(_) => "loading events",
//        }
//    }
//
//    fn cause(&self) -> Option<&error::Error> {
//        match *self {
//            LoadAggregateError::Snapshot(ref e) => Some(e),
//            LoadAggregateError::Events(ref e) => Some(e),
//        }
//    }
//}
//
//#[derive(Debug, Clone, Hash, PartialEq)]
//pub enum PersistAggregateError<EErr, SErr> {
//    Snapshot(SErr),
//    Events(EErr),
//}
//
//impl<EErr, SErr> fmt::Display for PersistAggregateError<EErr, SErr>
//    where
//        EErr: error::Error,
//        SErr: error::Error,
//{
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        let err = self as &error::Error;
//        f.write_str(err.description())?;
//        f.write_str(": ")?;
//        write!(f, "{}", err.cause().unwrap())
//    }
//}
//
//impl<EErr, SErr> error::Error for PersistAggregateError<EErr, SErr>
//    where
//        EErr: error::Error,
//        SErr: error::Error,
//{
//    fn description(&self) -> &str {
//        match *self {
//            PersistAggregateError::Snapshot(_) => "storing snapshot",
//            PersistAggregateError::Events(_) => "storing events",
//        }
//    }
//
//    fn cause(&self) -> Option<&error::Error> {
//        match *self {
//            PersistAggregateError::Snapshot(ref e) => Some(e),
//            PersistAggregateError::Events(ref e) => Some(e),
//        }
//    }
//}
//
//#[derive(Debug, Clone, Hash, PartialEq)]
//pub enum ExecuteError<CErr, LErr> {
//    PreconditionFailed(AggregatePrecondition),
//    Command(CErr),
//    Load(LErr),
//}
//
//impl<CErr, LErr> fmt::Display for ExecuteError<CErr, LErr>
//    where
//        CErr: error::Error,
//        LErr: error::Error,
//{
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        if let ExecuteError::PreconditionFailed(ref precondition) = *self {
//            f.write_str("precondition failed: ")?;
//            match *precondition {
//                AggregatePrecondition::Exists => f.write_str("aggregate does not exist (expected existing)"),
//                AggregatePrecondition::New => f.write_str("aggregate already exists (expected new)"),
//                AggregatePrecondition::ExpectedVersion(AggregateVersion::Initial) => f.write_str("aggregate version does not match (expected default)"),
//                AggregatePrecondition::ExpectedVersion(AggregateVersion::Version(v)) => write!(f, "aggregate version does not match (expected version {})", v),
//            }
//        } else {
//            let err = self as &error::Error;
//            f.write_str(err.description())?;
//            if let Some(cause) = err.cause() {
//                write!(f, ": {}", cause)
//            } else {
//                Ok(())
//            }
//        }
//    }
//}
//
//impl<CErr, LErr> error::Error for ExecuteError<CErr, LErr>
//    where
//        CErr: error::Error,
//        LErr: error::Error,
//{
//    fn description(&self) -> &str {
//        match *self {
//            ExecuteError::PreconditionFailed(_) => "precondition failed",
//            ExecuteError::Command(_) => "invalid command",
//            ExecuteError::Load(_) => "loading aggregate",
//        }
//    }
//
//    fn cause(&self) -> Option<&error::Error> {
//        match *self {
//            ExecuteError::PreconditionFailed(_) => None,
//            ExecuteError::Command(ref e) => Some(e),
//            ExecuteError::Load(ref e) => Some(e),
//        }
//    }
//}
//
//impl<CErr, EErr, SErr> From<LoadAggregateError<EErr, SErr>> for ExecuteError<CErr, LoadAggregateError<EErr, SErr>>
//    where
//        CErr: error::Error,
//        EErr: error::Error,
//        SErr: error::Error,
//{
//    fn from(e: LoadAggregateError<EErr, SErr>) -> Self {
//        ExecuteError::Load(e)
//    }
//}
//
//#[derive(Debug, Clone, Hash, PartialEq)]
//pub enum ExecuteAndPersistError<EErr, PErr> {
//    Execute(EErr),
//    Persist(PErr),
//}
//
//impl<EErr, PErr> fmt::Display for ExecuteAndPersistError<EErr, PErr>
//    where
//        EErr: error::Error,
//        PErr: error::Error,
//{
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        let err = self as &error::Error;
//        f.write_str(err.description())?;
//        if let Some(cause) = err.cause() {
//            write!(f, ": {}", cause)
//        } else {
//            Ok(())
//        }
//    }
//}
//
//impl<EErr, PErr> error::Error for ExecuteAndPersistError<EErr, PErr>
//    where
//        EErr: error::Error,
//        PErr: error::Error,
//{
//    fn description(&self) -> &str {
//        match *self {
//            ExecuteAndPersistError::Execute(_) => "executing command",
//            ExecuteAndPersistError::Persist(_) => "persisting aggregate",
//        }
//    }
//
//    fn cause(&self) -> Option<&error::Error> {
//        match *self {
//            ExecuteAndPersistError::Execute(ref e) => Some(e),
//            ExecuteAndPersistError::Persist(ref e) => Some(e),
//        }
//    }
//}
//
//impl<XErr, EErr, SErr> From<PersistAggregateError<EErr, SErr>> for ExecuteAndPersistError<XErr, PersistAggregateError<EErr, SErr>>
//    where
//        XErr: error::Error,
//        EErr: error::Error,
//        SErr: error::Error,
//{
//    fn from(err: PersistAggregateError<EErr, SErr>) -> Self {
//        ExecuteAndPersistError::Persist(err)
//    }
//}
//
//impl<CErr, LErr, PErr> From<ExecuteError<CErr, LErr>> for ExecuteAndPersistError<ExecuteError<CErr, LErr>, PErr>
//    where
//        CErr: error::Error,
//        LErr: error::Error,
//        PErr: error::Error,
//{
//    fn from(err: ExecuteError<CErr, LErr>) -> Self {
//        ExecuteAndPersistError::Execute(err)
//    }
//}

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
            AppendEventsError::PreconditionFailed(Precondition::ExpectedVersion(v)) => write!(f, "expected aggregate with version {}", v),
            AppendEventsError::PreconditionFailed(Precondition::New) => f.write_str("expected to create new aggregate"),
            AppendEventsError::PreconditionFailed(Precondition::Exists) => f.write_str("expected existing aggregate"),
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