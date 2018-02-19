#![cfg_attr(all(nightly, test), feature(plugin))]
#![cfg_attr(all(nightly, test), plugin(clippy))]

#[cfg(test)]
extern crate smallvec;

use std::ops::{Add, AddAssign};
use std::sync::Arc;
use std::rc::Rc;
use std::fmt;

use std::error::Error as StdError;

pub mod trivial;
pub mod domain;
pub mod error;

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

impl fmt::Display for Version {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl PartialEq<usize> for Version {
    #[inline]
    fn eq(&self, rhs: &usize) -> bool {
        *rhs == self.0
    }
}

impl PartialEq<Version> for usize {
    #[inline]
    fn eq(&self, rhs: &Version) -> bool {
        rhs.0 == *self
    }
}

impl Add<usize> for Version {
    type Output = Version;
    #[inline]
    fn add(self, rhs: usize) -> Self::Output {
        Version(self.0 + rhs)
    }
}

impl AddAssign<usize> for Version {
    #[inline]
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl ::std::borrow::Borrow<usize> for Version {
    #[inline]
    fn borrow(&self) -> &usize {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Since {
    BeginningOfStream,
    Version(Version),
}

impl From<Version> for Since {
    #[inline]
    fn from(v: Version) -> Self {
        Since::Version(v)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Precondition {
    LastVersion(Version),
    NewStream,
    EmptyStream,
}

impl From<Version> for Precondition {
    #[inline]
    fn from(v: Version) -> Self {
        Precondition::LastVersion(v)
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct VersionedEvent<Event>
{
    pub version: Version,
    pub event: Event,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct VersionedSnapshot<Snapshot> {
    pub version: Version,
    pub snapshot: Snapshot,
}

pub mod exp {
    use super::*;

    pub struct MappedEventSource<'a, ES, E, F>
        where
            ES: EventSource + 'a,
            F: Fn(ES::Event) -> E,
    {
        pub backend: &'a ES,
        pub f: F,
        pub _phantom: ::std::marker::PhantomData<E>,
    }
}

pub trait EventSource: Sized {
    type AggregateId;
    type Event;
    type Events;
    type Error: StdError;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error>;
    fn map<'a, E, F: Fn(Self::Event) -> E>(&'a self, f: F) -> exp::MappedEventSource<'a, Self, E, F> {
        exp::MappedEventSource {
            backend: &self,
            f,
            _phantom: ::std::marker::PhantomData,
        }
    }
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
    type Error: StdError;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Option<Precondition>) -> Result<(), Self::Error>;
}

impl<'a, T: EventAppend + 'a> EventAppend for &'a T {
    type AggregateId = T::AggregateId;
    type Event = T::Event;
    type Error = T::Error;

    #[inline]
    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Option<Precondition>) -> Result<(), Self::Error> {
        (**self).append_events(agg_id, events, precondition)
    }
}

impl<T: EventAppend> EventAppend for Rc<T> {
    type AggregateId = T::AggregateId;
    type Event = T::Event;
    type Error = T::Error;

    #[inline]
    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Option<Precondition>) -> Result<(), Self::Error> {
        (**self).append_events(agg_id, events, precondition)
    }
}

impl<T: EventAppend> EventAppend for Arc<T> {
    type AggregateId = T::AggregateId;
    type Event = T::Event;
    type Error = T::Error;

    #[inline]
    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Option<Precondition>) -> Result<(), Self::Error> {
        (**self).append_events(agg_id, events, precondition)
    }
}

pub trait SnapshotSource {
    type AggregateId;
    type Snapshot;
    type Error: StdError;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<VersionedSnapshot<Self::Snapshot>>, Self::Error>;
}

impl<'a, T: SnapshotSource + 'a> SnapshotSource for &'a T {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<VersionedSnapshot<Self::Snapshot>>, Self::Error> {
        (**self).get_snapshot(agg_id)
    }
}

impl<T: SnapshotSource> SnapshotSource for Rc<T> {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<VersionedSnapshot<Self::Snapshot>>, Self::Error> {
        (**self).get_snapshot(agg_id)
    }
}

impl<T: SnapshotSource> SnapshotSource for Arc<T> {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<VersionedSnapshot<Self::Snapshot>>, Self::Error> {
        (**self).get_snapshot(agg_id)
    }
}

pub trait SnapshotPersist {
    type AggregateId;
    type Snapshot;
    type Error: StdError;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: VersionedSnapshot<Self::Snapshot>) -> Result<(), Self::Error>;
}

impl<'a, T: SnapshotPersist + 'a> SnapshotPersist for &'a T {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: VersionedSnapshot<Self::Snapshot>) -> Result<(), Self::Error> {
        (**self).persist_snapshot(agg_id, snapshot)
    }
}

impl<T: SnapshotPersist> SnapshotPersist for Rc<T> {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: VersionedSnapshot<Self::Snapshot>) -> Result<(), Self::Error> {
        (**self).persist_snapshot(agg_id, snapshot)
    }
}

impl<T: SnapshotPersist> SnapshotPersist for Arc<T> {
    type AggregateId = T::AggregateId;
    type Snapshot = T::Snapshot;
    type Error = T::Error;

    #[inline]
    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: VersionedSnapshot<Self::Snapshot>) -> Result<(), Self::Error> {
        (**self).persist_snapshot(agg_id, snapshot)
    }
}

pub trait EventDecorator {
    type Event;
    type DecoratedEvent;

    fn decorate(&self, event: Self::Event) -> Self::DecoratedEvent;
    fn decorate_events<Events: IntoIterator<Item=Self::Event>>(&self, events: Events) -> Vec<Self::DecoratedEvent> {
        events.into_iter()
            .map(|e| self.decorate(e))
            .collect()
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;