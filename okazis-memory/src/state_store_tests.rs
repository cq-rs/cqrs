pub use super::*;

#[derive(Default, Clone, Copy, PartialEq, Hash, Debug)]
struct TestState;

#[test]
fn can_create_default_instance() {
    let _: MemoryStateStore<usize, TestState> = Default::default();
}

#[test]
fn can_get_state_from_store() {
    let ms : MemoryStateStore<usize, TestState> = Default::default();
    let ts = ms.get_state(0);
    assert!(ts.is_ok());
}

#[test]
fn can_round_trip_a_value() {
    let ms: MemoryStateStore<_, _> = Default::default();
    let expected = PersistedState {
        offset: 23,
        state: TestState,
    };
    ms.put_state(0,23, expected.state.clone());
    let ts = ms.get_state(0);
    assert_eq!(Ok(Some(expected)), ts);
}