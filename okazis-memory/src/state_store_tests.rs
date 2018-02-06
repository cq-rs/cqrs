pub use super::*;

#[derive(Default, Clone, Copy, PartialEq, Hash, Debug)]
struct TestState;

#[test]
fn can_create_default_instance() {
    let _: MemoryStateStore<usize, usize, TestState> = Default::default();
}

#[test]
fn can_get_state_from_store() {
    let ms: MemoryStateStore<usize, usize, TestState> = Default::default();
    let ts = ms.get_state(0);
    assert!(ts.is_ok());
}

#[test]
fn can_round_trip_a_value() {
    let ms = MemoryStateStore::<usize, usize, TestState>::default();
    let expected = PersistedState {
        offset: 23,
        state: TestState,
    };
    ms.put_state(0, expected.offset, expected.state.clone());
    let ts = ms.get_state(0);
    assert_eq!(Ok(Some(expected)), ts);
}

#[test]
fn can_round_trip_multiple_values() {
    let ms = MemoryStateStore::<usize, usize, TestState>::default();
    let e0 = PersistedState {
        offset: 14,
        state: TestState,
    };
    let e1 = PersistedState {
        offset: 299,
        state: TestState,
    };
    ms.put_state(0, e0.offset, e0.state.clone());
    ms.put_state(1, e1.offset, e1.state.clone());
    let t0 = ms.get_state(0);
    let t1 = ms.get_state(1);
    let t2 = ms.get_state(2);
    assert_eq!(Ok(Some(e0)), t0);
    assert_eq!(Ok(Some(e1)), t1);
    assert_eq!(Ok(None), t2);
}

#[test]
fn can_have_memory_state_store_with_alternate_hasher() {
    let ms = MemoryStateStore::<_, _, _, ::fnv::FnvBuildHasher>::default();
    let e0 = PersistedState {
        offset: 14,
        state: TestState,
    };
    let e1 = PersistedState {
        offset: 299,
        state: TestState,
    };
    ms.put_state(0, e0.offset, e0.state.clone());
    ms.put_state(1, e1.offset, e1.state.clone());
    let t0 = ms.get_state(0);
    let t1 = ms.get_state(1);
    let t2 = ms.get_state(2);
    assert_eq!(Ok(Some(e0)), t0);
    assert_eq!(Ok(Some(e1)), t1);
    assert_eq!(Ok(None), t2);
}

#[test]
fn can_have_memory_state_store_with_alternate_key() {
    let ms = MemoryStateStore::<_, _, _>::default();
    let e0 = PersistedState {
        offset: 14,
        state: TestState,
    };
    let e1 = PersistedState {
        offset: 299,
        state: TestState,
    };
    ms.put_state("0", e0.offset, e0.state.clone());
    ms.put_state("1", e1.offset, e1.state.clone());
    let t0 = ms.get_state("0");
    let t1 = ms.get_state("1");
    let t2 = ms.get_state("2");
    assert_eq!(Ok(Some(e0)), t0);
    assert_eq!(Ok(Some(e1)), t1);
    assert_eq!(Ok(None), t2);
}