use aggregate::Aggregate;
use types::{CqrsError, EventNumber, Precondition, VersionedEvent, Since, VersionedAggregate, VersionedAggregateView};

pub trait EventSource<A: Aggregate> {
    type Events: IntoIterator<Item=Result<VersionedEvent<A::Event>, Self::Error>>;
    type Error: CqrsError;

    fn read_events(&self, id: &str, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error>;
}

pub trait EventSink<A: Aggregate, M> {
    type Error: CqrsError;

    fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>, metadata: M) -> Result<EventNumber, Self::Error>;

    fn append_events_from_iterator<I>(&self, id: &str, event_iter: I, precondition: Option<Precondition>, metadata: M) -> Result<EventNumber, Self::Error>
        where
            I: IntoIterator<Item=A::Event>,
            Self: Sized,
    {
        let events: Vec<A::Event> = event_iter.into_iter().collect();
        self.append_events(id, &events, precondition, metadata)
    }
}

pub trait SnapshotSource<A: Aggregate> {
    type Error: CqrsError;

    fn get_snapshot(&self, id: &str) -> Result<Option<VersionedAggregate<A>>, Self::Error>;
}

pub trait SnapshotSink<A: Aggregate> {
    type Error: CqrsError;

    fn persist_snapshot(&self, id: &str, view: VersionedAggregateView<A>) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::Empty;
    use void::Void;

    /// A test aggregate with no state
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct TestAggregate;

    /// A test event with no data
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct TestEvent;

    /// A test command with no data
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct TestCommand;

    impl Aggregate for TestAggregate {
        type Event = TestEvent;
        type Events = Empty<Self::Event>;

        type Command = TestCommand;
        type Error = Void;

        fn apply(&mut self, _event: Self::Event) {}

        fn execute(&self, _command: Self::Command) -> Result<Self::Events, Self::Error> {
            Ok(Empty::default())
        }

        fn entity_type() -> &'static str {
            "test"
        }
    }

    assert_obj_safe!(event_source_object_safety; EventSource<TestAggregate, Events=Empty<()>, Error=Void>);
    assert_obj_safe!(event_sink_object_safety; EventSink<TestAggregate, Error=Void>);
    assert_obj_safe!(snapshot_source_object_safety; SnapshotSource<TestAggregate, Error=Void>);
    assert_obj_safe!(snapshot_sink_object_safety; SnapshotSink<TestAggregate, Error=Void>);
}
