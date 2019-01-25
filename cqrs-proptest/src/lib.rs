use cqrs_core::{Aggregate, CqrsError, Event, SerializableEvent, DeserializableEvent};
use proptest::prelude::*;
use std::fmt;

pub fn arb_events<E: Event + fmt::Debug>(event_strategy: impl Strategy<Value = E>, size: impl Into<prop::collection::SizeRange>) -> impl Strategy<Value = Vec<E>> {
    prop::collection::vec(event_strategy, size)
}

pub fn arb_aggregate<A: Aggregate + fmt::Debug>(events_strategy: impl Strategy<Value = Vec<A::Event>>) -> impl Strategy<Value = A> where A::Event: fmt::Debug {
    events_strategy.prop_map(|e| {
        let mut aggregate = A::default();
        for event in e {
            aggregate.apply(event);
        }
        aggregate
    })
}

pub fn roundtrip_through_serialization<E: SerializableEvent + DeserializableEvent + Eq + fmt::Debug>(original: &E) -> E {
    let mut buffer = Vec::default();
    original.serialize_event_to_buffer(&mut buffer).expect("serialization");

    let roundtrip = E::deserialize_event_from_buffer(&buffer, original.event_type()).expect("deserialization");
    roundtrip.expect("known event type")
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct AggregateFromEventSequence<A: Aggregate>(A);

impl<A: Aggregate> From<A> for AggregateFromEventSequence<A> {
    #[inline]
    fn from(aggregate: A) -> Self {
        AggregateFromEventSequence(aggregate)
    }
}

impl<A: Aggregate> AggregateFromEventSequence<A> {
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

        //any::<A::Event>()
//        Just(Self::default).boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}