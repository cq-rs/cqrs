use super::{Precondition, Since, Version};
use super::{VersionedEvent, VersionedSnapshot};
use std::borrow::Borrow;
use std::ops;

pub mod query;
pub mod command;

pub trait Aggregate: Default {
    type Events;//: Borrow<[Self::Event]> + IntoIterator<Item=Self::Event>;
    type Event;
    type Snapshot;
    type Command;
    type CommandError;

    fn from_snapshot(snapshot: Self::Snapshot) -> Self;
    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::CommandError>;
    fn snapshot(self) -> Self::Snapshot;
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

impl ops::AddAssign<usize> for AggregateVersion {
    #[inline]
    fn add_assign(&mut self, rhs: usize) {
        if rhs == 0 {
            return;
        }

        if let AggregateVersion::Version(v) = *self {
            *self = AggregateVersion::Version(v + rhs);
        } else {
            *self = AggregateVersion::Version(Version(rhs - 1))
        }
    }
}

impl From<Version> for AggregateVersion {
    #[inline]
    fn from(v: Version) -> Self {
        AggregateVersion::Version(v)
    }
}

impl From<AggregateVersion> for Since {
    fn from(v: AggregateVersion) -> Self {
        match v {
            AggregateVersion::Initial => Since::BeginningOfStream,
            AggregateVersion::Version(v) => Since::Version(v),
        }
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

#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct HydratedAggregate<Agg: Aggregate> {
    version: AggregateVersion,
    aggregate: Agg,
}

impl <Agg: Aggregate> HydratedAggregate<Agg> {
    pub fn new(version: AggregateVersion, aggregate: Agg) -> HydratedAggregate<Agg> {
        HydratedAggregate {
            version,
            aggregate
        }
    }

    #[inline]
    pub fn is_initial(&self) -> bool {
        self.version == AggregateVersion::Initial
    }

    pub fn get_version(&self) -> AggregateVersion {
        self.version
    }

    pub fn snapshot(self) -> Option<VersionedSnapshot<Agg::Snapshot>> {
        if let AggregateVersion::Version(v) = self.version {
            Some(VersionedSnapshot {
                version: v,
                snapshot: self.aggregate.snapshot(),
            })
        } else {
            None
        }
    }
}

impl<Agg, Event> HydratedAggregate<Agg>
    where
        Agg: Aggregate<Event=Event>,
{
    pub fn apply(&mut self, event: VersionedEvent<Event>) {
        self.aggregate.apply(event.event);
        self.version = event.version.into();
    }
}

impl<Agg: Aggregate> Borrow<Agg> for HydratedAggregate<Agg> {
    fn borrow(&self) -> &Agg {
        &self.aggregate
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;