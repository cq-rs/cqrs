use std::iter::Empty;
use void::Void;
use cqrs_core::{Aggregate, Event};

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

impl Aggregate for TestAggregate {
    type Event = TestEvent;
    type Events = Empty<Self::Event>;

    type Command = TestCommand;
    type Error = Void;

    fn apply(&mut self, _event: Self::Event) {}

    fn execute(&self, _command: Self::Command) -> Result<Self::Events, Self::Error> {
        Ok(Self::Events::default())
    }

    fn entity_type() -> &'static str {
        "test"
    }
}

const EVENT_TYPE: &str = "test";

impl Event for TestEvent {
    fn event_type(&self) -> &'static str {
        EVENT_TYPE
    }
}

#[cfg(feature = "proptest")]
pub mod proptest {

}