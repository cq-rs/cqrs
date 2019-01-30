use cqrs_core::{Aggregate, AggregateCommand, AggregateEvent, AggregateId, Event};
use void::Void;

/// A test aggregate with no state
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestAggregate;

/// A test event with no data
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestEvent;

/// A test metadata with no data
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestMetadata;

/// A test command with no data
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestCommand;

/// A test identifier
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestId<'a>(pub &'a str);

impl<'a> AsRef<str> for TestId<'a> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Aggregate for TestAggregate {
    type Event = TestEvent;

    fn entity_type() -> &'static str {
        "test"
    }
}

impl<'a> AggregateId for TestId<'a> {
    type Aggregate = TestAggregate;
}

impl AggregateCommand for TestCommand {
    type Aggregate = TestAggregate;
    type Error = Void;
    type Events = Vec<TestEvent>;

    fn execute_on(self, _aggregate: &Self::Aggregate) -> Result<Self::Events, Self::Error> {
        Ok(Vec::new())
    }
}

const EVENT_TYPE: &str = "test";

impl Event for TestEvent {
    fn event_type(&self) -> &'static str {
        EVENT_TYPE
    }
}

impl AggregateEvent for TestEvent {
    type Aggregate = TestAggregate;

    fn apply_to(self, _aggregate: &mut Self::Aggregate) {}
}
