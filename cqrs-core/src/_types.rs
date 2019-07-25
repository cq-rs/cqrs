use std::{
    convert::{TryFrom, TryInto as _},
    fmt,
    num::{NonZeroU64, NonZeroU8, TryFromIntError},
};

use super::{Aggregate, AggregateCommand, AggregateEvent};

/// A precondition that must be upheld for a command to be executed or for events to be persisted.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Precondition {
    /// Requires that the target aggregate must not have had any events previously applied to it.
    New,
    /// Requires that the target event at least exist, whether as a snapshot, or as having had at least one event applied.
    Exists,
    /// Requires that the target aggregate have the exact version specified.
    ExpectedVersion(Version),
}

impl Precondition {
    /// Verifies that the precondition holds, given the `current_version`. If the precondition is violated, an error is
    /// returned with the precondition.
    pub fn verify(self, current_version: Option<Version>) -> Result<(), Self> {
        match (self, current_version) {
            (Precondition::ExpectedVersion(Version::Initial), None) => Ok(()),
            (Precondition::ExpectedVersion(Version::Initial), Some(Version::Initial)) => Ok(()),
            (Precondition::ExpectedVersion(e), Some(x)) if e == x => Ok(()),
            (Precondition::New, None) => Ok(()),
            (Precondition::Exists, Some(_)) => Ok(()),
            (precondition, _) => Err(precondition),
        }
    }
}

impl From<Version> for Precondition {
    #[inline]
    fn from(v: Version) -> Self {
        Precondition::ExpectedVersion(v)
    }
}

impl fmt::Display for Precondition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Precondition::Exists => f.write_str("expect aggregate exists"),
            Precondition::New => f.write_str("expect aggregate does not exist"),
            Precondition::ExpectedVersion(Version::Initial) => {
                f.write_str("expect aggregate to exist in initial state")
            }
            Precondition::ExpectedVersion(Version::Number(v)) => {
                write!(f, "expect aggregate to exist with version {}", v)
            }
        }
    }
}

/// Represents a common trait that all errors handled by CQRS should implement.
pub trait CqrsError: fmt::Debug + fmt::Display + Send + Sync + 'static {}

impl<T> CqrsError for T where T: fmt::Debug + fmt::Display + Send + Sync + 'static {}

/// An owned, raw view of event data.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RawEvent {
    /// The event id.
    pub event_id: EventNumber,
    /// The aggregate type.
    pub aggregate_type: String,
    /// The entity id.
    pub entity_id: String,
    /// The sequence number of this event in the entity's event stream.
    pub sequence: EventNumber,
    /// The event type.
    pub event_type: String,
    /// The raw event payload.
    pub payload: Vec<u8>,
}

/// An owned, raw view of event data.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct BorrowedRawEvent<'row> {
    /// The event id.
    pub event_id: EventNumber,
    /// The aggregate type.
    pub aggregate_type: &'row str,
    /// The entity id.
    pub entity_id: &'row str,
    /// The sequence number of this event in the entity's event stream.
    pub sequence: EventNumber,
    /// The event type.
    pub event_type: &'row str,
    /// The raw event payload.
    pub payload: &'row [u8],
}
