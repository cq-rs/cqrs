pub use super::*;
use cqrs::Version;
use fnv::FnvBuildHasher;

#[derive(Default, Clone, Copy, PartialEq, Hash, Debug)]
struct TestState;

type TestStateStore = MemoryStateStore<TestState, usize, FnvBuildHasher>;

#[test]
fn can_create_default_instance() {
    let _ = MemoryStateStore::<TestState, usize>::default();
}

#[test]
fn can_create_default_instance_with_alternate_hasher() {
    let _ = MemoryStateStore::<TestState, usize, FnvBuildHasher>::default();
}

#[test]
fn can_get_snapshot_from_store() {
    let ms = TestStateStore::default();
    let ts = ms.get_snapshot(&0);
    assert!(ts.is_ok());
}

#[test]
fn can_round_trip_a_value() {
    let ms = TestStateStore::default();
    let expected = VersionedSnapshot {
        version: Version::new(23),
        snapshot: TestState,
    };
    ms.persist_snapshot(&0, expected.clone()).unwrap();
    let ts = ms.get_snapshot(&0);
    assert_eq!(Ok(Some(expected)), ts);
}

#[test]
fn can_round_trip_multiple_values() {
    let ms = TestStateStore::default();
    let e0 = VersionedSnapshot {
        version: Version::new(14),
        snapshot: TestState,
    };
    let e1 = VersionedSnapshot {
        version: Version::new(299),
        snapshot: TestState,
    };
    ms.persist_snapshot(&0, e0.clone()).unwrap();
    ms.persist_snapshot(&1, e1.clone()).unwrap();
    let t0 = ms.get_snapshot(&0);
    let t1 = ms.get_snapshot(&1);
    let t2 = ms.get_snapshot(&2);
    assert_eq!(Ok(Some(e0)), t0);
    assert_eq!(Ok(Some(e1)), t1);
    assert_eq!(Ok(None), t2);
}

#[test]
fn can_have_memory_snapshot_store_with_alternate_hasher() {
    let ms = TestStateStore::default();
    let e0 = VersionedSnapshot {
        version: Version::new(14),
        snapshot: TestState,
    };
    let e1 = VersionedSnapshot {
        version: Version::new(299),
        snapshot: TestState,
    };
    ms.persist_snapshot(&0, e0.clone()).unwrap();
    ms.persist_snapshot(&1, e1.clone()).unwrap();
    let t0 = ms.get_snapshot(&0);
    let t1 = ms.get_snapshot(&1);
    let t2 = ms.get_snapshot(&2);
    assert_eq!(Ok(Some(e0)), t0);
    assert_eq!(Ok(Some(e1)), t1);
    assert_eq!(Ok(None), t2);
}

#[test]
fn can_have_memory_snapshot_store_with_alternate_key() {
    let ms = MemoryStateStore::<_, &'static str>::default();
    let e0 = VersionedSnapshot {
        version: Version::new(14),
        snapshot: TestState,
    };
    let e1 = VersionedSnapshot {
        version: Version::new(299),
        snapshot: TestState,
    };
    ms.persist_snapshot(&"0", e0.clone()).unwrap();
    ms.persist_snapshot(&"1", e1.clone()).unwrap();
    let t0 = ms.get_snapshot(&"0");
    let t1 = ms.get_snapshot(&"1");
    let t2 = ms.get_snapshot(&"2");
    assert_eq!(Ok(Some(e0)), t0);
    assert_eq!(Ok(Some(e1)), t1);
    assert_eq!(Ok(None), t2);
}