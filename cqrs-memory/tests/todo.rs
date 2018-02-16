extern crate cqrs;
extern crate cqrs_memory;
extern crate fnv;
extern crate smallvec;

use cqrs::trivial::NopEventDecorator;
use cqrs::{Since, VersionedEvent, Version};
use cqrs::domain::Aggregate;
use cqrs::domain::command::{DecoratedAggregateCommand, PersistAndSnapshotAggregateCommander};
use cqrs::domain::query::SnapshotPlusEventsAggregateView;
use cqrs::error::{CommandAggregateError, LoadAggregateError, PersistAggregateError, AppendEventsError, Never};
use cqrs_memory::{MemoryEventStore, MemoryStateStore};
use smallvec::SmallVec;

use std::time::{Duration, Instant};
use std::fmt;
use std::error;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    TextUpdated(String),
    ReminderUpdated(Option<Instant>),
    Completed,
    Uncompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SetReminderData {
    current_time: Instant,
    reminder_time: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    UpdateText(String),
    SetReminder(SetReminderData),
    CancelReminder,
    ToggleCompletion,
    MarkCompleted,
    ResetCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CommandError {
    InvalidText,
    ReminderTimeInPast,
}

impl error::Error for CommandError {
    fn description(&self) -> &str {
        match *self {
            CommandError::InvalidText => "invalid command text",
            CommandError::ReminderTimeInPast => "reminder time cannot be in the past",
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = self as &error::Error;
        f.write_str(err.description())
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum TodoStatus {
    Completed,
    NotCompleted,
}

impl Default for TodoStatus {
    fn default() -> Self {
        TodoStatus::NotCompleted
    }
}

#[derive(Debug, Clone, PartialEq)]
struct TodoState {
    event_count: usize,
    description: String,
    reminder: Option<Instant>,
    status: TodoStatus,
}

impl Default for TodoState {
    fn default() -> Self {
        println!("default");
        TodoState {
            event_count: 0,
            description: String::default(),
            reminder: None,
            status: TodoStatus::NotCompleted,
        }
    }
}

impl Aggregate for TodoState {
    type Events = SmallVec<[Self::Event;1]>;
    type Event = Event;
    type Snapshot = Self;
    type Command = Command;
    type CommandError = CommandError;

    fn from_snapshot(snapshot: Self::Snapshot) -> Self {
        snapshot
    }

    fn apply(&mut self, evt: Self::Event) {
        println!("apply {:?}", evt);
        self.event_count += 1;
        match evt {
            Event::TextUpdated(txt) => self.description = txt,
            Event::ReminderUpdated(r) => self.reminder = r,
            Event::Completed => self.status = TodoStatus::Completed,
            Event::Uncompleted => self.status = TodoStatus::NotCompleted,
        }
    }

    fn execute(&self, cmd: Self::Command) -> Result<Self::Events, Self::CommandError> {
        let mut events = SmallVec::new();
        println!("execute {:?}", cmd);
        match cmd {
            Command::UpdateText(txt) => {
                if txt.is_empty() {
                    return Err(CommandError::InvalidText);
                } else if txt != self.description {
                    events.push(Event::TextUpdated(txt));
                }
            }
            Command::SetReminder(rem) => {
                if rem.current_time >= rem.reminder_time {
                    return Err(CommandError::ReminderTimeInPast);
                } else {
                    match self.reminder {
                        Some(existing_time) if existing_time == rem.reminder_time => {},
                        _ => events.push(Event::ReminderUpdated(Some(rem.reminder_time))),
                    }
                }
            }
            Command::CancelReminder if self.reminder.is_some() => events.push(Event::ReminderUpdated(None)),
            Command::CancelReminder => {},
            Command::ToggleCompletion => {
                match self.status {
                    TodoStatus::Completed => events.push(Event::Uncompleted),
                    TodoStatus::NotCompleted => events.push(Event::Completed),
                }
            }
            Command::MarkCompleted => {
                match self.status {
                    TodoStatus::Completed => {},
                    TodoStatus::NotCompleted => events.push(Event::Completed),
                }
            }
            Command::ResetCompleted => {
                match self.status {
                    TodoStatus::Completed => events.push(Event::Uncompleted),
                    TodoStatus::NotCompleted => {},
                }
            }
        }
        Ok(events)
    }

    fn snapshot(self) -> Self::Snapshot {
        self
    }
}

#[test]
fn main_test() {
    let es = MemoryEventStore::<Event, usize, fnv::FnvBuildHasher>::default();
    //let es = okazis::NullEventStore::<Event, usize>::default();
    let ss = MemoryStateStore::<TodoState, usize, fnv::FnvBuildHasher>::default();
    //let ss = okazis::NullStateStore::<TodoState, usize>::default();

    let view = SnapshotPlusEventsAggregateView::new(&es, &ss);
    let command_view = SnapshotPlusEventsAggregateView::new(&es, &ss);
    let command = PersistAndSnapshotAggregateCommander::new(command_view, &es, &ss);

    let command =
        &command as &DecoratedAggregateCommand<TodoState, NopEventDecorator<Event>, AggregateId=usize, Error=CommandAggregateError<CommandError, LoadAggregateError<Never, Never>, PersistAggregateError<AppendEventsError<Never>, Never>>>;

    let agg_1 = 0;
    let agg_2 = 34;

    let now = Instant::now();
    let duration = Duration::from_secs(1000);
    let past_time = now - duration;
    let future_time = now + duration;

    let decorator = NopEventDecorator::<Event>::default();

    command.execute_with_decorator(&agg_1, Command::UpdateText("Hello world!".to_string()), decorator).unwrap();
    println!("0: {:#?}", view);
    command.execute_with_decorator(&agg_2, Command::SetReminder(SetReminderData { current_time: now, reminder_time: future_time }), decorator).unwrap();
    println!("1: {:#?}", view);
    command.execute_with_decorator(&agg_2, Command::ToggleCompletion, decorator).unwrap();
    println!("2: {:#?}", view);
    command.execute_with_decorator(&agg_2, Command::MarkCompleted, decorator).unwrap();
    println!("3: {:#?}", view);
    command.execute_with_decorator(&agg_2, Command::ResetCompleted, decorator).unwrap();
    println!("4: {:#?}", view);
    let err = command.execute_with_decorator(&agg_2, Command::SetReminder(SetReminderData { current_time: now, reminder_time: past_time }), decorator).unwrap_err();
    println!("err: {:?}", err);
    println!("5: {:#?}", view);
    command.execute_with_decorator(&agg_2, Command::CancelReminder, decorator).unwrap();
    println!("6: {:#?}", view);
    command.execute_with_decorator(&agg_2, Command::UpdateText("Complete CQRS!".to_string()), decorator).unwrap();
    println!("7: {:#?}", view);
    command.execute_with_decorator(&agg_2, Command::MarkCompleted, decorator).unwrap();
    println!("8: {:#?}", view);
    command.execute_with_decorator(&agg_2, Command::MarkCompleted, decorator).unwrap();
    println!("9: {:#?}", view);

    let expected_events = vec![
        VersionedEvent { version: Version::new(0), event: Event::ReminderUpdated(Some(future_time)) },
        VersionedEvent { version: Version::new(1), event: Event::Completed },
        VersionedEvent { version: Version::new(2), event: Event::Uncompleted },
        VersionedEvent { version: Version::new(3), event: Event::ReminderUpdated(None) },
        VersionedEvent { version: Version::new(4), event: Event::TextUpdated("Complete CQRS!".to_string()) },
        VersionedEvent { version: Version::new(5), event: Event::Completed },
    ];
    let actual_events =
        cqrs::EventSource::read_events(&es, &agg_2, Since::BeginningOfStream).unwrap().unwrap();

    assert_eq!(actual_events, expected_events);
    println!("---DONE---");
}
