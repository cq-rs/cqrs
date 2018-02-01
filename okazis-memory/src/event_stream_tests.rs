pub use super::*;

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

