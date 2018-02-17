use super::{Precondition, Since, Version};
use super::{VersionedEvent, VersionedSnapshot};
use std::borrow::Borrow;
use std::ops;
use std::fmt;

pub mod query;
pub mod command;

pub trait Aggregate: Default {
    type Events;//: Borrow<[Self::Event]> + IntoIterator<Item=Self::Event>;
    type Event;
    type Command;
    type CommandError;

    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::CommandError>;
}

pub trait SnapshotAggregate: Aggregate {
    type Snapshot;

    fn to_snapshot(self) -> Self::Snapshot;
}

pub trait RestoreAggregate: Aggregate {
    type Snapshot;

    fn restore(snapshot: Self::Snapshot) -> Self;
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum AggregateVersion {
    Initial,
    Version(Version),
}

impl fmt::Display for AggregateVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AggregateVersion::Initial => f.write_str("initial"),
            AggregateVersion::Version(ref v) => v.fmt(f),
        }
    }
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
    rehydrated_version: AggregateVersion,
}

impl <Agg: Aggregate> HydratedAggregate<Agg> {
    #[inline]
    pub fn is_initial(&self) -> bool {
        self.version == AggregateVersion::Initial
    }

    pub fn get_version(&self) -> AggregateVersion {
        self.version
    }

    pub fn inspect_aggregate(&self) -> &Agg {
        &self.aggregate
    }

    pub fn last_snapshot(&self) -> AggregateVersion {
        self.rehydrated_version
    }
}

impl<Agg: RestoreAggregate> HydratedAggregate<Agg> {
    fn restore(snapshot: Option<VersionedSnapshot<Agg::Snapshot>>) -> Self {
        if let Some(snap) = snapshot {
            HydratedAggregate {
                version: AggregateVersion::Version(snap.version),
                aggregate: Agg::restore(snap.snapshot),
                rehydrated_version: AggregateVersion::Version(snap.version),
            }
        } else {
            HydratedAggregate::default()
        }
    }
}

impl<Agg: SnapshotAggregate> HydratedAggregate<Agg> {
    fn to_snapshot(self) -> Option<VersionedSnapshot<Agg::Snapshot>> {
        if let AggregateVersion::Version(v) = self.version {
            Some(VersionedSnapshot {
                version: v,
                snapshot: self.aggregate.to_snapshot(),
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