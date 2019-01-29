//! # cqrs-proptest
//!
//! `cqrs-proptest` contains various utilities for building property tests with the
//! `proptest` crate and aggregates from `cqrs` or `cqrs-core`.
//!
//! ## Examples
//!
//! ```
//! use cqrs_core::{Aggregate, Event};
//! use cqrs_proptest::AggregateFromEventSequence;
//! use proptest::{prelude::*, strategy::{TupleUnion, ValueTree, W}, test_runner::TestRunner, prop_oneof};
//!
//! #[derive(Debug, Default)]
//! struct MyAggregate {
//!     active: bool
//! }
//!
//! impl Aggregate for MyAggregate {
//!     type Event = MyEvent;
//!     type Events = Vec<MyEvent>;
//!     type Command = ();
//!     type Error = String;
//!
//!     fn entity_type() -> &'static str {
//!         "my_aggregate"
//!     }
//!
//!     fn apply(&mut self, event: Self::Event) {
//!         match event {
//!             MyEvent::Created => self.active = true,
//!             MyEvent::Deleted => self.active = false,
//!         }
//!     }
//!
//!     fn execute(&self, _: Self::Command) -> Result<Self::Events, Self::Error> {
//!         Ok(Vec::default())
//!     }
//! }
//!
//! #[derive(Clone, Copy, Debug)]
//! enum MyEvent {
//!     Created,
//!     Deleted,
//! }
//!
//! impl Event for MyEvent {
//!     fn event_type(&self) -> &'static str {
//!         match *self {
//!             MyEvent::Created => "created",
//!             MyEvent::Deleted => "deleted",
//!         }
//!     }
//! }
//!
//! impl Arbitrary for MyEvent {
//!     type Parameters = ();
//!
//!     fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
//!         prop_oneof![
//!             Just(MyEvent::Created),
//!             Just(MyEvent::Deleted),
//!         ]
//!     }
//!
//!     type Strategy = TupleUnion<(W<Just<Self>>, W<Just<Self>>)>;
//! }
//!
//! any::<AggregateFromEventSequence<MyAggregate>>()
//!     .new_tree(&mut TestRunner::default())
//!     .unwrap()
//!     .current()
//!     .into_aggregate();
//!
//! let parameters = (prop::collection::SizeRange::from(1..10), ());
//! any_with::<AggregateFromEventSequence<MyAggregate>>(parameters)
//!     .new_tree(&mut TestRunner::default())
//!     .unwrap()
//!     .current();
//! ```

#![warn(
    unused_import_braces,
    unused_imports,
    unused_qualifications,
)]

#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
    missing_docs,
)]

use cqrs_core::{Aggregate, CqrsError, Event, SerializableEvent, DeserializableEvent};
use proptest::prelude::*;
use std::fmt;

/// Produces a strategy to generate an arbitrary vector of events, given a strategy to generate an arbitrary event and a size range.
///
/// # Examples
///
/// ```
/// use cqrs_core::Event;
/// use cqrs_proptest::arb_events;
/// use proptest::{prelude::*, strategy::ValueTree, test_runner::TestRunner, prop_oneof};
///
/// #[derive(Clone, Copy, Debug)]
/// enum MyEvent {
///     Created,
///     Deleted,
/// }
///
/// impl Event for MyEvent {
///     fn event_type(&self) -> &'static str {
///         match *self {
///             MyEvent::Created => "created",
///             MyEvent::Deleted => "deleted",
///         }
///     }
/// }
///
/// fn arb_my_event() -> impl Strategy<Value = MyEvent> {
///     prop_oneof![
///         Just(MyEvent::Created),
///         Just(MyEvent::Deleted),
///     ]
/// }
///
/// arb_events(arb_my_event(), 0..10)
///     .new_tree(&mut TestRunner::default())
///     .unwrap()
///     .current();
/// ```
pub fn arb_events<E: Event + fmt::Debug>(event_strategy: impl Strategy<Value = E>, size: impl Into<prop::collection::SizeRange>) -> impl Strategy<Value = Vec<E>> {
    prop::collection::vec(event_strategy, size)
}

/// Produces a strategy to generate an arbitrary aggregate, given a strategy to generate an arbitrary vector of events
///
/// # Examples
///
/// ```
/// use cqrs_core::{Aggregate, Event};
/// use cqrs_proptest::{arb_aggregate, arb_events};
/// use proptest::{prelude::*, strategy::{TupleUnion, ValueTree, W}, test_runner::TestRunner, prop_oneof};
///
/// #[derive(Debug, Default)]
/// struct MyAggregate {
///     active: bool
/// }
///
/// impl Aggregate for MyAggregate {
///     type Event = MyEvent;
///     type Events = Vec<MyEvent>;
///     type Command = ();
///     type Error = String;
///
///     fn entity_type() -> &'static str {
///         "my_aggregate"
///     }
///
///     fn apply(&mut self, event: Self::Event) {
///         match event {
///             MyEvent::Created => self.active = true,
///             MyEvent::Deleted => self.active = false,
///         }
///     }
///
///     fn execute(&self, _: Self::Command) -> Result<Self::Events, Self::Error> {
///         Ok(Vec::default())
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// enum MyEvent {
///     Created,
///     Deleted,
/// }
///
/// impl Event for MyEvent {
///     fn event_type(&self) -> &'static str {
///         match *self {
///             MyEvent::Created => "created",
///             MyEvent::Deleted => "deleted",
///         }
///     }
/// }
///
/// impl Arbitrary for MyEvent {
///     type Parameters = ();
///
///     fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
///         prop_oneof![
///             Just(MyEvent::Created),
///             Just(MyEvent::Deleted),
///         ]
///     }
///
///     type Strategy = TupleUnion<(W<Just<Self>>, W<Just<Self>>)>;
/// }
///
/// arb_aggregate::<MyAggregate, _>(arb_events(any::<MyEvent>(), 0..10))
///     .new_tree(&mut TestRunner::default())
///     .unwrap()
///     .current();
/// ```
pub fn arb_aggregate<A: Aggregate + fmt::Debug, S: Strategy<Value = Vec<A::Event>>>(events_strategy: S) -> impl Strategy<Value = A>
where
    A::Event: fmt::Debug,
{
    events_strategy.prop_map(|e| {
        let mut aggregate = A::default();
        for event in e {
            aggregate.apply(event);
        }
        aggregate
    })
}

/// Given a serializable event, constructs a buffer, serializes the event to the buffer, and then deserializes the event, returning the deserialized value.
///
/// # Examples
///
/// ```
/// use cqrs_core::{Event, SerializableEvent, DeserializableEvent};
/// use cqrs_proptest::roundtrip_through_serialization;
///
/// #[derive(Debug, PartialEq)]
/// enum MyEvent {
///     Created,
///     Deleted,
/// }
///
/// impl Event for MyEvent {
///     fn event_type(&self) -> &'static str {
///         match *self {
///             MyEvent::Created => "created",
///             MyEvent::Deleted => "deleted",
///         }
///     }
/// }
///
/// impl SerializableEvent for MyEvent {
///     type Error = String;
///
///     fn serialize_event_to_buffer(&self, buffer: &mut Vec<u8>) -> Result<(), Self::Error> {
///         Ok(())
///     }
/// }
///
/// impl DeserializableEvent for MyEvent {
///     type Error = String;
///
///     fn deserialize_event_from_buffer(buffer: &[u8], event_type: &str) -> Result<Option<Self>, Self::Error> {
///         match event_type {
///             "created" => Ok(Some(MyEvent::Created)),
///             "deleted" => Ok(Some(MyEvent::Deleted)),
///             _ => Ok(None),
///         }
///     }
/// }
///
/// let original = MyEvent::Created;
/// let roundtrip = roundtrip_through_serialization(&original);
/// assert_eq!(original, roundtrip);
/// ```
pub fn roundtrip_through_serialization<E: SerializableEvent + DeserializableEvent>(original: &E) -> E {
    let mut buffer = Vec::default();
    original.serialize_event_to_buffer(&mut buffer).expect("serialization");

    let roundtrip = E::deserialize_event_from_buffer(&buffer, original.event_type()).expect("deserialization");
    roundtrip.expect("known event type")
}

/// A wrapper for an aggregate that was generated from an arbitrary sequence of events.
///
/// # Examples
///
/// ```
/// use cqrs_core::{Aggregate, Event};
/// use cqrs_proptest::AggregateFromEventSequence;
/// use proptest::{prelude::*, strategy::{TupleUnion, ValueTree, W}, test_runner::TestRunner, prop_oneof};
///
/// #[derive(Debug, Default)]
/// struct MyAggregate {
///     active: bool
/// }
///
/// impl Aggregate for MyAggregate {
///     type Event = MyEvent;
///     type Events = Vec<MyEvent>;
///     type Command = ();
///     type Error = String;
///
///     fn entity_type() -> &'static str {
///         "my_aggregate"
///     }
///
///     fn apply(&mut self, event: Self::Event) {
///         match event {
///             MyEvent::Created => self.active = true,
///             MyEvent::Deleted => self.active = false,
///         }
///     }
///
///     fn execute(&self, _: Self::Command) -> Result<Self::Events, Self::Error> {
///         Ok(Vec::default())
///     }
/// }
///
/// #[derive(Clone, Copy, Debug)]
/// enum MyEvent {
///     Created,
///     Deleted,
/// }
///
/// impl Event for MyEvent {
///     fn event_type(&self) -> &'static str {
///         match *self {
///             MyEvent::Created => "created",
///             MyEvent::Deleted => "deleted",
///         }
///     }
/// }
///
/// impl Arbitrary for MyEvent {
///     type Parameters = ();
///
///     fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
///         prop_oneof![
///             Just(MyEvent::Created),
///             Just(MyEvent::Deleted),
///         ]
///     }
///
///     type Strategy = TupleUnion<(W<Just<Self>>, W<Just<Self>>)>;
/// }
///
/// any::<AggregateFromEventSequence<MyAggregate>>()
///     .new_tree(&mut TestRunner::default())
///     .unwrap()
///     .current()
///     .into_aggregate();
///
/// let parameters = (prop::collection::SizeRange::from(1..10), ());
/// any_with::<AggregateFromEventSequence<MyAggregate>>(parameters)
///     .new_tree(&mut TestRunner::default())
///     .unwrap()
///     .current();
/// ```
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct AggregateFromEventSequence<A: Aggregate>(A);

impl<A: Aggregate> From<A> for AggregateFromEventSequence<A> {
    #[inline]
    fn from(aggregate: A) -> Self {
        AggregateFromEventSequence(aggregate)
    }
}

impl<A: Aggregate> AggregateFromEventSequence<A> {
    /// Unwraps the generated aggregate.
    #[inline]
    pub fn into_aggregate(self) -> A {
        self.0
    }
}

impl<A> Arbitrary for AggregateFromEventSequence<A>
where
    A: Aggregate + fmt::Debug,
    A::Error: CqrsError,
    A::Event: Arbitrary + 'static,
    Self: fmt::Debug,
{
    type Parameters = (prop::collection::SizeRange, <A::Event as Arbitrary>::Parameters);

    fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
        any_with::<Vec<A::Event>>(args).prop_map(|events| {
            let mut aggregate = A::default();
            for event in events {
                aggregate.apply(event);
            }
            AggregateFromEventSequence(aggregate)
        }).boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}