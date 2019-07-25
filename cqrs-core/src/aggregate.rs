//! Aggregate related definitions.

#![allow(clippy::module_name_repetitions)]

use std::{
    borrow::Borrow,
    convert::{TryFrom, TryInto as _},
    fmt,
    num::TryFromIntError,
    ops,
};

use async_trait::async_trait;

use super::{Event, EventNumber, EventSourced, NumberedEvent};

/// [DDD aggregate] that represents an isolated tree of entities, is
/// capable of handling [`Command`]s and is always kept in a consistent state.
///
/// In case [`Aggregate`] is [`EventSourced`] we assume that [`Aggregate`]
/// exists if at least one [`Event`] exists for it, so [`Aggregate`] is usable
/// and distinguishable only after at least one [`Event`] is applied to its
/// initial state.
///
/// [DDD aggregate]: https://martinfowler.com/bliki/DDD_Aggregate.html
/// [`Command`]: super::Command
pub trait Aggregate: Default {
    /// Type of [`Aggregate`]'s unique identifier (ID).
    type Id;

    /// Returns static string representing the type of this [`Aggregate`].
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    fn aggregate_type() -> &'static str;

    /// Returns unique ID of this [`Aggregate`].
    fn id(&self) -> &Self::Id;
}

/// Source for loading snapshots of some [`Aggregate`].
#[async_trait]
pub trait SnapshotSource<A: Aggregate> {
    /// Type of the shapshot loading error.
    /// If it never fails, consider to specify [`Infallible`].
    ///
    /// [`Infallible`]: std::convert::Infallible
    type Err;

    /// Loads latest stored snapshot of a given [`Aggregate`].
    async fn load_snapshot(&self, id: &A::Id) -> Result<Option<(A, Version)>, Self::Err>;
}

/// Sink for persisting snapshots of some [`Aggregate`].
#[async_trait]
pub trait SnapshotSink<A: Aggregate> {
    /// Type of the shapshot persisting error.
    /// If it never fails, consider to specify [`Infallible`].
    ///
    /// [`Infallible`]: std::convert::Infallible
    type Err;

    /// Persists [`Aggregate`]'s snapshot of a given [`Version`].
    async fn persist_snapshot(
        &self,
        id: &A::Id,
        aggregate: &A,
        version: Version,
    ) -> Result<(), Self::Err>;
}

/// [`Aggregate`] that is [`EventSourced`] and keeps track of the version of its
/// last snapshot and the current version.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct HydratedAggregate<A> {
    /// Current [`Version`] of this [`Aggregate`].
    ver: Version,
    /// [`Version`] of last snapshot of this [`Aggregate`].
    snapshot_ver: Option<Version>,
    /// The [`Aggregate`] itself.
    state: A,
}

impl<A> HydratedAggregate<A> {
    /// Creates new [`HydratedAggregate`] from a given [`Aggregate`] and its
    /// current [`Version`].
    #[inline]
    pub fn from_version(agg: A, ver: Version) -> Self {
        Self {
            ver,
            snapshot_ver: None,
            state: agg,
        }
    }
    /// Creates new [`HydratedAggregate`] from a given [`Aggregate`] and its
    /// latest snapshot [`Version`].
    #[inline]
    pub fn from_snapshot(agg: A, ver: Version) -> Self {
        Self {
            ver,
            snapshot_ver: Some(ver),
            state: agg,
        }
    }

    /// Returns ID of this [`Aggregate`].
    #[inline(always)]
    pub fn id(&self) -> &A::Id
    where
        A: Aggregate,
    {
        self.state.id()
    }

    /// Returns the current [`Version`] of this [`Aggregate`].
    #[inline(always)]
    pub fn version(&self) -> Version {
        self.ver
    }

    /// Returns [`Version`] of the latest snapshot for this [`Aggregate`].
    #[inline(always)]
    pub fn snapshot_version(&self) -> Option<Version> {
        self.snapshot_ver
    }

    /// Sets [`Version`] of the latest snapshot for this [`Aggregate`].
    #[inline(always)]
    pub fn set_snapshot_version(&mut self, new: Version) {
        self.snapshot_ver = Some(new);
    }

    /// Returns the inner [`Aggregate`] itself.
    #[inline(always)]
    pub fn state(&self) -> &A {
        &self.state
    }

    /// Applies given [`NumberedEvent`] to this [`Aggregate`] renewing its
    /// [`Version`] with [`EventNumber`] of the [`NumberedEvent`].
    #[inline]
    pub fn apply<'a, E, R>(&mut self, event: R)
    where
        R: Into<NumberedEvent<&'a E>>,
        E: Event + 'a,
        A: EventSourced<E>,
    {
        let e = event.into();
        self.state.apply_event(e.data);
        self.ver = e.num.into();
    }

    /// Applies given [`NumberedEvent`]s to this [`Aggregate`] renewing its
    /// [`Version`] appropriately.
    #[inline]
    pub fn apply_events<'a, E, R, I>(&mut self, events: I)
    where
        I: IntoIterator<Item = R>,
        R: Into<NumberedEvent<&'a E>>,
        E: Event + 'a,
        A: EventSourced<E>,
    {
        for event in events {
            self.apply(event);
        }
    }
}

impl<A> AsRef<A> for HydratedAggregate<A> {
    #[inline]
    fn as_ref(&self) -> &A {
        &self.state
    }
}

impl<A> Borrow<A> for HydratedAggregate<A> {
    #[inline]
    fn borrow(&self) -> &A {
        &self.state
    }
}

/// Version of an [`Aggregate`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Version {
    /// Version has no [`Event`]s applied to it.
    Initial,
    /// Version of the last [`Event`] applied to an [`Aggregate`].
    Number(EventNumber),
}

impl Default for Version {
    #[inline]
    fn default() -> Self {
        Version::Initial
    }
}

impl Version {
    /// Creates new [`Version`] from a number.
    ///
    /// The number `0` gets interpreted as being [`Version::Initial`], while any
    /// other number is interpreted as the latest [`EventNumber`] applied.
    #[inline]
    pub fn new<N: Into<u128>>(number: N) -> Self {
        EventNumber::new(number).map_or(Version::Initial, Version::Number)
    }

    /// Increments [`Version`] number to the next in sequence.
    #[inline]
    pub fn incr(&mut self) {
        match *self {
            Version::Initial => *self = Version::Number(EventNumber::MIN_VALUE),
            Version::Number(ref mut en) => en.incr(),
        }
    }

    /// Returns next [`EventNumber`] in a sequence.
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

    /// Returns [`Version`] number as [`EventNumber`], returning [`None`] if the
    /// current [`Version`] is [`Version::Initial`].
    #[inline]
    pub fn event_number(self) -> Option<EventNumber> {
        match self {
            Version::Initial => None,
            Version::Number(en) => Some(en),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Version::Initial => f.write_str("initial"),
            Version::Number(ref event_number) => event_number.fmt(f),
        }
    }
}

impl From<EventNumber> for Version {
    #[inline]
    fn from(n: EventNumber) -> Self {
        Version::Number(n)
    }
}

macro_rules! impl_into_version_for {
    ($t:ty) => {
        impl From<$t> for Version {
            #[inline]
            fn from(n: $t) -> Self {
                Self::new(n)
            }
        }
    };
}
impl_into_version_for!(u8);
impl_into_version_for!(u16);
impl_into_version_for!(u32);
impl_into_version_for!(u64);
impl_into_version_for!(u128);

macro_rules! impl_try_into_version_for {
    ($t:ty) => {
        impl TryFrom<$t> for Version {
            type Error = TryFromIntError;

            #[inline]
            fn try_from(n: $t) -> Result<Self, Self::Error> {
                match n {
                    0 => Ok(Version::Initial),
                    _ => n.try_into(),
                }
            }
        }
    };
}
impl_try_into_version_for!(i8);
impl_try_into_version_for!(i16);
impl_try_into_version_for!(i32);
impl_try_into_version_for!(i64);
impl_try_into_version_for!(i128);
impl_try_into_version_for!(usize);
impl_try_into_version_for!(isize);

impl From<Version> for u128 {
    #[inline]
    fn from(v: Version) -> Self {
        match v {
            Version::Initial => 0,
            Version::Number(n) => n.into(),
        }
    }
}

macro_rules! impl_try_from_version_for {
    ($t:ty) => {
        impl TryFrom<Version> for $t {
            type Error = TryFromIntError;

            #[inline]
            fn try_from(v: Version) -> Result<Self, Self::Error> {
                match v {
                    Version::Initial => Ok(0),
                    Version::Number(n) => n.try_into(),
                }
            }
        }
    };
}
impl_try_from_version_for!(u8);
impl_try_from_version_for!(i8);
impl_try_from_version_for!(u16);
impl_try_from_version_for!(i16);
impl_try_from_version_for!(u32);
impl_try_from_version_for!(i32);
impl_try_from_version_for!(u64);
impl_try_from_version_for!(i64);
impl_try_from_version_for!(i128);
impl_try_from_version_for!(usize);
impl_try_from_version_for!(isize);

impl ops::Sub for Version {
    type Output = i128;

    fn sub(self, rhs: Self) -> Self::Output {
        i128::try_from(self).unwrap() - i128::try_from(rhs).unwrap()
    }
}

/// Recommendation on whether or not a snapshot of an [`Aggregate`] should be
/// persisted.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SnapshotRecommendation {
    /// Snapshot should be taken.
    ShouldSnapshot,
    /// Snapshot should not be taken.
    DoNotSnapshot,
}

/// Strategy determining when a snapshot of an [`Aggregate`] should be taken.
pub trait SnapshotStrategy {
    /// Gives the [`SnapshotRecommendation`] on whether or not to perform
    /// a snapshot for an [`Aggregate`].
    fn snapshot_recommendation(
        &self,
        ver: Version,
        last_snapshot_ver: Option<Version>,
    ) -> SnapshotRecommendation;
}

/// [`SnapshotStrategy`] that will never recommend taking a snapshot.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct NeverSnapshot;

impl SnapshotStrategy for NeverSnapshot {
    /// Always returns [`SnapshotRecommendation::DoNotSnapshot`].
    #[inline]
    fn snapshot_recommendation(&self, _: Version, _: Option<Version>) -> SnapshotRecommendation {
        SnapshotRecommendation::DoNotSnapshot
    }
}

/// [`SnapshotStrategy`] that will always recommend taking a snapshot.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct AlwaysSnapshot;

impl SnapshotStrategy for AlwaysSnapshot {
    /// Always returns [`SnapshotRecommendation::ShouldSnapshot`].
    #[inline]
    fn snapshot_recommendation(&self, _: Version, _: Option<Version>) -> SnapshotRecommendation {
        SnapshotRecommendation::ShouldSnapshot
    }
}
