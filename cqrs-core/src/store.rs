use aggregate::{Aggregate, AggregateId};
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
    fn read_events<I>(&self, id: &I, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<Aggregate=A>;
}

/// A sink for writing/persisting events with associated metadata.
pub trait EventSink<A: Aggregate, M> {
    /// The error type.
    type Error: CqrsError;

    /// Appends events to a given source, with an optional precondition, and associated metadata.
    ///
    /// The associated metadata is applied to all events in the append group.
    fn append_events<I>(&self, id: &I, events: &[A::Event], precondition: Option<Precondition>, metadata: M) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<Aggregate=A>;
}

/// A source for reading/loading snapshots of aggregates.
pub trait SnapshotSource<A: Aggregate> {
    /// The error type.
    type Error: CqrsError;

    /// Loads a versioned aggregate from the snapshot source.
    fn get_snapshot<I>(&self, id: &I) -> Result<Option<VersionedAggregate<A>>, Self::Error>
    where
        I: AggregateId<Aggregate=A>;
}

/// A sink for writing/persisting snapshots of aggregates.
pub trait SnapshotSink<A: Aggregate> {
    /// The error type.
    type Error: CqrsError;

    /// Writes a versioned view of the aggregate to the sink.
    fn persist_snapshot<I>(&self, id: &I, view: VersionedAggregateView<A>) -> Result<(), Self::Error>
    where
        I: AggregateId<Aggregate=A>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AggregateEvent, AggregateCommand, Event};
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
        type Events = Vec<TestEvent>;
        type Error = Void;

        fn execute_on(self, _aggregate: &Self::Aggregate) -> Result<Self::Events, Self::Error> {
            Ok(Vec::default())
        }
    }

    impl Event for TestEvent {
        fn event_type(&self) -> &'static str {
            "test"
        }
    }
}
