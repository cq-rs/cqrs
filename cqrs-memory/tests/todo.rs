extern crate cqrs;
extern crate cqrs_memory;
extern crate chrono;
extern crate fnv;
extern crate cqrs_todo_core;

use cqrs::trivial::NopEventDecorator;
use cqrs::{Since, VersionedEvent, Version};
use cqrs::domain::{AggregateVersion, HydratedAggregate, AggregatePrecondition};
use cqrs::domain::query::QueryableSnapshotAggregate;
use cqrs::domain::execute::ViewExecutor;
use cqrs::domain::persist::PersistableSnapshotAggregate;
use cqrs::error::{LoadAggregateError, PersistAggregateError, AppendEventsError, Never};
use cqrs_memory::{MemoryEventStore, MemoryStateStore};

use chrono::prelude::*;
use chrono::Duration;

use cqrs_todo_core::{Event, TodoAggregate, TodoState, TodoData, TodoStatus, Command};
use cqrs_todo_core::domain;
use cqrs_todo_core::error;

#[test]
fn main_test() {
    let es = MemoryEventStore::<Event, usize, fnv::FnvBuildHasher>::default();
    //let es = okazis::NullEventStore::<Event, usize>::default();
    let ss = MemoryStateStore::<TodoState, usize, fnv::FnvBuildHasher>::default();
    //let ss = okazis::NullStateStore::<TodoState, usize>::default();

    let view = TodoAggregate::snapshot_with_events_view(&es, &ss);
    let command_view = TodoAggregate::snapshot_with_events_view(&es, &ss);
    let command = TodoAggregate::persist_events_and_snapshot(ViewExecutor::new(command_view), &es, &ss);

    let agg_1 = 0;
    let agg_2 = 34;

    let now = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
    let duration = Duration::seconds(1000);
    //let past_time = now - duration;
    let future_time = now + duration;

    let creation_description = domain::Description::new("Hello world!").unwrap();
    let other_creation_description = domain::Description::new("Otherwise").unwrap();
    let future_reminder = domain::Reminder::new(future_time, now).unwrap();
    let updated_description = domain::Description::new("Complete CQRS implementation").unwrap();

    let decorator = NopEventDecorator::<Event>::default();

    command.execute_and_persist_with_decorator(&agg_1, Command::Create(other_creation_description.clone(), Some(future_reminder)), Some(AggregatePrecondition::New), decorator).unwrap();
    command.execute_and_persist_with_decorator(&agg_2, Command::Create(creation_description.clone(), None), Some(AggregatePrecondition::New), decorator).unwrap();
    println!("0: {:#?}\n", view);
    command.execute_and_persist_with_decorator(&agg_2, Command::SetReminder(future_reminder), Some(AggregatePrecondition::Exists), decorator).unwrap();
    println!("1: {:#?}\n", view);
    command.execute_and_persist_with_decorator(&agg_2, Command::ToggleCompletion, Some(AggregatePrecondition::ExpectedVersion(AggregateVersion::Version(1.into()))), decorator).unwrap();
    println!("2: {:#?}\n", view);
    command.execute_and_persist_with_decorator(&agg_2, Command::MarkCompleted, Some(AggregatePrecondition::Exists), decorator).unwrap();
    println!("3: {:#?}\n", view);
    command.execute_and_persist_with_decorator(&agg_2, Command::ResetCompleted, Some(AggregatePrecondition::Exists), decorator).unwrap();
    println!("4: {:#?}\n", view);
    command.execute_and_persist_with_decorator(&agg_2, Command::CancelReminder, Some(AggregatePrecondition::Exists), decorator).unwrap();
    println!("5: {:#?}\n", view);
    command.execute_and_persist_with_decorator(&agg_2, Command::UpdateText(updated_description.clone()), Some(AggregatePrecondition::Exists), decorator).unwrap();
    println!("6: {:#?}\n", view);
    command.execute_and_persist_with_decorator(&agg_2, Command::MarkCompleted, Some(AggregatePrecondition::Exists), decorator).unwrap();
    println!("7: {:#?}\n", view);
    command.execute_and_persist_with_decorator(&agg_2, Command::MarkCompleted, Some(AggregatePrecondition::Exists), decorator).unwrap();
    println!("8: {:#?}\n", view);

    let expected_state_1 = TodoState::Created(TodoData::new(
        other_creation_description.clone(),
        Some(future_reminder),
        TodoStatus::NotCompleted,
    ));
    let actual_aggregate_1: HydratedAggregate<TodoAggregate> =
        view.rehydrate(&agg_1).unwrap().unwrap();

    assert_eq!(actual_aggregate_1.get_version(), AggregateVersion::Version(Version::new(1)));
    assert_eq!(actual_aggregate_1.inspect_aggregate().inspect_state(), &expected_state_1);

    let expected_events_1 = vec![
        VersionedEvent { version: Version::new(0), event: Event::Created(other_creation_description) },
        VersionedEvent { version: Version::new(1), event: Event::ReminderUpdated(Some(future_reminder)) },
    ];
    let actual_events_1 =
        cqrs::EventSource::read_events(&es, &agg_1, Since::BeginningOfStream).unwrap().unwrap();

    assert_eq!(actual_events_1, expected_events_1);

    let expected_events_2 = vec![
        VersionedEvent { version: Version::new(0), event: Event::Created(creation_description) },
        VersionedEvent { version: Version::new(1), event: Event::ReminderUpdated(Some(future_reminder)) },
        VersionedEvent { version: Version::new(2), event: Event::Completed },
        VersionedEvent { version: Version::new(3), event: Event::Uncompleted },
        VersionedEvent { version: Version::new(4), event: Event::ReminderUpdated(None) },
        VersionedEvent { version: Version::new(5), event: Event::TextUpdated(updated_description) },
        VersionedEvent { version: Version::new(6), event: Event::Completed },
    ];
    let actual_events_2 =
        cqrs::EventSource::read_events(&es, &agg_2, Since::BeginningOfStream).unwrap().unwrap();

    assert_eq!(actual_events_2, expected_events_2);
}
