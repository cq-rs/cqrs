use std::ops::{Add, AddAssign};
use std::sync::Arc;
use std::rc::Rc;
use std::error;

pub mod trivial;
pub mod domain;
pub mod store;

#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(usize);

impl Version {
    pub fn new(version: usize) -> Self {
        Version(version)
    }

    pub fn number(&self) -> usize {
        self.0
    }
}

impl PartialEq<usize> for Version {
    fn eq(&self, rhs: &usize) -> bool {
        *rhs == self.0
    }
}

impl PartialEq<Version> for usize {
    fn eq(&self, rhs: &Version) -> bool {
        rhs.0 == *self
    }
}

impl Add<usize> for Version {
    type Output = Version;
    fn add(self, rhs: usize) -> Self::Output {
        Version(self.0 + rhs)
    }
}

impl AddAssign<usize> for Version {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl AsRef<usize> for Version {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AggregateVersion {
    Initial,
    Version(Version),
}

impl Default for AggregateVersion {
    #[inline]
    fn default() -> Self {
        AggregateVersion::Initial
    }
}

impl From<Version> for AggregateVersion {
    #[inline]
    fn from(v: Version) -> Self {
        AggregateVersion::Version(v)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Since {
    BeginningOfStream,
    Version(Version),
}

impl From<AggregateVersion> for Since {
    fn from(v: AggregateVersion) -> Self {
        match v {
            AggregateVersion::Initial => Since::BeginningOfStream,
            AggregateVersion::Version(v) => Since::Version(v),
        }
    }
}

impl From<Version> for Since {
    #[inline]
    fn from(v: Version) -> Self {
        Since::Version(v)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Precondition {
    Always,
    LastVersion(Version),
    NewStream,
    EmptyStream,
}

impl Default for Precondition {
    #[inline]
    fn default() -> Self {
        Precondition::Always
    }
}

impl From<Version> for Precondition {
    #[inline]
    fn from(v: Version) -> Self {
        Precondition::LastVersion(v)
    }
}

impl From<AggregateVersion> for Precondition {
    fn from(av: AggregateVersion) -> Self {
        if let AggregateVersion::Version(v) = av {
            Precondition::LastVersion(v)
        } else {
            Precondition::EmptyStream
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum LoadResult<T, E> {
    Found(T),
    NotFound,
    Error(E),
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum AppendError<Err> {
    PreconditionFailed(Precondition),
    WriteError(Err),
}

impl<Err> From<Precondition> for AppendError<Err> {
    fn from(p: Precondition) -> Self {
        AppendError::PreconditionFailed(p)
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedEvent<Event>
{
    pub version: Version,
    pub event: Event,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedSnapshot<State> {
    pub version: Version,
    pub data: State,
}

pub trait EventSource {
    type AggregateId;
    type Event;
    type Events: IntoIterator<Item=PersistedEvent<Self::Event>>;
    type Error: error::Error;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error>;
}

impl<'a, T: EventSource + 'a> EventSource for &'a T {
    type AggregateId = T::AggregateId;
    type Event = T::Event;
    type Events = T::Events;
    type Error = T::Error;

    #[inline]
    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        (**self).read_events(agg_id, since)
    }
}

impl<T: EventSource> EventSource for Rc<T> {
    type AggregateId = T::AggregateId;
    type Event = T::Event;
    type Events = T::Events;
    type Error = T::Error;

    #[inline]
    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        (**self).read_events(agg_id, since)
    }
}

impl<T: EventSource> EventSource for Arc<T> {
    type AggregateId = T::AggregateId;
    type Event = T::Event;
    type Events = T::Events;
    type Error = T::Error;

    #[inline]
    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        (**self).read_events(agg_id, since)
    }
}

pub trait EventAppend {
    type AggregateId;
    type Event;
    type Error: error::Error;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], condition: Precondition) -> Result<(), Self::Error>;
}

impl<'a, T: EventAppend + 'a> EventAppend for &'a T {
    type AggregateId = T::AggregateId;
    type Event = T::Event;
    type Error = T::Error;

    #[inline]
    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], condition: Precondition) -> Result<(), Self::Error> {
        (**self).append_events(agg_id, events, condition)
    }
}

impl<T: EventAppend> EventAppend for Rc<T> {
    type AggregateId = T::AggregateId;
    type Event = T::Event;
    type Error = T::Error;

    #[inline]
    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], condition: Precondition) -> Result<(), Self::Error> {
        (**self).append_events(agg_id, events, condition)
    }
}

impl<T: EventAppend> EventAppend for Arc<T> {
    type AggregateId = T::AggregateId;
    type Event = T::Event;
    type Error = T::Error;

    #[inline]
    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], condition: Precondition) -> Result<(), Self::Error> {
        (**self).append_events(agg_id, events, condition)
    }
}

pub trait SnapshotSource {
    type AggregateId;
    type Snapshot;
    type Error: error::Error;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<PersistedSnapshot<Self::Snapshot>>, Self::Error>;
}

impl<'a, T: SnapshotSource + 'a> SnapshotSource for &'a T {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<PersistedSnapshot<Self::Snapshot>>, Self::Error> {
        (**self).get_snapshot(agg_id)
    }
}

impl<T: SnapshotSource> SnapshotSource for Rc<T> {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<PersistedSnapshot<Self::Snapshot>>, Self::Error> {
        (**self).get_snapshot(agg_id)
    }
}

impl<T: SnapshotSource> SnapshotSource for Arc<T> {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<PersistedSnapshot<Self::Snapshot>>, Self::Error> {
        (**self).get_snapshot(agg_id)
    }
}

pub trait SnapshotPersist {
    type AggregateId;
    type Snapshot;
    type Error: error::Error;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, version: Version, snapshot: Self::Snapshot) -> Result<(), Self::Error>;
}

impl<'a, T: SnapshotPersist + 'a> SnapshotPersist for &'a T {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn persist_snapshot(&self, agg_id: &Self::AggregateId, version: Version, snapshot: Self::Snapshot) -> Result<(), Self::Error> {
        (**self).persist_snapshot(agg_id, version, snapshot)
    }
}

impl<T: SnapshotPersist> SnapshotPersist for Rc<T> {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn persist_snapshot(&self, agg_id: &Self::AggregateId, version: Version, snapshot: Self::Snapshot) -> Result<(), Self::Error> {
        (**self).persist_snapshot(agg_id, version, snapshot)
    }
}

impl<T: SnapshotPersist> SnapshotPersist for Arc<T> {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn persist_snapshot(&self, agg_id: &Self::AggregateId, version: Version, snapshot: Self::Snapshot) -> Result<(), Self::Error> {
        (**self).persist_snapshot(agg_id, version, snapshot)
    }
}

pub trait EventDecorator {
    type Event;
    type DecoratedEvent;

    fn decorate(&self, event: Self::Event) -> Self::DecoratedEvent;
    fn decorate_events(&self, events: Vec<Self::Event>) -> Vec<Self::DecoratedEvent> {
        events.into_iter()
            .map(|e| self.decorate(e))
            .collect()
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

impl ::std::fmt::Display for Never {
    fn fmt(&self, _: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        unreachable!()
    }
}

#[cfg(feature = "never_type")]
type Never = !;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;