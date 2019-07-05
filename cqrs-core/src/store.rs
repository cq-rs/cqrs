use crate::{
    aggregate::{Aggregate, AggregateEvent},
    types::{
        CqrsError, EventNumber, Precondition, Since, SnapshotRecommendation, Version,
        VersionedAggregate, VersionedEvent,
    },
};

/// A source for reading/loading events.
pub trait EventSource<A, E>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    /// Represents the sequence of events read from the event source.
    type Events: IntoIterator<Item = VersionedEvent<E>>;

    /// The error type.
    type Error: CqrsError;

    /// Reads events from the event source for a given identifier.
    ///
    /// Only loads events after the event number provided in `since` (See [Since]), and will only load a maximum of
    /// `max_count` events, if given. If not given, will read all remaining events.
    fn read_events(
        &self,
        id: &A::Id,
        since: Since,
        max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>;
}

/// A sink for writing/persisting events with associated metadata.
pub trait EventSink<A, E, M>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    /// The error type.
    type Error: CqrsError;

    /// Appends events to a given source, with an optional precondition, and associated metadata.
    ///
    /// The associated metadata is applied to all events in the append group.
    fn append_events(
        &self,
        id: &A::Id,
        events: &[E],
        precondition: Option<Precondition>,
        metadata: M,
    ) -> Result<EventNumber, Self::Error>;
}

/// A source for reading/loading snapshots of aggregates.
pub trait SnapshotSource<A>
where
    A: Aggregate,
{
    /// The error type.
    type Error: CqrsError;

    /// Loads a versioned aggregate from the snapshot source.
    fn get_snapshot(&self, id: &A::Id) -> Result<Option<VersionedAggregate<A>>, Self::Error>;
}

/// A sink for writing/persisting snapshots of aggregates.
pub trait SnapshotSink<A>
where
    A: Aggregate,
{
    /// The error type.
    type Error: CqrsError;

    /// Writes an aggregate with its version to the sink. Returns the version number of the latest snapshot.
    fn persist_snapshot(
        &self,
        id: &A::Id,
        aggregate: &A,
        version: Version,
        last_snapshot_version: Option<Version>,
    ) -> Result<Version, Self::Error>;
}

/// A strategy determining when to recommend a snapshot be taken.
pub trait SnapshotStrategy {
    /// Gives the sink's recommendation on whether or not to perform a snapshot
    fn snapshot_recommendation(
        &self,
        version: Version,
        last_snapshot_version: Option<Version>,
    ) -> SnapshotRecommendation;
}

/// A snapshot strategy that will never recommend taking a snapshot.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct NeverSnapshot;

impl SnapshotStrategy for NeverSnapshot {
    fn snapshot_recommendation(&self, _: Version, _: Option<Version>) -> SnapshotRecommendation {
        SnapshotRecommendation::DoNotSnapshot
    }
}

/// A snapshot strategy that will always recommend taking a snapshot.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct AlwaysSnapshot;

impl SnapshotStrategy for AlwaysSnapshot {
    fn snapshot_recommendation(&self, _: Version, _: Option<Version>) -> SnapshotRecommendation {
        SnapshotRecommendation::ShouldSnapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AggregateCommand, AggregateEvent, Event};
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

    /// Constant ID of test aggregate.
    const TEST_AGGREGATE_ID: u8 = 1;

    impl Aggregate for TestAggregate {
        type Id = u8;

        fn aggregate_type() -> &'static str {
            "test"
        }

        fn id(&self) -> &u8 {
            &TEST_AGGREGATE_ID
        }
    }

    impl AggregateEvent<TestAggregate> for TestEvent {
        fn apply_to(self, _aggregate: &mut TestAggregate) {}
    }

    impl AggregateCommand<TestAggregate> for TestCommand {
        type Error = Void;
        type Event = TestEvent;
        type Events = Vec<TestEvent>;

        fn execute_on(self, _aggregate: &TestAggregate) -> Result<Self::Events, Self::Error> {
            Ok(Vec::default())
        }
    }

    impl Event for TestEvent {
        fn event_type(&self) -> &'static str {
            "test"
        }
    }
}
