use super::*;
use ::event::{EventSource as EI, EventSink as EO};
use ::state::{SnapshotSource as SI, SnapshotSink as SO};
use void::ResultVoidExt;

struct TestAggregate;

impl cqrs::Aggregate for TestAggregate {
    type Event = TestEvent;
    type Events = Option<Self::Event>;
    type Command = ();
    type Error = ();

    fn apply(&mut self, event: Self::Event) {
        unimplemented!()
    }

    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error> {
        unimplemented!()
    }

    fn entity_type() -> &'static str where Self: Sized {
        "test"
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
struct TestEvent;

type TestMemoryEventStore = EventStore<usize, TestEvent>;

#[test]
fn can_get_an_event_stream_with_expected_count_of_events() {
    let es = TestMemoryEventStore::default();
    let id = 0;
    es.append_events(&id, &vec![
        TestEvent
    ], None).unwrap();
    let events = es.read_events(&id, Since::BeginningOfStream).void_unwrap().unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn can_get_an_event_stream_with_expected_count_of_events_when_not_starting_from_beginning_of_stream() {
    let es = TestMemoryEventStore::default();
    let id = 0;
    es.append_events(&id, &vec![
        TestEvent
    ], None).unwrap();
    let events = es.read_events(&id, Since::Event(EventNumber::new(0))).void_unwrap().unwrap();
    assert_eq!(events.len(), 0);
}

#[test]
fn can_get_an_event_stream_with_expected_count_of_events_when_asking_past_end_of_stream() {
    let es = TestMemoryEventStore::default();
    let id = 0;
    es.append_events(&id, &vec![
        TestEvent
    ], None).unwrap();
    let events = es.read_events(&id, Since::Event(EventNumber::new(1))).void_unwrap().unwrap();
    assert_eq!(events.len(), 0);
}

#[test]
fn can_get_an_event_stream_multiple_times_are_equal() {
    let es = TestMemoryEventStore::default();
    let id = 0;
    es.append_events(&id, &vec![
        TestEvent
    ], None).unwrap();
    let events1 = es.read_events(&id, Since::BeginningOfStream);
    let events2 = es.read_events(&id, Since::BeginningOfStream);
    assert_eq!(events1, events2);
}

#[test]
fn can_get_different_event_streams() {
    let es = TestMemoryEventStore::default();

    es.append_events(&0, &vec![
        TestEvent
    ], None).unwrap();
    let events1 = es.read_events(&0, Since::BeginningOfStream);
    let events2 = es.read_events(&1, Since::BeginningOfStream);
    assert_ne!(events1, events2);
}
