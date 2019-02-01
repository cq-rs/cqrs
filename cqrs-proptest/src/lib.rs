//! # cqrs-proptest
//!
//! `cqrs-proptest` contains various utilities for building property tests with the
//! `proptest` crate and aggregates from `cqrs` or `cqrs-core`.
//!
//! ## Examples
//!
//! ```
//! use cqrs_core::{Aggregate, AggregateEvent, Event};
//! use cqrs_proptest::AggregateFromEventSequence;
//! use proptest::{prelude::*, strategy::{TupleUnion, ValueTree, W}, test_runner::TestRunner, prop_oneof};
//!
//! #[derive(Debug, Default)]
//! struct MyAggregate {
//!     active: bool
//! }
//!
//! impl Aggregate for MyAggregate {
//!     fn aggregate_type() -> &'static str {
//!         "my_aggregate"
//!     }
//! }
//!
//! #[derive(Clone, Copy, Debug)]
//! struct CreatedEvent{};
//!
//! impl Event for CreatedEvent {
//!     fn event_type(&self) -> &'static str {
//!         "created"
//!     }
//! }
//!
//! impl AggregateEvent<MyAggregate> for CreatedEvent {
//!     fn apply_to(self, aggregate: &mut MyAggregate) {
//!         aggregate.active = true;
//!     }
//! }
//!
//! #[derive(Clone, Copy, Debug)]
//! struct DeletedEvent{};
//!
//! impl Event for DeletedEvent {
//!     fn event_type(&self) -> &'static str {
//!         "deleted"
//!     }
//! }
//!
//! impl AggregateEvent<MyAggregate> for DeletedEvent {
//!     fn apply_to(self, aggregate: &mut MyAggregate) {
//!         aggregate.active = false;
//!     }
//! }
//!
//! #[derive(Clone, Copy, Debug)]
//! enum MyEvents {
//!     Created(CreatedEvent),
//!     Deleted(DeletedEvent),
//! }
//!
//! impl Event for MyEvents {
//!     fn event_type(&self) -> &'static str {
//!         match *self {
//!             MyEvents::Created(ref e) => e.event_type(),
//!             MyEvents::Deleted(ref e) => e.event_type(),
//!         }
//!     }
//! }
//!
//! impl AggregateEvent<MyAggregate> for MyEvents {
//!     fn apply_to(self, aggregate: &mut MyAggregate) {
//!         match self {
//!             MyEvents::Created(e) => e.apply_to(aggregate),
//!             MyEvents::Deleted(e) => e.apply_to(aggregate),
//!         }
//!     }
//! }
//!
//! impl Arbitrary for MyEvents {
//!     type Parameters = ();
//!
//!     fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
//!         prop_oneof![
//!             Just(MyEvents::Created(CreatedEvent{})),
//!             Just(MyEvents::Deleted(DeletedEvent{})),
//!         ]
//!     }
//!
//!     type Strategy = TupleUnion<(W<Just<Self>>, W<Just<Self>>)>;
//! }
//!
//! any::<AggregateFromEventSequence<MyAggregate, MyEvents>>()
//!     .new_tree(&mut TestRunner::default())
//!     .unwrap()
//!     .current()
//!     .into_aggregate();
//!
//! let parameters = (prop::collection::SizeRange::from(1..10), ());
//! any_with::<AggregateFromEventSequence<MyAggregate, MyEvents>>(parameters)
//!     .new_tree(&mut TestRunner::default())
//!     .unwrap()
//!     .current();
//! ```

#![warn(unused_import_braces, unused_imports, unused_qualifications)]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
    missing_docs
)]

use cqrs_core::{Aggregate, AggregateEvent, DeserializableEvent, Event, SerializableEvent};
use proptest::prelude::*;
use std::{fmt, marker::PhantomData};

/// Produces a strategy to generate an arbitrary vector of events, given a strategy
/// to generate an arbitrary event and a size range.
///
/// # Examples
///
/// ```
/// use cqrs_core::Event;
/// use cqrs_proptest::arb_events;
/// use proptest::{prelude::*, strategy::ValueTree, test_runner::TestRunner, prop_oneof};
///
/// #[derive(Clone, Copy, Debug)]
/// struct CreatedEvent{};
///
/// impl Event for CreatedEvent {
///     fn event_type(&self) -> &'static str {
///         "created"
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// struct DeletedEvent{};
///
/// impl Event for DeletedEvent {
///     fn event_type(&self) -> &'static str {
///         "deleted"
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// enum MyEvents {
///     Created(CreatedEvent),
///     Deleted(DeletedEvent),
/// }
///
/// impl Event for MyEvents {
///     fn event_type(&self) -> &'static str {
///         match *self {
///             MyEvents::Created(ref e) => e.event_type(),
///             MyEvents::Deleted(ref e) => e.event_type(),
///         }
///     }
/// }
///
/// fn arb_my_events() -> impl Strategy<Value = MyEvents> {
///     prop_oneof![
///         Just(MyEvents::Created(CreatedEvent{})),
///         Just(MyEvents::Deleted(DeletedEvent{})),
///     ]
/// }
///
/// arb_events(arb_my_events(), 0..10)
///     .new_tree(&mut TestRunner::default())
///     .unwrap()
///     .current();
/// ```
pub fn arb_events<E: Event + fmt::Debug>(
    event_strategy: impl Strategy<Value = E>,
    size: impl Into<prop::collection::SizeRange>,
) -> impl Strategy<Value = Vec<E>> {
    prop::collection::vec(event_strategy, size)
}

/// Produces a strategy to generate an arbitrary aggregate, given a strategy to generate
/// an arbitrary vector of events
///
/// # Examples
///
/// ```
/// use cqrs_core::{Aggregate, AggregateEvent, Event};
/// use cqrs_proptest::{arb_aggregate, arb_events};
/// use proptest::{prelude::*, strategy::{TupleUnion, ValueTree, W}, test_runner::TestRunner, prop_oneof};
///
/// #[derive(Debug, Default)]
/// struct MyAggregate {
///     active: bool
/// }
///
/// impl Aggregate for MyAggregate {
///     fn aggregate_type() -> &'static str {
///         "my_aggregate"
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// struct CreatedEvent{};
///
/// impl Event for CreatedEvent {
///     fn event_type(&self) -> &'static str {
///         "created"
///     }
/// }
///
/// impl AggregateEvent<MyAggregate> for CreatedEvent {
///     fn apply_to(self, aggregate: &mut MyAggregate) {
///         aggregate.active = true;
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// struct DeletedEvent{};
///
/// impl Event for DeletedEvent {
///     fn event_type(&self) -> &'static str {
///         "deleted"
///     }
/// }
///
/// impl AggregateEvent<MyAggregate> for DeletedEvent {
///     fn apply_to(self, aggregate: &mut MyAggregate) {
///         aggregate.active = false;
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// enum MyEvents {
///     Created(CreatedEvent),
///     Deleted(DeletedEvent),
/// }
///
/// impl Event for MyEvents {
///     fn event_type(&self) -> &'static str {
///         match *self {
///             MyEvents::Created(ref e) => e.event_type(),
///             MyEvents::Deleted(ref e) => e.event_type(),
///         }
///     }
/// }
///
/// impl AggregateEvent<MyAggregate> for MyEvents {
///     fn apply_to(self, aggregate: &mut MyAggregate) {
///         match self {
///             MyEvents::Created(e) => e.apply_to(aggregate),
///             MyEvents::Deleted(e) => e.apply_to(aggregate),
///         }
///     }
/// }
///
/// impl Arbitrary for MyEvents {
///     type Parameters = ();
///
///     fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
///         prop_oneof![
///             Just(MyEvents::Created(CreatedEvent{})),
///             Just(MyEvents::Deleted(DeletedEvent{})),
///         ]
///     }
///
///     type Strategy = TupleUnion<(W<Just<Self>>, W<Just<Self>>)>;
/// }
///
/// arb_aggregate::<MyAggregate, MyEvents, _>(arb_events(any::<MyEvents>(), 0..10))
///     .new_tree(&mut TestRunner::default())
///     .unwrap()
///     .current();
/// ```
pub fn arb_aggregate<A, E, S>(events_strategy: S) -> impl Strategy<Value = A>
where
    A: Aggregate + fmt::Debug,
    E: AggregateEvent<A> + fmt::Debug,
    S: Strategy<Value = Vec<E>>,
{
    events_strategy.prop_map(|e| {
        let mut aggregate = A::default();
        for event in e {
            aggregate.apply(event);
        }
        aggregate
    })
}

/// Given a serializable event, constructs a buffer, serializes the event to the buffer, and then
/// deserializes the event, returning the deserialized value.
///
/// # Examples
///
/// ```
/// use cqrs_core::{Event, SerializableEvent, DeserializableEvent};
/// use cqrs_proptest::roundtrip_through_serialization;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// struct CreatedEvent{};
///
/// impl Event for CreatedEvent {
///     fn event_type(&self) -> &'static str {
///         "created"
///     }
/// }
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// struct DeletedEvent{};
///
/// impl Event for DeletedEvent {
///     fn event_type(&self) -> &'static str {
///         "deleted"
///     }
/// }
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// enum MyEvents {
///     Created(CreatedEvent),
///     Deleted(DeletedEvent),
/// }
///
/// impl Event for MyEvents {
///     fn event_type(&self) -> &'static str {
///         match *self {
///             MyEvents::Created(ref e) => e.event_type(),
///             MyEvents::Deleted(ref e) => e.event_type(),
///         }
///     }
/// }
///
/// impl SerializableEvent for MyEvents {
///     type Error = serde_json::Error;
///
///     fn serialize_event_to_buffer(&self, buffer: &mut Vec<u8>) -> Result<(), Self::Error> {
///         match *self {
///             MyEvents::Created(ref e) => serde_json::to_writer(buffer, e),
///             MyEvents::Deleted(ref e) => serde_json::to_writer(buffer, e),
///         }
///     }
/// }
///
/// impl DeserializableEvent for MyEvents {
///     type Error = serde_json::Error;
///
///     fn deserialize_event_from_buffer(buffer: &[u8], event_type: &str) -> Result<Option<Self>, Self::Error> {
///         match event_type {
///             "created" => serde_json::from_reader(buffer).map(MyEvents::Created).map(Some),
///             "deleted" => serde_json::from_reader(buffer).map(MyEvents::Deleted).map(Some),
///             _ => Ok(None),
///         }
///     }
/// }
///
/// let original = MyEvents::Created(CreatedEvent{});
/// let roundtrip = roundtrip_through_serialization(&original);
/// assert_eq!(original, roundtrip);
/// ```
pub fn roundtrip_through_serialization<E: SerializableEvent + DeserializableEvent>(
    original: &E,
) -> E {
    let mut buffer = Vec::default();
    original
        .serialize_event_to_buffer(&mut buffer)
        .expect("serialization");

    let roundtrip =
        E::deserialize_event_from_buffer(&buffer, original.event_type()).expect("deserialization");
    roundtrip.expect("known event type")
}

/// A wrapper for an aggregate that was generated from an arbitrary sequence of events.
///
/// # Examples
///
/// ```
/// use cqrs_core::{Aggregate, AggregateEvent, Event};
/// use cqrs_proptest::AggregateFromEventSequence;
/// use proptest::{prelude::*, strategy::{TupleUnion, ValueTree, W}, test_runner::TestRunner, prop_oneof};
///
/// #[derive(Debug, Default)]
/// struct MyAggregate {
///     active: bool
/// }
///
/// impl Aggregate for MyAggregate {
///     fn aggregate_type() -> &'static str {
///         "my_aggregate"
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// struct CreatedEvent{};
///
/// impl Event for CreatedEvent {
///     fn event_type(&self) -> &'static str {
///         "created"
///     }
/// }
///
/// impl AggregateEvent<MyAggregate> for CreatedEvent {
///     fn apply_to(self, aggregate: &mut MyAggregate) {
///         aggregate.active = true;
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// struct DeletedEvent{};
///
/// impl Event for DeletedEvent {
///     fn event_type(&self) -> &'static str {
///         "deleted"
///     }
/// }
///
/// impl AggregateEvent<MyAggregate> for DeletedEvent {
///     fn apply_to(self, aggregate: &mut MyAggregate) {
///         aggregate.active = false;
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// enum MyEvents {
///     Created(CreatedEvent),
///     Deleted(DeletedEvent),
/// }
///
/// impl Event for MyEvents {
///     fn event_type(&self) -> &'static str {
///         match *self {
///             MyEvents::Created(ref e) => e.event_type(),
///             MyEvents::Deleted(ref e) => e.event_type(),
///         }
///     }
/// }
///
/// impl AggregateEvent<MyAggregate> for MyEvents {
///     fn apply_to(self, aggregate: &mut MyAggregate) {
///         match self {
///             MyEvents::Created(e) => e.apply_to(aggregate),
///             MyEvents::Deleted(e) => e.apply_to(aggregate),
///         }
///     }
/// }
///
/// impl Arbitrary for MyEvents {
///     type Parameters = ();
///
///     fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
///         prop_oneof![
///             Just(MyEvents::Created(CreatedEvent{})),
///             Just(MyEvents::Deleted(DeletedEvent{})),
///         ]
///     }
///
///     type Strategy = TupleUnion<(W<Just<Self>>, W<Just<Self>>)>;
/// }
///
/// any::<AggregateFromEventSequence<MyAggregate, MyEvents>>()
///     .new_tree(&mut TestRunner::default())
///     .unwrap()
///     .current()
///     .into_aggregate();
///
/// let parameters = (prop::collection::SizeRange::from(1..10), ());
/// any_with::<AggregateFromEventSequence<MyAggregate, MyEvents>>(parameters)
///     .new_tree(&mut TestRunner::default())
///     .unwrap()
///     .current();
/// ```
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq)]
pub struct AggregateFromEventSequence<A, E>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    aggregate: A,
    _phantom: PhantomData<*const E>,
}

impl<A, E> fmt::Debug for AggregateFromEventSequence<A, E>
where
    A: Aggregate + fmt::Debug,
    E: AggregateEvent<A>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("AggregateFromEventSequence")
            .field(&self.aggregate)
            .finish()
    }
}

impl<A, E> From<A> for AggregateFromEventSequence<A, E>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    #[inline]
    fn from(aggregate: A) -> Self {
        AggregateFromEventSequence {
            aggregate,
            _phantom: PhantomData,
        }
    }
}

impl<A, E> AggregateFromEventSequence<A, E>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    /// Unwraps the generated aggregate.
    #[inline]
    pub fn into_aggregate(self) -> A {
        self.aggregate
    }
}

impl<A, E> Arbitrary for AggregateFromEventSequence<A, E>
where
    A: Aggregate + fmt::Debug,
    E: AggregateEvent<A> + Arbitrary + 'static,
{
    type Parameters = (prop::collection::SizeRange, <E as Arbitrary>::Parameters);
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        any_with::<Vec<E>>(args)
            .prop_map(|events| {
                let mut aggregate = A::default();
                for event in events {
                    aggregate.apply(event);
                }
                AggregateFromEventSequence {
                    aggregate,
                    _phantom: PhantomData,
                }
            })
            .boxed()
    }
}
