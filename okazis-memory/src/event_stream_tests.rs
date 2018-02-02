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
    let events: Vec<TestEvent> = es.read(0);
    assert_eq!(events, Vec::default());
}

#[test]
fn can_add_events_and_read_them_back_out() {
    let es = MemoryEventStream::new();
    let expected_events = vec![
        TestEvent { value: 143 },
        TestEvent { value: 554 },
    ];
    es.append_events(expected_events.clone());
    let actual_events = es.read(0);
    assert_eq!(expected_events, actual_events);
}
