#![feature(async_await)]

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

    fn aggregate_type() -> &'static str {
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
    fn event_type(&self) -> &'static str {
        "test"
    }
}

impl cqrs::EventSourced<TestEvent> for TestAggregate {
    fn apply_event(&mut self, _: &TestEvent) {}
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

#[async_trait]
impl cqrs::CommandHandler<TestCommand> for TestAggregate {
    type Context = ();
    type Event = TestEvent;
    type Err = Infallible;
    type Ok = ();

    async fn handle_command(&self, _: TestCommand, _: &()) -> Result<(), Infallible> {
        Ok(())
    }
}

/// Test metadata with no data.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct TestMetadata;
