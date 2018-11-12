use aggregate::{Aggregate, PersistableAggregate};
use types::{CqrsError, EventNumber, Precondition, VersionedEvent, Since, VersionedAggregate, VersionedAggregateView};

pub trait EventSource<A: Aggregate> {
    type Events: IntoIterator<Item=Result<VersionedEvent<A::Event>, Self::Error>>;
    type Error: CqrsError;

    fn read_events(&self, id: &str, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error>;
}

pub trait EventSink<A: Aggregate> {
    type Error: CqrsError;

    fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error>;

    fn append_events_from_iterator<I>(&self, id: &str, event_iter: I, precondition: Option<Precondition>) -> Result<EventNumber, Self::Error>
        where
            I: IntoIterator<Item=A::Event>,
            Self: Sized,
    {
        let events: Vec<A::Event> = event_iter.into_iter().collect();
        self.append_events(id, &events, precondition)
    }
}

pub trait SnapshotSource<A: PersistableAggregate> {
    type Error: CqrsError;

    fn get_snapshot(&self, id: &str) -> Result<Option<VersionedAggregate<A>>, Self::Error>;
}

pub trait SnapshotSink<A: PersistableAggregate> {
    type Error: CqrsError;

    fn persist_snapshot(&self, id: &str, aggregate: VersionedAggregateView<A>) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::Empty;
    use aggregate::testing::TestAggregate;
    use void::Void;

    assert_obj_safe!(event_source_object_safety; EventSource<TestAggregate, Events=Empty<()>, Error=Void>);
    assert_obj_safe!(event_sink_object_safety; EventSink<TestAggregate, Error=Void>);
    assert_obj_safe!(snapshot_source_object_safety; SnapshotSource<TestAggregate, Error=Void>);
    assert_obj_safe!(snapshot_sink_object_safety; SnapshotSink<TestAggregate, Error=Void>);
}
