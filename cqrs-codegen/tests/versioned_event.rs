#![allow(dead_code)]

use cqrs::VersionedEvent as _;
use cqrs_codegen::{Event, VersionedEvent};

#[test]
fn derives_for_struct() {
    #[derive(Default, Event, VersionedEvent)]
    #[event(type = "test.event", version = 1)]
    struct TestEvent {
        id: i32,
        data: String,
    };

    let version = cqrs::EventVersion::new(1).unwrap();

    assert_eq!(TestEvent::EVENT_VERSION, version);
    assert_eq!(*TestEvent::default().event_version(), version);
}

#[test]
fn derives_for_generic_struct() {
    #[derive(Default, Event, VersionedEvent)]
    #[event(type = "test.event.generic", version = 1)]
    struct TestEventGeneric<ID, Data> {
        id: ID,
        data: Data,
    };

    type TestEvent = TestEventGeneric<i32, String>;

    let version = cqrs::EventVersion::new(1).unwrap();

    assert_eq!(TestEvent::EVENT_VERSION, version);
    assert_eq!(*TestEvent::default().event_version(), version);
}

#[test]
fn derives_for_enum() {
    #[derive(Default, Event, VersionedEvent)]
    #[event(type = "test.event.1", version = 1)]
    struct TestEvent1;

    #[derive(Default, Event, VersionedEvent)]
    #[event(type = "test.event.2", version = 2)]
    struct TestEvent2;

    #[derive(Event, VersionedEvent)]
    enum TestEvent {
        TestEventTuple(TestEvent1),
        TestEventStruct { event: TestEvent2 },
    }

    let version1 = cqrs::EventVersion::new(1).unwrap();
    let version2 = cqrs::EventVersion::new(2).unwrap();

    assert_eq!(
        *TestEvent::TestEventTuple(Default::default()).event_version(),
        version1,
    );
    assert_eq!(
        *TestEvent::TestEventStruct {
            event: Default::default()
        }
        .event_version(),
        version2,
    );
}

#[test]
fn derives_for_generic_enum() {
    #[derive(Default, Event, VersionedEvent)]
    #[event(type = "test.event.1", version = 1)]
    struct TestEvent1;

    #[derive(Default, Event, VersionedEvent)]
    #[event(type = "test.event.2", version = 2)]
    struct TestEvent2;

    #[derive(Default, Event, VersionedEvent)]
    #[event(type = "test.event.generic.1", version = 3)]
    struct TestEventGeneric1<ID, Data> {
        id: ID,
        data: Data,
    }

    #[derive(Default, Event, VersionedEvent)]
    #[event(type = "test.event.generic.2", version = 4)]
    struct TestEventGeneric2<ID, Data> {
        id: ID,
        data: Data,
    }

    #[derive(Event, VersionedEvent)]
    enum TestEventGeneric<TE1, TE2, ID, Data> {
        TestEventTuple(TE1),
        TestEventStruct { event: TE2 },
        TestEventTupleGeneric(TestEventGeneric1<ID, Data>),
        TestEventStructGeneric { event: TestEventGeneric2<ID, Data> },
    }

    type TestEvent = TestEventGeneric<TestEvent1, TestEvent2, i32, String>;

    let version1 = cqrs::EventVersion::new(1).unwrap();
    let version2 = cqrs::EventVersion::new(2).unwrap();
    let version3 = cqrs::EventVersion::new(3).unwrap();
    let version4 = cqrs::EventVersion::new(4).unwrap();

    assert_eq!(
        *TestEvent::TestEventTuple(Default::default()).event_version(),
        version1,
    );
    assert_eq!(
        *TestEvent::TestEventStruct {
            event: Default::default()
        }
        .event_version(),
        version2,
    );
    assert_eq!(
        *TestEvent::TestEventTupleGeneric(Default::default()).event_version(),
        version3,
    );
    assert_eq!(
        *TestEvent::TestEventStructGeneric {
            event: Default::default()
        }
        .event_version(),
        version4,
    );
}
