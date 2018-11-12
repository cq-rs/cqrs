use super::*;
use testing::*;
use {EventSink, EventSource};

type TestMemoryEventStore = EventStore<TestAggregate>;

#[test]
fn can_get_an_event_stream_with_expected_count_of_events() {
    let es = TestMemoryEventStore::default();
    let id = "";
    es.append_events(id, &vec![
        TestEvent
    ], None).unwrap();
    let events = es.read_events(id, Since::BeginningOfStream, None).unwrap().unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn can_get_an_event_stream_with_expected_count_of_events_when_not_starting_from_beginning_of_stream() {
    let es = TestMemoryEventStore::default();
    let id = "";
    es.append_events(id, &vec![
        TestEvent
    ], None).unwrap();
    let events = es.read_events(id, Since::Event(EventNumber::MIN_VALUE), None).unwrap().unwrap();
    assert_eq!(events.len(), 0);
}

#[test]
fn can_get_an_event_stream_with_expected_count_of_events_when_asking_past_end_of_stream() {
    let es = TestMemoryEventStore::default();
    let id = "";
    es.append_events(id, &vec![
        TestEvent
    ], None).unwrap();
    let events = es.read_events(id, Since::Event(EventNumber::MIN_VALUE.incr()), None).unwrap().unwrap();
    assert_eq!(events.len(), 0);
}

#[test]
fn can_get_an_event_stream_multiple_times_are_equal() {
    let es = TestMemoryEventStore::default();
    let id = "";
    es.append_events(id, &vec![
        TestEvent
    ], None).unwrap();
    let events1 = es.read_events(id, Since::BeginningOfStream, None);
    let events2 = es.read_events(id, Since::BeginningOfStream, None);
    assert_eq!(events1, events2);
}

#[test]
fn can_get_different_event_streams() {
    let es = TestMemoryEventStore::default();

    es.append_events("", &vec![
        TestEvent
    ], None).unwrap();
    let events1 = es.read_events("", Since::BeginningOfStream, None);
    let events2 = es.read_events("other", Since::BeginningOfStream, None);
    assert_ne!(events1, events2);
}
