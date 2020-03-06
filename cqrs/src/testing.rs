use cqrs_core::{Aggregate, AggregateCommand, AggregateEvent, AggregateId, Event, View};
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

/// A test view with no data
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestView;

/// A test identifier
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestId<'a>(pub &'a str);

impl Aggregate for TestAggregate {
    fn aggregate_type() -> &'static str {
        "test"
    }
}

impl<'a> AggregateId<TestAggregate> for TestId<'a> {
    fn as_str(&self) -> &str {
        self.0
    }
}

impl AggregateCommand<TestAggregate> for TestCommand {
    type Error = Void;
    type Event = TestEvent;
    type Events = Vec<TestEvent>;

    fn execute_on(self, _aggregate: &TestAggregate) -> Result<Self::Events, Self::Error> {
        Ok(Vec::new())
    }
}

const EVENT_TYPE: &str = "test";

impl Event for TestEvent {
    fn event_type(&self) -> &'static str {
        EVENT_TYPE
    }
}

impl AggregateEvent<TestAggregate> for TestEvent {
    fn apply_to(&self, _aggregate: &mut TestAggregate) {}
}

impl View<TestEvent> for TestView {
    fn apply_events(&mut self, events: &Vec<TestEvent>) {}
}
