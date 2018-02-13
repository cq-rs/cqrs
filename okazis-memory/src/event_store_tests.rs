pub use super::*;
use okazis::{EventStore, Since};
use fnv::FnvBuildHasher;

#[derive(Clone, Debug, Hash, PartialEq, Copy)]
struct TestEvent;

#[test]
fn implements_default_trait() {
    let _: MemoryEventStore<usize, TestEvent> = Default::default();
}

#[test]
fn can_use_custom_hasher() {
    let _: MemoryEventStore<usize, TestEvent, FnvBuildHasher> = Default::default();
}

type TestMemoryEventStore = MemoryEventStore<usize, TestEvent, FnvBuildHasher>;

#[test]
fn can_get_an_event_stream_multiple_times_are_equal() {
    let es = TestMemoryEventStore::default();
    let id = 0;
    es.append_events(&id, &vec![
        TestEvent
    ], Precondition::Always).unwrap();
    let events1 = es.read(&id, Since::BeginningOfStream);
    let events2 = es.read(&id, Since::BeginningOfStream);
    assert_eq!(events1, events2);
}

#[test]
fn can_get_different_event_streams() {
    let es = TestMemoryEventStore::default();

    es.append_events(&0, &vec![
        TestEvent
    ], Precondition::Always).unwrap();
    let events1 = es.read(&0, Since::BeginningOfStream);
    let events2 = es.read(&1, Since::BeginningOfStream);
    assert_ne!(events1, events2);
}
