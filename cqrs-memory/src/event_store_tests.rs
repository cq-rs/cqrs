pub use super::*;
use fnv::FnvBuildHasher;

#[derive(Clone, Debug, Hash, PartialEq, Copy)]
struct TestEvent;

#[test]
fn implements_default_trait() {
    let _: MemoryEventStore<TestEvent, usize> = Default::default();
}

#[test]
fn can_use_custom_hasher() {
    let _: MemoryEventStore<TestEvent, usize, FnvBuildHasher> = Default::default();
}

type TestMemoryEventStore = MemoryEventStore<TestEvent, usize, FnvBuildHasher>;

#[test]
fn can_get_an_event_stream_multiple_times_are_equal() {
    let es = TestMemoryEventStore::default();
    let id = 0;
    es.append_events(&id, &vec![
        TestEvent
    ], None).unwrap();
    let events1 = es.read_events(&id, Since::BeginningOfStream);
    let events2 = es.read_events(&id, Since::BeginningOfStream);
    assert_eq!(events1, events2);
}

#[test]
fn can_get_different_event_streams() {
    let es = TestMemoryEventStore::default();

    es.append_events(&0, &vec![
        TestEvent
    ], None).unwrap();
    let events1 = es.read_events(&0, Since::BeginningOfStream);
    let events2 = es.read_events(&1, Since::BeginningOfStream);
    assert_ne!(events1, events2);
}
