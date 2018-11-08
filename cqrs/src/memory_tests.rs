use super::*;
use {Aggregate, EventSink, EventSource};
use void::ResultVoidExt;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct TestAggregate;

impl Aggregate for TestAggregate {
    type Event = TestEvent;
    type Events = ::std::iter::Empty<Self::Event>;
    type Command = ();
    type Error = ::void::Void;

    fn apply(&mut self, _event: Self::Event) {}

    fn execute(&self, _command: Self::Command) -> Result<Self::Events, Self::Error> {
        Ok(Default::default())
    }

    fn entity_type() -> &'static str where Self: Sized {
        "test"
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub(crate) struct TestEvent;

type TestMemoryEventStore = EventStore<TestAggregate>;

#[test]
fn can_get_an_event_stream_with_expected_count_of_events() {
    let es = TestMemoryEventStore::default();
    let id = "";
    es.append_events(id, &vec![
        TestEvent
    ], None).unwrap();
    let events = es.read_events(id, Since::BeginningOfStream).void_unwrap().unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn can_get_an_event_stream_with_expected_count_of_events_when_not_starting_from_beginning_of_stream() {
    let es = TestMemoryEventStore::default();
    let id = "";
    es.append_events(id, &vec![
        TestEvent
    ], None).unwrap();
    let events = es.read_events(id, Since::Event(EventNumber::MIN_VALUE)).void_unwrap().unwrap();
    assert_eq!(events.len(), 0);
}

#[test]
fn can_get_an_event_stream_with_expected_count_of_events_when_asking_past_end_of_stream() {
    let es = TestMemoryEventStore::default();
    let id = "";
    es.append_events(id, &vec![
        TestEvent
    ], None).unwrap();
    let events = es.read_events(id, Since::Event(EventNumber::MIN_VALUE.incr())).void_unwrap().unwrap();
    assert_eq!(events.len(), 0);
}

#[test]
fn can_get_an_event_stream_multiple_times_are_equal() {
    let es = TestMemoryEventStore::default();
    let id = "";
    es.append_events(id, &vec![
        TestEvent
    ], None).unwrap();
    let events1 = es.read_events(id, Since::BeginningOfStream);
    let events2 = es.read_events(id, Since::BeginningOfStream);
    assert_eq!(events1, events2);
}

#[test]
fn can_get_different_event_streams() {
    let es = TestMemoryEventStore::default();

    es.append_events("", &vec![
        TestEvent
    ], None).unwrap();
    let events1 = es.read_events("", Since::BeginningOfStream);
    let events2 = es.read_events("other", Since::BeginningOfStream);
    assert_ne!(events1, events2);
}
