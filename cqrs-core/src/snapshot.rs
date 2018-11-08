use aggregate::Aggregate;
use types::{CqrsError, StateSnapshot, StateSnapshotView};

pub trait SnapshotSource<A: Aggregate> {
    type Error: CqrsError;

    fn get_snapshot(&self, id: &str) -> Result<Option<StateSnapshot<A>>, Self::Error>;
}

pub trait SnapshotSink<A: Aggregate> {
    type Error: CqrsError;

    fn persist_snapshot(&self, id: &str, snapshot: StateSnapshotView<A>) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use aggregate::testing::TestAggregate;
    use void::Void;

    assert_obj_safe!(snapshot_source_object_safety; SnapshotSource<TestAggregate, Error=Void>);
    assert_obj_safe!(snapshot_sink_object_safety; SnapshotSink<TestAggregate, Error=Void>);
}