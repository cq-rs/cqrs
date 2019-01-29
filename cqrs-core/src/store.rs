use aggregate::Aggregate;
use types::{CqrsError, EventNumber, Precondition, VersionedEvent, Since, VersionedAggregate, VersionedAggregateView};

/// A source for reading/loading events.
pub trait EventSource<A: Aggregate> {
    /// Represents the sequence of events read from the event source.
    type Events: IntoIterator<Item=Result<VersionedEvent<A::Event>, Self::Error>>;

    /// The error type.
    type Error: CqrsError;

    /// Reads events from the event source for a given identifier.
    ///
    /// Only loads events after the event number provided in `since` (See [Since]), and will only load a maximum of
    /// `max_count` events, if given. If not given, will read all remaining events.
    fn read_events(&self, id: &str, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error>;
}

/// A sink for writing/persisting events.
pub trait EventSink<A: Aggregate, M> {
    /// The error type.
    type Error: CqrsError;

    /// Appends events to a given source, with an optional precondition, and associated metadata.
    ///
    /// The associated metadata is applied to all events in the append group.
    fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>, metadata: M) -> Result<EventNumber, Self::Error>;
}

/// A source for reading/loading snapshots of aggregates.
pub trait SnapshotSource<A: Aggregate> {
    /// The error type.
    type Error: CqrsError;

    /// Loads a versioned aggregate from the snapshot source.
    fn get_snapshot(&self, id: &str) -> Result<Option<VersionedAggregate<A>>, Self::Error>;
}

/// A sink for writing/persisting snapshots of aggregates.
pub trait SnapshotSink<A: Aggregate> {
    /// The error type.
    type Error: CqrsError;

    /// Writes a versioned view of the aggregate to the sink.
    fn persist_snapshot(&self, id: &str, view: VersionedAggregateView<A>) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AggregateEvent, AggregateCommand, Event};
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

    /// A test metadata with no data
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct TestMetadata;

    impl Aggregate for TestAggregate {
        fn entity_type() -> &'static str {
            "test"
        }

        type Event = TestEvent;
    }

    impl AggregateEvent for TestEvent {
        type Aggregate = TestAggregate;

        fn apply_to(self, _aggregate: &mut Self::Aggregate) {}
    }

    impl AggregateCommand for TestCommand {
        type Aggregate = TestAggregate;
        type Events = Empty<TestEvent>;
        type Error = Void;

        fn execute_on(self, _aggregate: &Self::Aggregate) -> Result<Self::Events, Self::Error> {
            Ok(Empty::default())
        }
    }

    impl Event for TestEvent {
        fn event_type(&self) -> &'static str {
            "test"
        }
    }

    assert_obj_safe!(event_source_object_safety; EventSource<TestEvent, Events=Empty<()>, Error=Void>);
    assert_obj_safe!(event_sink_object_safety; EventSink<TestEvent, TestMetadata, Error=Void>);
    assert_obj_safe!(snapshot_source_object_safety; SnapshotSource<TestAggregate, Error=Void>);
    assert_obj_safe!(snapshot_sink_object_safety; SnapshotSink<TestAggregate, Error=Void>);
}
