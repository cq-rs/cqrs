use cqrs::Event as _;
use cqrs_codegen::Event;

#[test]
fn derives_for_struct() {
    #[derive(Default, Event)]
    #[event(type = "test.event")]
    struct TestEvent {
        id: i32,
        data: String,
    };

    assert_eq!(TestEvent::EVENT_TYPE, "test.event");
    assert_eq!(TestEvent::default().event_type(), "test.event");
}

#[test]
fn derives_for_generic_struct() {
    #[derive(Default, Event)]
    #[event(type = "test.event.generic")]
    struct TestEventGeneric<ID, Data> {
        id: ID,
        data: Data,
    };

    type TestEvent = TestEventGeneric<i32, String>;

    assert_eq!(TestEvent::EVENT_TYPE, "test.event.generic");
    assert_eq!(TestEvent::default().event_type(), "test.event.generic");
}

#[test]
fn derives_for_enum() {
    #[derive(Default, Event)]
    #[event(type = "test.event.1")]
    struct TestEvent1;

    #[derive(Default, Event)]
    #[event(type = "test.event.2")]
    struct TestEvent2;

    #[derive(Event)]
    enum TestEvent {
        TestEventTuple(TestEvent1),
        TestEventStruct { event: TestEvent2 },
    }

    assert_eq!(
        TestEvent::TestEventTuple(Default::default()).event_type(),
        "test.event.1"
    );
    assert_eq!(
        TestEvent::TestEventStruct {
            event: Default::default()
        }
        .event_type(),
        "test.event.2"
    );
}

#[test]
fn derives_for_generic_enum() {
    #[derive(Default, Event)]
    #[event(type = "test.event.1")]
    struct TestEvent1;

    #[derive(Default, Event)]
    #[event(type = "test.event.2")]
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

    #[derive(Event)]
    enum TestEventGeneric<TE1, TE2, ID, Data> {
        TestEventTuple(TE1),
        TestEventStruct { event: TE2 },
        TestEventTupleGeneric(TestEventGeneric1<ID, Data>),
        TestEventStructGeneric { event: TestEventGeneric2<ID, Data> },
    }

    type TestEvent = TestEventGeneric<TestEvent1, TestEvent2, i32, String>;

    assert_eq!(
        TestEvent::TestEventTuple(Default::default()).event_type(),
        "test.event.1"
    );
    assert_eq!(
        TestEvent::TestEventStruct {
            event: Default::default()
        }
        .event_type(),
        "test.event.2"
    );
    assert_eq!(
        TestEvent::TestEventTupleGeneric(Default::default()).event_type(),
        "test.event.generic.1"
    );
    assert_eq!(
        TestEvent::TestEventStructGeneric {
            event: Default::default()
        }
        .event_type(),
        "test.event.generic.2"
    );
}
