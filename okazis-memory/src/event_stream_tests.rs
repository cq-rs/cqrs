pub use super::*;

#[derive(PartialEq,Clone,Hash,Debug)]
pub struct TestEvent;

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
