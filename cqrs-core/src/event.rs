//! Event related definitions.

#![allow(clippy::module_name_repetitions)]

use std::{
    convert::{Infallible, TryFrom, TryInto as _},
    fmt,
    num::{NonZeroU128, NonZeroU8, TryFromIntError},
};

#[cfg(feature = "arrayvec")]
use arrayvec::{Array, ArrayVec};
use async_trait::async_trait;

use super::{Aggregate, LocalBoxTryStream, Version};

/// [Event Sourcing] event that describes something that has occurred (happened
/// fact).
///
/// A sequence of [`Event`]s may represent a concrete versioned state of an
/// [`Aggregate`]. The state is calculated by implementing [`EventSourced`] for
/// the desired [`Aggregate`] (or any other stateful entity).
///
/// [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html
pub trait Event {
    /// Returns type of this [`Event`].
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    fn event_type(&self) -> EventType;
}

/// State that can be calculated by applying specified [`Event`].
///
/// Usually, implemented by an [`Aggregate`].
pub trait EventSourced<Ev: Event + ?Sized> {
    /// Applies given [`Event`] to the current state.
    fn apply(&mut self, event: &Ev);
}

/// Different version of [`Event`] with the same [`EventType`].
///
/// The single type of [`Event`] may have different versions, which allows
/// evolving [`Event`] in the type. To overcome the necessity of dealing with
/// multiple types of the same [`Event`], it's recommended for the last actual
/// version of [`Event`] to implement trait [`From`] its previous versions, so
/// they can be automatically transformed into the latest actual version of
/// [`Event`].
pub trait VersionedEvent: Event {
    /// Returns [`Event`]'s version.
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    fn event_version(&self) -> &'static EventVersion;
}

/// Structured pair combining an [`Event`] and its [`EventNumber`].
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct NumberedEvent<Ev> {
    /// Number of the [`Event`].
    pub num: EventNumber,

    /// The [`Event`] itself.
    pub data: Ev,
}

/// A structured tuple combining an [`Event`], its [`EventNumber`] and
/// arbitrary metadata related to this [`Event`].
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct NumberedEventWithMeta<Ev, Mt> {
    /// Number of the [`Event`].
    pub num: EventNumber,

    /// The [`Event`] itself.
    pub data: Ev,

    /// Metadata related to the [`Event`].
    pub meta: Mt,
}

impl<'a, Ev> From<&'a NumberedEvent<Ev>> for NumberedEvent<&'a Ev> {
    #[inline]
    fn from(e: &'a NumberedEvent<Ev>) -> Self {
        Self {
            num: e.num,
            data: &e.data,
        }
    }
}

impl<'a, Ev, Mt> From<&'a NumberedEventWithMeta<Ev, Mt>> for NumberedEvent<&'a Ev> {
    #[inline]
    fn from(e: &'a NumberedEventWithMeta<Ev, Mt>) -> Self {
        Self {
            num: e.num,
            data: &e.data,
        }
    }
}

/// Source of reading all [`Event`]s belonging to some [`Aggregate`].
pub trait EventSource<Agg, Ev>
where
    Agg: Aggregate + EventSourced<Ev>,
    Ev: Event,
{
    /// Type of the error if reading [`NumberedEvent`]s fails.
    /// If it never fails, consider to specify [`Infallible`].
    type Err;

    /// Reads all stored [`Event`]s of a given [`Aggregate`].
    ///
    /// The returned [`Stream`] is finite and should end with the last stored
    /// [`Event`] of the [`Aggregate`].
    ///
    /// Only loads [`Event`]s after the [`EventNumber`] provided in `since`
    /// (see [`Since`] type for details).
    ///
    /// Any batching for loading should be handled on the implementation side
    /// if necessary.
    ///
    /// [`Stream`]: futures::Stream
    fn read_events(
        &self,
        id: &Agg::Id,
        since: Since,
    ) -> LocalBoxTryStream<'_, NumberedEvent<Ev>, Self::Err>;
}

/// Sink for persisting [`Event`]s belonging to some [`Aggregate`].
#[async_trait(?Send)]
pub trait EventSink<Agg, Ev, Mt>
where
    Agg: Aggregate + EventSourced<Ev>,
    Ev: Event,
    Mt: ?Sized,
{
    /// Type of the error if persisting [`Event`]s fails.
    /// If it never fails, consider to specify [`Infallible`].
    type Err;
    /// Type of returned [`NumberedEvent`]s which were persisted.
    // TODO: Try return NumberedEvent<&E>, to avoid unnecessary cloning in
    //       implementations, when Rust will support GATs, as at the moment
    //       lifetime parameter is required, and providing one complicates
    //       the whole framework code as HRTB conflicts with extracting
    //       associated types.
    type Ok: IntoIterator<Item = NumberedEvent<Ev>>;

    /// Persists given [`Event`]s with associated metadata and returns them
    /// as [`NumberedEvent`]s in the order they were persisted.
    ///
    /// The associated metadata is applied to all [`Event`]s in the given group.
    ///
    /// It's responsibility of the implementation to assign a correct
    /// [`EventNumber`] for each [`Event`].
    async fn append_events(
        &self,
        id: &Agg::Id,
        events: &[Ev],
        meta: &Mt,
    ) -> Result<Self::Ok, Self::Err>;
}

/// Type of an [`Event`].
pub type EventType = &'static str;

/// Representation of [`VersionedEvent`]'s version.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventVersion(NonZeroU8);

impl EventVersion {
    /// Attempts to create a new [`EventVersion`] from a given number.
    /// Will return [`None`] if the given number is `0`.
    #[inline]
    pub fn new<N: Into<u8>>(n: N) -> Option<Self> {
        Some(Self(NonZeroU8::new(n.into())?))
    }

    /// Creates new [`EventVersion`] from a raw [`u8`] value without
    /// checking it.
    ///
    /// # Safety
    ///
    /// The value must not be zero.
    #[allow(unsafe_code)]
    #[inline]
    pub const unsafe fn new_unchecked(n: u8) -> Self {
        // TODO: use safety guard for debug assertion
        Self(NonZeroU8::new_unchecked(n))
    }
}

impl fmt::Display for EventVersion {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

macro_rules! impl_from_event_version_for {
    ($t:ty) => {
        impl From<EventVersion> for $t {
            #[inline]
            fn from(v: EventVersion) -> Self {
                v.0.get().into()
            }
        }
    };
}
impl_from_event_version_for!(u8);
impl_from_event_version_for!(u16);
impl_from_event_version_for!(i16);
impl_from_event_version_for!(u32);
impl_from_event_version_for!(i32);
impl_from_event_version_for!(usize);
impl_from_event_version_for!(isize);
impl_from_event_version_for!(u64);
impl_from_event_version_for!(i64);
impl_from_event_version_for!(u128);
impl_from_event_version_for!(i128);

impl TryFrom<EventVersion> for i8 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(n: EventVersion) -> Result<Self, Self::Error> {
        n.0.get().try_into()
    }
}

/// Representation of [`Event`] sequence number, starting at 1.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventNumber(NonZeroU128);

impl EventNumber {
    /// Minimum possible [`EventNumber`].
    #[allow(unsafe_code)]
    pub const MIN_VALUE: Self =
        // One is absolutely non-zero, and this is required for this to be
        // usable in a `const` context.
        Self(unsafe {NonZeroU128::new_unchecked(1)});

    /// Attempts to create a new [`EventNumber`] from a given [`u128`] number.
    /// Returns [`None`] if the given number is `0`.
    #[inline]
    pub fn new<N: Into<u128>>(x: N) -> Option<Self> {
        Some(Self(NonZeroU128::new(x.into())?))
    }

    /// Increments [`EventNumber`] to the next value.
    #[inline]
    pub fn incr(&mut self) {
        self.0 = NonZeroU128::new(self.0.get() + 1).unwrap();
    }

    /// Gets the next [`EventNumber`] after the current one.
    #[inline]
    #[must_use]
    pub fn next(mut self) -> Self {
        self.0 = NonZeroU128::new(self.0.get() + 1).unwrap();
        self
    }
}

impl fmt::Display for EventNumber {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

macro_rules! impl_try_into_event_number_for {
    ($t:ty) => {
        impl TryFrom<$t> for EventNumber {
            type Error = TryIntoEventNumberError;

            #[inline]
            fn try_from(n: $t) -> Result<Self, Self::Error> {
                Self::new(u128::try_from(n)?).ok_or(TryIntoEventNumberError::Zero)
            }
        }
    };
}
impl_try_into_event_number_for!(u8);
impl_try_into_event_number_for!(i8);
impl_try_into_event_number_for!(u16);
impl_try_into_event_number_for!(i16);
impl_try_into_event_number_for!(u32);
impl_try_into_event_number_for!(i32);
impl_try_into_event_number_for!(u64);
impl_try_into_event_number_for!(i64);
impl_try_into_event_number_for!(u128);
impl_try_into_event_number_for!(i128);
impl_try_into_event_number_for!(usize);
impl_try_into_event_number_for!(isize);

/// Error of converting arbitrary number to [`EventNumber`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TryIntoEventNumberError {
    /// Zero number is not allowed.
    Zero,
    /// Converting to [`u128`] has failed.
    Conversion(TryFromIntError),
}

impl From<TryFromIntError> for TryIntoEventNumberError {
    #[inline]
    fn from(e: TryFromIntError) -> Self {
        TryIntoEventNumberError::Conversion(e)
    }
}

impl From<Infallible> for TryIntoEventNumberError {
    #[inline]
    fn from(x: Infallible) -> Self {
        match x {}
    }
}

impl fmt::Display for TryIntoEventNumberError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TryIntoEventNumberError::*;
        match self {
            Zero => f.write_str("zero value is forbidden"),
            Conversion(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl From<EventNumber> for u128 {
    #[inline]
    fn from(n: EventNumber) -> Self {
        n.0.get()
    }
}

macro_rules! impl_try_from_event_number_for {
    ($t:ty) => {
        impl TryFrom<EventNumber> for $t {
            type Error = TryFromIntError;

            #[inline]
            fn try_from(n: EventNumber) -> Result<Self, Self::Error> {
                n.0.get().try_into()
            }
        }
    };
}
impl_try_from_event_number_for!(u8);
impl_try_from_event_number_for!(i8);
impl_try_from_event_number_for!(u16);
impl_try_from_event_number_for!(i16);
impl_try_from_event_number_for!(u32);
impl_try_from_event_number_for!(i32);
impl_try_from_event_number_for!(u64);
impl_try_from_event_number_for!(i64);
impl_try_from_event_number_for!(i128);
impl_try_from_event_number_for!(usize);
impl_try_from_event_number_for!(isize);

/// Starting point for reading a stream of values from an [`EventSource`].
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum Since {
    /// Begins reading events from the very beginning.
    BeginningOfStream,
    /// Begins reading events after the given [`EventNumber`] (e.g. if the
    /// number was `4`, then reading should begin at number `5`).
    Event(EventNumber),
}

impl From<EventNumber> for Since {
    #[inline]
    fn from(n: EventNumber) -> Self {
        Since::Event(n)
    }
}

impl From<Version> for Since {
    #[inline]
    fn from(v: Version) -> Self {
        match v {
            Version::Initial => Since::BeginningOfStream,
            Version::Number(x) => Since::Event(x),
        }
    }
}

/// Conversion to a collection of [`Event`]s.
pub trait IntoEvents<Ev> {
    /// Type that represents a collection of [`Event`]s viewable as slice.
    type Iter: AsRef<[Ev]>;

    /// Performs the conversion into [`Event`]s collection.
    fn into_events(self) -> Self::Iter;
}

impl<Ev> IntoEvents<Ev> for () {
    type Iter = [Ev; 0];

    #[inline]
    fn into_events(self) -> Self::Iter {
        []
    }
}

impl<Ev, A> IntoEvents<Ev> for (A,)
where
    Ev: From<A>,
{
    type Iter = [Ev; 1];

    #[inline]
    fn into_events(self) -> Self::Iter {
        [self.0.into()]
    }
}

impl<Ev, A, B> IntoEvents<Ev> for (A, B)
where
    Ev: From<A> + From<B>,
{
    type Iter = [Ev; 2];

    #[inline]
    fn into_events(self) -> Self::Iter {
        [self.0.into(), self.1.into()]
    }
}

impl<Ev, A, B, C> IntoEvents<Ev> for (A, B, C)
where
    Ev: From<A> + From<B> + From<C>,
{
    type Iter = [Ev; 3];

    #[inline]
    fn into_events(self) -> Self::Iter {
        [self.0.into(), self.1.into(), self.2.into()]
    }
}

impl<Ev, A, B, C, D> IntoEvents<Ev> for (A, B, C, D)
where
    Ev: From<A> + From<B> + From<C> + From<D>,
{
    type Iter = [Ev; 4];

    #[inline]
    fn into_events(self) -> Self::Iter {
        [self.0.into(), self.1.into(), self.2.into(), self.3.into()]
    }
}

impl<Ev> IntoEvents<Ev> for Vec<Ev> {
    type Iter = Self;

    #[inline]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<Ev> IntoEvents<Ev> for [Ev; 0] {
    type Iter = Self;

    #[inline]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<Ev> IntoEvents<Ev> for [Ev; 1] {
    type Iter = Self;

    #[inline]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<Ev> IntoEvents<Ev> for [Ev; 2] {
    type Iter = Self;

    #[inline]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<Ev> IntoEvents<Ev> for [Ev; 3] {
    type Iter = Self;

    #[inline]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<Ev> IntoEvents<Ev> for [Ev; 4] {
    type Iter = Self;

    #[inline]
    fn into_events(self) -> Self::Iter {
        self
    }
}

#[cfg(feature = "arrayvec")]
impl<Ev, A: Array<Item = Ev>> IntoEvents<Ev> for ArrayVec<A> {
    type Iter = Self;

    #[inline]
    fn into_events(self) -> Self::Iter {
        self
    }
}

impl<Ev, T> IntoEvents<Ev> for AsEventsRef<T>
where
    T: AsRef<[Ev]>,
{
    type Iter = T;

    #[inline]
    fn into_events(self) -> Self::Iter {
        self.0
    }
}

/// Trivial and transparent wrapper-type that provides [`IntoEvents`]
/// implementation for types which implement `AsRef<[Event]>` already, but don't
/// implement [`IntoEvents`].
#[allow(missing_debug_implementations)]
pub struct AsEventsRef<T>(pub T);

impl<T> From<T> for AsEventsRef<T> {
    #[inline]
    fn from(v: T) -> Self {
        Self(v)
    }
}
