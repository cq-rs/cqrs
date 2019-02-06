use std::{fmt, num::NonZeroU64};

/// Represents an event sequence number, starting at 1
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventNumber(NonZeroU64);

impl EventNumber {
    /// The minimum [EventNumber].
    #[allow(unsafe_code)]
    pub const MIN_VALUE: EventNumber =
        // One is absolutely non-zero, and this is required for this to be usable in a `const` context.
        EventNumber(unsafe {NonZeroU64::new_unchecked(1)});

    /// Attempts to create a new event number from a given number. Will return non if the given number is `0`.
    #[inline]
    pub fn new(x: u64) -> Option<Self> {
        Some(EventNumber(NonZeroU64::new(x)?))
    }

    /// Extracts the event number as a plain `u64`.
    #[inline]
    pub fn get(self) -> u64 {
        self.0.get()
    }

    /// Increments the event number to the next value.
    #[inline]
    pub fn incr(&mut self) {
        self.0 = NonZeroU64::new(self.0.get() + 1).unwrap();
    }

    /// Gets the event number after the current one.
    #[inline]
    #[must_use]
    pub fn next(self) -> Self {
        let mut slf = self;
        slf.0 = NonZeroU64::new(self.0.get() + 1).unwrap();
        slf
    }
}

impl fmt::Display for EventNumber {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

//impl PartialEq<usize> for EventNumber {
//    #[inline]
//    fn eq(&self, rhs: &usize) -> bool {
//        *rhs == self.0
//    }
//}
//
//impl PartialEq<EventNumber> for usize {
//    #[inline]
//    fn eq(&self, rhs: &EventNumber) -> bool {
//        rhs.0 == *self
//    }
//}

/// An aggregate version.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    /// The version of an aggregate that has not had any events applied to it.
    Initial,
    /// The version of the last event applied to the aggregate.
    Number(EventNumber),
}

impl Version {
    /// Creates a new `Version` from a number.
    ///
    /// The number `0` gets interpreted as being `Version::Initial`, while any other number is interpreted as the
    /// latest event number applied.
    #[inline]
    pub fn new(number: u64) -> Self {
        NonZeroU64::new(number)
            .map(EventNumber)
            .map(Version::Number)
            .unwrap_or(Version::Initial)
    }

    /// Increments the version number to the next in sequence.
    #[inline]
    pub fn incr(&mut self) {
        match *self {
            Version::Initial => *self = Version::Number(EventNumber::MIN_VALUE),
            Version::Number(ref mut en) => en.incr(),
        }
    }

    /// Returns the next event number in the sequence.
    #[inline]
    pub fn next_event(self) -> EventNumber {
        match self {
            Version::Initial => EventNumber::MIN_VALUE,
            Version::Number(mut en) => {
                en.incr();
                en
            }
        }
    }

    /// Gets the version number as a raw `u64`.
    #[inline]
    pub fn get(self) -> u64 {
        match self {
            Version::Initial => 0,
            Version::Number(en) => en.get(),
        }
    }

    /// Gets the version number as an [EventNumber], returning `None` if the current verison is [Version::Initial].
    #[inline]
    pub fn event_number(self) -> Option<EventNumber> {
        match self {
            Version::Initial => None,
            Version::Number(en) => Some(en),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Version::Initial => f.write_str("initial"),
            Version::Number(ref event_number) => event_number.fmt(f),
        }
    }
}

impl Default for Version {
    #[inline]
    fn default() -> Self {
        Version::Initial
    }
}

//impl PartialEq<EventNumber> for Version {
//    fn eq(&self, rhs: &EventNumber) -> bool {
//        if let Version::Number(ref v) = *self {
//            v == rhs
//        } else {
//            false
//        }
//    }
//}

impl From<EventNumber> for Version {
    #[inline]
    fn from(event_number: EventNumber) -> Self {
        Version::Number(event_number)
    }
}

impl ::std::ops::Sub for Version {
    type Output = i64;

    fn sub(self, rhs: Version) -> Self::Output {
        let l = match self {
            Version::Initial => 0,
            Version::Number(n) => n.get() as i64,
        };
        let r = match rhs {
            Version::Initial => 0,
            Version::Number(n) => n.get() as i64,
        };

        l - r
    }
}

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

/// A structured tuple combining an event number and an event.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct VersionedEvent<E> {
    /// The event number.
    pub sequence: EventNumber,

    /// The event.
    pub event: E,
}

/// A structured tuple combining an aggregate and its current version.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VersionedAggregate<A> {
    /// The current version of the aggregate.
    pub version: Version,

    /// The aggregate.
    pub payload: A,
}

/// The starting point when reading a stream of values from an [EventSource].
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Since {
    /// Begins reading events from the very beginning of the stream.
    BeginningOfStream,

    /// Begins reading events after the given [EventNumber].
    ///
    /// E.g. if the event number were 4, then reading should begin at event number 5.
    Event(EventNumber),
}

impl From<Version> for Since {
    fn from(v: Version) -> Self {
        match v {
            Version::Initial => Since::BeginningOfStream,
            Version::Number(x) => Since::Event(x),
        }
    }
}

/// A recommendation on whether or not a snapshot should be persisted.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SnapshotRecommendation {
    /// Recommends that a snapshot be taken.
    ShouldSnapshot,

    /// Recommends that a snapshot should not be taken.
    DoNotSnapshot,
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
