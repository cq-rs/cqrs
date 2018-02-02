pub use super::*;

#[derive(PartialEq, Clone, Copy, Hash, Debug)]
pub struct TestEvent {
    value: usize,
}

#[test]
fn can_create_internally() {
    let _ = MemoryEventStream::<TestEvent>::new();
}

#[test]
fn can_add_events_with_event_stream_trait() {
    let mut es = MemoryEventStream::new();
    let events = Vec::<TestEvent>::new();
    EventStream::append_events(&mut es, events);
}

#[test]
fn can_read_events_from_event_stream() {
    let es = MemoryEventStream::new();
    let events: Result<Vec<TestEvent>, _> = es.read(0);
    assert_eq!(events, Ok(Vec::default()));
}

#[test]
fn can_add_events_and_read_them_back_out() {
    let mut es = MemoryEventStream::new();
    let expected_events = vec![
        TestEvent { value: 143 },
        TestEvent { value: 554 },
    ];
    es.append_events(expected_events.clone());
    let actual_events = es.read(0);
    assert_eq!(Ok(expected_events), actual_events);
}

#[test]
fn can_add_events_and_read_from_middle() {
    let mut es = MemoryEventStream::new();
    let all_events = vec![
        TestEvent { value: 143 },
        TestEvent { value: 554 },
    ];
    es.append_events(all_events.clone());
    let expected_events = vec![
        TestEvent { value: 554 },
    ];
    let actual_events = es.read(1);
    assert_eq!(Ok(expected_events), actual_events);
}

#[test]
fn reading_with_offset_one_past_end_gives_empty_set() {
    let mut es = MemoryEventStream::new();
    let all_events = vec![
        TestEvent { value: 143 },
        TestEvent { value: 554 },
    ];
    es.append_events(all_events.clone());
    let expected_events = Vec::<TestEvent>::default();
    let actual_events = es.read(2);
    assert_eq!(Ok(expected_events), actual_events);
}

#[test]
fn reading_with_offset_more_than_one_past_end_gives_error() {
    let mut es = MemoryEventStream::new();
    let all_events = vec![
        TestEvent { value: 143 },
        TestEvent { value: 554 },
    ];
    es.append_events(all_events.clone());
    let expected_events = Err(ReadError::ReadPastEndOfStream);
    let actual_events = es.read(3);
    assert_eq!(expected_events, actual_events);
}
