pub use super::*;
use fnv::FnvBuildHasher;

#[derive(Default, Clone, Copy, PartialEq, Hash, Debug)]
struct TestState;

type TestStateStore = MemoryStateStore<usize, usize, TestState, FnvBuildHasher>;

#[test]
fn can_create_default_instance() {
    let _ = MemoryStateStore::<usize, usize, TestState>::default();
}

#[test]
fn can_create_default_instance_with_alternate_hasher() {
    let _ = MemoryStateStore::<usize, usize, TestState, FnvBuildHasher>::default();
}

#[test]
fn can_get_state_from_store() {
    let ms = TestStateStore::default();
    let ts = ms.get_state(&0);
    assert!(ts.is_ok());
}

#[test]
fn can_round_trip_a_value() {
    let ms = TestStateStore::default();
    let expected = PersistedSnapshot {
        version: 23,
        data: TestState,
    };
    ms.put_state(&0, expected.version, expected.data.clone()).unwrap();
    let ts = ms.get_state(&0);
    assert_eq!(Ok(Some(expected)), ts);
}

#[test]
fn can_round_trip_multiple_values() {
    let ms = TestStateStore::default();
    let e0 = PersistedSnapshot {
        version: 14,
        data: TestState,
    };
    let e1 = PersistedSnapshot {
        version: 299,
        data: TestState,
    };
    ms.put_state(&0, e0.version, e0.data.clone()).unwrap();
    ms.put_state(&1, e1.version, e1.data.clone()).unwrap();
    let t0 = ms.get_state(&0);
    let t1 = ms.get_state(&1);
    let t2 = ms.get_state(&2);
    assert_eq!(Ok(Some(e0)), t0);
    assert_eq!(Ok(Some(e1)), t1);
    assert_eq!(Ok(None), t2);
}

#[test]
fn can_have_memory_state_store_with_alternate_hasher() {
    let ms = TestStateStore::default();
    let e0 = PersistedSnapshot {
        version: 14,
        data: TestState,
    };
    let e1 = PersistedSnapshot {
        version: 299,
        data: TestState,
    };
    ms.put_state(&0, e0.version, e0.data.clone()).unwrap();
    ms.put_state(&1, e1.version, e1.data.clone()).unwrap();
    let t0 = ms.get_state(&0);
    let t1 = ms.get_state(&1);
    let t2 = ms.get_state(&2);
    assert_eq!(Ok(Some(e0)), t0);
    assert_eq!(Ok(Some(e1)), t1);
    assert_eq!(Ok(None), t2);
}

#[test]
fn can_have_memory_state_store_with_alternate_key() {
    let ms = MemoryStateStore::<&'static str, usize, _>::default();
    let e0 = PersistedSnapshot {
        version: 14,
        data: TestState,
    };
    let e1 = PersistedSnapshot {
        version: 299,
        data: TestState,
    };
    ms.put_state(&"0", e0.version, e0.data.clone()).unwrap();
    ms.put_state(&"1", e1.version, e1.data.clone()).unwrap();
    let t0 = ms.get_state(&"0");
    let t1 = ms.get_state(&"1");
    let t2 = ms.get_state(&"2");
    assert_eq!(Ok(Some(e0)), t0);
    assert_eq!(Ok(Some(e1)), t1);
    assert_eq!(Ok(None), t2);
}