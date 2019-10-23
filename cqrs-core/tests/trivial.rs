use std::convert::Infallible;

use async_trait::async_trait;
use cqrs_core as cqrs;

/// Test aggregate with no state.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct TestAggregate;

impl TestAggregate {
    /// Constant ID of test aggregate.
    const ID: u8 = 1;
}

impl cqrs::Aggregate for TestAggregate {
    type Id = u8;

    fn aggregate_type(&self) -> cqrs::AggregateType {
        "test"
    }

    fn id(&self) -> &Self::Id {
        &TestAggregate::ID
    }
}

/// Test event with no data.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct TestEvent;

impl cqrs::Event for TestEvent {
    fn event_type(&self) -> cqrs::EventType {
        "test"
    }
}

impl cqrs::EventSourced<TestEvent> for TestAggregate {
    fn apply(&mut self, _: &TestEvent) {}
}

/// Test command with no data.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct TestCommand;

impl cqrs::Command for TestCommand {
    type Aggregate = TestAggregate;

    fn aggregate_id(&self) -> Option<&u8> {
        Some(&TestAggregate::ID)
    }
}

#[async_trait(?Send)]
impl cqrs::CommandHandler<TestCommand> for TestAggregate {
    type Context = ();
    type Err = Infallible;
    type Event = TestEvent;
    type Ok = ();

    async fn handle(&self, _: TestCommand, _: &()) -> Result<(), Infallible> {
        Ok(())
    }
}

/// Test metadata with no data.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct TestMetadata;
