pub use super::*;

#[derive(Clone,Debug,Hash,PartialEq,Copy)]
struct TestEvent;

#[test]
fn implements_default_trait() {
    let _: MemoryEventStore<TestEvent> = Default::default();
}

#[test]
fn can_get_an_event_stream() {
    let es: MemoryEventStore<TestEvent> = Default::default();
    let id = 0usize;
    let _: MemoryEventStream<TestEvent> = es.open_stream(id);
}

