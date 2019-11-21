#![allow(dead_code)]

use cqrs::AggregateEvent as _;
use cqrs_codegen::{Aggregate, AggregateEvent, Event};

#[derive(Aggregate, Default)]
#[aggregate(type = "aggregate")]
struct Aggregate {
    id: i32,
}

#[test]
fn derives_for_enum() {
    #[derive(Default, Event)]
    #[event(type = "test.event.1")]
    struct TestEvent1;

    #[derive(Default, Event)]
    #[event(type = "test.event.2")]
    struct TestEvent2;

    #[derive(Event, AggregateEvent)]
    #[event(aggregate = "Aggregate")]
    enum TestEvent {
        TestEventTuple(TestEvent1),
        TestEventStruct { event: TestEvent2 },
    }

    assert_eq!(
        TestEvent::EVENT_TYPES,
        [TestEvent1::EVENT_TYPE, TestEvent2::EVENT_TYPE],
    );
    assert_eq!(
        TestEvent::event_types(),
        [TestEvent1::EVENT_TYPE, TestEvent2::EVENT_TYPE],
    );
}

#[test]
fn derives_for_generic_enum() {
    #[derive(Default, Event)]
    #[event(type = "test.event", version = 1)]
    struct TestEvent1;

    #[derive(Default, Event)]
    #[event(type = "test.event", version = 2)]
    struct TestEvent2;

    #[derive(Default, Event)]
    #[event(type = "test.event.generic.1")]
    struct TestEventGeneric1<ID, Data> {
        id: ID,
        data: Data,
    }

    #[derive(Default, Event)]
    #[event(type = "test.event.generic.2")]
    struct TestEventGeneric2<ID, Data> {
        id: ID,
        data: Data,
    }

    #[derive(AggregateEvent, Event)]
    #[event(aggregate = "Aggregate")]
    enum TestEventGeneric<ID, Data> {
        TestEventTuple(TestEvent1),
        TestEventStruct { event: TestEvent2 },
        TestEventTupleGeneric(TestEventGeneric1<ID, Data>),
        TestEventStructGeneric { event: TestEventGeneric2<ID, Data> },
    }

    type TestEvent = TestEventGeneric<i32, String>;

    assert_eq!(
        TestEvent::EVENT_TYPES,
        [
            TestEvent1::EVENT_TYPE,
            TestEvent2::EVENT_TYPE,
            TestEventGeneric1::<i32, String>::EVENT_TYPE,
            TestEventGeneric2::<i32, String>::EVENT_TYPE,
        ],
    );
    assert_eq!(
        TestEvent::event_types(),
        [
            TestEvent1::EVENT_TYPE,
            TestEvent2::EVENT_TYPE,
            TestEventGeneric1::<i32, String>::EVENT_TYPE,
            TestEventGeneric2::<i32, String>::EVENT_TYPE,
        ],
    );
}
