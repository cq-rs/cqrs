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
    let es = MemoryEventStream::new();
    let events = Vec::<TestEvent>::new();
    EventStream::append_events(&es, events);
}

#[test]
fn can_read_events_from_event_stream() {
    let es = MemoryEventStream::new();
    let events: Result<Vec<PersistedEvent<_, TestEvent, _>>, _> = es.read(BeginningOfStream);
    assert_eq!(events, Ok(Vec::default()));
}

#[test]
fn can_add_events_and_read_them_back_out() {
    let es = MemoryEventStream::new();
    let all_events = vec![
        PersistedEvent { offset: 0, payload: TestEvent { value: 143 }, metadata: () },
        PersistedEvent { offset: 1, payload: TestEvent { value: 554 }, metadata: () },
    ];
    es.append_events(all_events.iter().map(|pe| pe.payload.clone()).collect());
    let actual_events = es.read(BeginningOfStream);
    assert_eq!(Ok(all_events), actual_events);
}

#[test]
fn can_add_events_and_read_from_middle() {
    let es = MemoryEventStream::new();
    let all_events = vec![
        PersistedEvent { offset: 0, payload: TestEvent { value: 143 }, metadata: () },
        PersistedEvent { offset: 1, payload: TestEvent { value: 554 }, metadata: () },
    ];
    let expected_events = vec![
        all_events[1].clone(),
    ];
    es.append_events(all_events.iter().map(|pe| pe.payload.clone()).collect());
    let actual_events = es.read(Offset(0));
    assert_eq!(Ok(expected_events), actual_events);
}

#[test]
fn reading_with_offset_one_past_end_gives_empty_set() {
    let es = MemoryEventStream::new();
    let all_events = vec![
        PersistedEvent { offset: 0, payload: TestEvent { value: 143 }, metadata: () },
        PersistedEvent { offset: 1, payload: TestEvent { value: 554 }, metadata: () },
    ];
    es.append_events(all_events.iter().map(|pe| pe.payload.clone()).collect());
    let expected_events = Vec::default();
    let actual_events = es.read(Offset(1));
    assert_eq!(Ok(expected_events), actual_events);
}

#[test]
fn reading_with_offset_more_than_one_past_end_gives_error() {
    let es = MemoryEventStream::new();
    let all_events = vec![
        PersistedEvent { offset: 0, payload: TestEvent { value: 143 }, metadata: () },
        PersistedEvent { offset: 1, payload: TestEvent { value: 554 }, metadata: () },
    ];
    es.append_events(all_events.iter().map(|pe| pe.payload.clone()).collect());
    let expected_events = Err(ReadError::ReadPastEndOfStream);
    let actual_events = es.read(Offset(2));
    assert_eq!(expected_events, actual_events);
}
