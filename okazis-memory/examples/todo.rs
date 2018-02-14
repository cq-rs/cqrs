extern crate okazis;
extern crate okazis_memory;
extern crate fnv;

use okazis::{CommandResult, Aggregate, SnapshotDecision};
use okazis::trivial::NopEventDecorator;
use okazis_memory::{MemoryEventStore, MemoryStateStore};

use std::time::{Duration, Instant};

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
    type Event = Event;
    type Command = Command;
    type CommandError = CommandError;

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

    fn execute(&self, cmd: Self::Command) -> CommandResult<Self::Event, Self::CommandError> {
        println!("execute {:?}", cmd);
        match cmd {
            Command::UpdateText(txt) => {
                if txt.is_empty() {
                    Err(CommandError::InvalidText)
                } else if txt != self.description {
                    Ok(vec![Event::TextUpdated(txt)])
                } else {
                    Ok(Vec::default())
                }
            }
            Command::SetReminder(rem) => {
                if rem.current_time >= rem.reminder_time {
                    Err(CommandError::ReminderTimeInPast)
                } else {
                    match self.reminder {
                        Some(existing_time) if existing_time == rem.reminder_time => Ok(Vec::default()),
                        _ => Ok(vec![Event::ReminderUpdated(Some(rem.reminder_time))]),
                    }
                }
            }
            Command::CancelReminder => {
                if self.reminder.is_some() {
                    Ok(vec![Event::ReminderUpdated(None)])
                } else {
                    Ok(Vec::default())
                }
            }
            Command::ToggleCompletion => {
                match self.status {
                    TodoStatus::Completed => Ok(vec![Event::Uncompleted]),
                    TodoStatus::NotCompleted => Ok(vec![Event::Completed]),
                }
            }
            Command::MarkCompleted => {
                match self.status {
                    TodoStatus::Completed => Ok(Vec::default()),
                    TodoStatus::NotCompleted => Ok(vec![Event::Completed]),
                }
            }
            Command::ResetCompleted => {
                match self.status {
                    TodoStatus::Completed => Ok(vec![Event::Uncompleted]),
                    TodoStatus::NotCompleted => Ok(Vec::default()),
                }
            }
        }
    }

    fn should_snapshot(&self) -> SnapshotDecision {
        if self.event_count % 4 == 0 {
            SnapshotDecision::Persist
        } else {
            SnapshotDecision::Skip
        }
    }
}

fn main() {
    let es = MemoryEventStore::<Event, usize, fnv::FnvBuildHasher>::default();
    //let es = okazis::NullEventStore::<Event, usize>::default();
    let ss = MemoryStateStore::<TodoState, usize, fnv::FnvBuildHasher>::default();
    //let ss = okazis::NullStateStore::<TodoState, usize>::default();
    let agg_store = okazis::AggregateStore::new(es, ss);

    let agg_1 = 0;
    let agg_2 = 34;

    let now = Instant::now();
    let duration = Duration::from_secs(1000);
    let past_time = now - duration;
    let future_time = now + duration;

    agg_store.execute_and_persist(&agg_1, Command::UpdateText("Hello world!".to_string()), NopEventDecorator::default()).unwrap();
    println!("0: {:#?}", agg_store);
    agg_store.execute_and_persist(&agg_2, Command::SetReminder(SetReminderData { current_time: now, reminder_time: future_time }), NopEventDecorator::default()).unwrap();
    println!("1: {:#?}", agg_store);
    agg_store.execute_and_persist(&agg_2, Command::ToggleCompletion, NopEventDecorator::default()).unwrap();
    println!("2: {:#?}", agg_store);
    agg_store.execute_and_persist(&agg_2, Command::MarkCompleted, NopEventDecorator::default()).unwrap();
    println!("3: {:#?}", agg_store);
    agg_store.execute_and_persist(&agg_2, Command::ResetCompleted, NopEventDecorator::default()).unwrap();
    println!("4: {:#?}", agg_store);
    let err = agg_store.execute_and_persist(&agg_2, Command::SetReminder(SetReminderData { current_time: now, reminder_time: past_time }), NopEventDecorator::default()).unwrap_err();
    println!("err: {:?}", err);
    println!("5: {:#?}", agg_store);
    agg_store.execute_and_persist(&agg_2, Command::UpdateText("Complete CQRS!".to_string()), NopEventDecorator::default()).unwrap();
    println!("6: {:#?}", agg_store);
    agg_store.execute_and_persist(&agg_2, Command::MarkCompleted, NopEventDecorator::default()).unwrap();
    println!("7: {:#?}", agg_store);
    agg_store.execute_and_persist(&agg_2, Command::MarkCompleted, NopEventDecorator::default()).unwrap();
    println!("8: {:#?}", agg_store);
    println!("---DONE---");
}
