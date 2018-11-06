pub use super::*;

#[derive(PartialEq, Clone, Copy, Hash, Debug)]
pub struct TestEvent {
    value: usize,
}

type TestMemoryEventStream = MemoryEventStream<TestEvent>;

#[test]
fn can_create_default() {
    let _ = TestMemoryEventStream::default();
}

#[test]
fn can_add_events_with_event_stream_trait() {
    let es = TestMemoryEventStream::default();
    let events = Vec::new();
    es.append_events(&events, None).unwrap();
}

#[test]
fn no_events_are_in_empty_event_stream() {
    let es = TestMemoryEventStream::default();
    let events = es.read(Since::BeginningOfStream);
    assert_eq!(events, Vec::default());
}

#[test]
fn can_add_events_and_read_them_back_out() {
    let es = TestMemoryEventStream::default();
    let all_events = vec![
        SequencedEvent { sequence: EventNumber::new(1).unwrap(), event: TestEvent { value: 143 } },
        SequencedEvent { sequence: EventNumber::new(2).unwrap(), event: TestEvent { value: 554 } },
    ];

    let decorated_events: Vec<_> = all_events.iter()
        .map(|pe| pe.event.clone())
        .collect();

    es.append_events(&decorated_events, None).unwrap();
    let actual_events = es.read(Since::BeginningOfStream);
    assert_eq!(all_events, actual_events);
}

#[test]
fn can_add_events_and_read_from_middle() {
    let es = TestMemoryEventStream::default();
    let all_events = vec![
        SequencedEvent { sequence: EventNumber::new(1).unwrap(), event: TestEvent { value: 143 } },
        SequencedEvent { sequence: EventNumber::new(2).unwrap(), event: TestEvent { value: 554 } },
    ];
    let expected_events = vec![
        all_events[1].clone(),
    ];

    let decorated_events: Vec<_> = all_events.iter()
        .map(|pe| pe.event.clone())
        .collect();

    es.append_events(&decorated_events, None).unwrap();
    let actual_events = es.read(Since::Event(EventNumber::new(1).unwrap()));
    assert_eq!(expected_events, actual_events);
}

#[test]
fn reading_with_version_one_past_end_gives_empty_set() {
    let es = TestMemoryEventStream::default();
    let all_events = vec![
        SequencedEvent { sequence: EventNumber::new(1).unwrap(), event: TestEvent { value: 143 } },
        SequencedEvent { sequence: EventNumber::new(2).unwrap(), event: TestEvent { value: 554 } },
    ];

    let decorated_events: Vec<_> = all_events.iter()
        .map(|pe| pe.event.clone())
        .collect();

    es.append_events(&decorated_events, None).unwrap();
    let expected_events = Vec::<SequencedEvent<TestEvent>>::default();
    let actual_events = es.read(Since::Event(EventNumber::new(2).unwrap()));
    assert_eq!(expected_events, actual_events);
}

#[test]
fn reading_with_version_more_than_one_past_end_gives_empty_stream() {
    let es = TestMemoryEventStream::default();
    let all_events = vec![
        SequencedEvent { sequence: EventNumber::new(1).unwrap(), event: TestEvent { value: 143 } },
        SequencedEvent { sequence: EventNumber::new(2).unwrap(), event: TestEvent { value: 554 } },
    ];

    let decorated_events: Vec<_> = all_events.iter()
        .map(|pe| pe.event.clone())
        .collect();

    es.append_events(&decorated_events, None).unwrap();
    let expected_events = Vec::<SequencedEvent<TestEvent>>::default();
    let actual_events = es.read(Since::Event(EventNumber::new(3).unwrap()));
    assert_eq!(expected_events, actual_events);
}
