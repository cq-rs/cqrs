pub use super::*;
use okazis::EventStream;
use okazis::ReadOffset::*;

#[derive(Clone, Debug, Hash, PartialEq, Copy)]
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

#[test]
fn can_get_an_event_stream_multiple_times_are_equal() {
    let es: MemoryEventStore<TestEvent> = Default::default();
    let id = 0;
    let stream1 = es.open_stream(id);
    let stream2 = es.open_stream(id);
    stream1.append_events(vec![
        TestEvent
    ]);
    let events1 = stream1.read(BeginningOfStream);
    let events2 = stream2.read(BeginningOfStream);
    assert_eq!(events1, events2);
}

#[test]
fn can_get_different_event_streams() {
    let es: MemoryEventStore<TestEvent> = Default::default();

    let stream1 = es.open_stream(0);
    let stream2 = es.open_stream(1);

    stream1.append_events(vec![
        TestEvent
    ]);
    let events1 = stream1.read(BeginningOfStream);
    let events2 = stream2.read(BeginningOfStream);
    assert_ne!(events1, events2);
}
