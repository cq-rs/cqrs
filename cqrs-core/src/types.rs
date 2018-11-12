use std::fmt;
use std::num::NonZeroU64;
use aggregate::PersistableAggregate;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventNumber(NonZeroU64);

impl EventNumber {
    pub const MIN_VALUE: EventNumber = EventNumber(unsafe {NonZeroU64::new_unchecked(1)});

    #[inline]
    pub fn new(x: u64) -> Option<Self> {
        Some(EventNumber(NonZeroU64::new(x)?))
    }

    #[inline]
    pub fn get(self) -> u64 {
        self.0.get()
    }

    #[inline]
    #[must_use]
    pub fn incr(self) -> Self {
        let x = self.0.get();
        EventNumber(NonZeroU64::new(x + 1).unwrap())
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    Initial,
    Number(EventNumber),
}

impl Version {
    #[inline]
    pub fn new(number: u64) -> Self {
        NonZeroU64::new(number)
            .map(EventNumber)
            .map(Version::Number)
            .unwrap_or(Version::Initial)
    }

    #[inline]
    #[must_use]
    pub fn incr(self) -> Self {
        match self {
            Version::Initial => Version::Number(EventNumber::MIN_VALUE),
            Version::Number(en) => Version::Number(en.incr()),
        }
    }

    #[inline]
    pub fn next_event(self) -> EventNumber {
        match self {
            Version::Initial => EventNumber::MIN_VALUE,
            Version::Number(en) => en.incr(),
        }
    }

    #[inline]
    pub fn get(self) -> u64 {
        match self {
            Version::Initial => 0,
            Version::Number(en) => en.get(),
        }
    }

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
            Version::Number(n) => n.get() as i64
        };
        let r = match rhs {
            Version::Initial => 0,
            Version::Number(n) => n.get() as i64
        };

        l - r
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Precondition {
    New,
    Exists,
    ExpectedVersion(Version),
}

impl Precondition {
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
            Precondition::ExpectedVersion(Version::Initial) => f.write_str("expect aggregate to exist in initial state"),
            Precondition::ExpectedVersion(Version::Number(v)) => write!(f, "expect aggregate to exist with version {}", v),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct VersionedEvent<Event>
{
    pub sequence: EventNumber,
    pub event: Event,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RawEvent {
    pub event_type: &'static str,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct VersionedRawEvent {
    pub event_type: &'static str,
    pub payload: Vec<u8>
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VersionedAggregate<A>
where A: PersistableAggregate
{
    pub version: Version,
    pub payload: A,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VersionedAggregateView<'a, A>
    where A: PersistableAggregate + 'a
{
    pub version: Version,
    pub payload: &'a A,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct VersionedRawSnapshot {
    pub version: Version,
    pub payload: Vec<u8>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct VersionedRawSnapshotView<'a> {
    pub version: Version,
    pub payload: &'a [u8],
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Since {
    BeginningOfStream,
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

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum EventDeserializeError<E>
where E: super::SerializableEvent
{
    UnknownEventType(String),
    InvalidPayload(E::PayloadError),
}

impl<E> EventDeserializeError<E>
where E: super::SerializableEvent
{
    pub fn new_unknown_event_type(event_type: impl Into<String>) -> Self {
        EventDeserializeError::UnknownEventType(event_type.into())
    }
}

impl<E> fmt::Debug for EventDeserializeError<E>
where
    E: super::SerializableEvent,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EventDeserializeError::UnknownEventType(ref evt_type) =>
                f.debug_tuple("UnknownEventType")
                    .field(evt_type)
                    .finish(),
            EventDeserializeError::InvalidPayload(ref err) =>
                f.debug_tuple("InvalidPayload")
                    .field(err)
                    .finish(),
        }
    }
}

impl<E> fmt::Display for EventDeserializeError<E>
where E: super::SerializableEvent
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EventDeserializeError::UnknownEventType(ref evt_type) =>
                write!(f, "event deserialize error; unknown event type: {}", evt_type),
            EventDeserializeError::InvalidPayload(ref err) =>
                write!(f, "event deserialize error; invalid payload: {}", err),
        }
    }
}

pub trait CqrsError: fmt::Debug + fmt::Display + Send + Sync + 'static {}

impl<T> CqrsError for T
where T: fmt::Debug + fmt::Display + Send + Sync + 'static {}