extern crate cqrs;
extern crate smallvec;

use smallvec::SmallVec;
use cqrs::domain::{Aggregate, RestoreAggregate, SnapshotAggregate};

pub mod domain {
    use std::time::Instant;
    use error::{InvalidDescription, InvalidReminderTime};
    use std::borrow::Borrow;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Reminder {
        time: Instant,
    }

    impl Reminder {
        pub fn new(reminder_time: Instant, current_time: Instant) -> Result<Reminder, InvalidReminderTime> {
            if reminder_time <= current_time {
                Err(InvalidReminderTime)
            } else {
                Ok(Reminder {
                    time: reminder_time,
                })
            }
        }

        pub fn time(&self) -> Instant {
            self.time
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Description {
        text: String,
    }

    impl Description {
        pub fn new<S: Into<String>>(text: S) -> Result<Description, InvalidDescription> {
            let text = text.into();
            if text.is_empty() {
                Err(InvalidDescription)
            } else {
                Ok(Description {
                    text,
                })
            }
        }

        pub fn as_str(&self) -> &str {
            self.text.as_str()
        }
    }

    impl Borrow<str> for Description {
        fn borrow(&self) -> &str {
            self.text.borrow()
        }
    }
}

pub mod error {
    use std::error;
    use std::fmt;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct InvalidReminderTime;

    impl error::Error for InvalidReminderTime {
        fn description(&self) -> &str {
            "reminder time cannot be in the past"
        }
    }

    impl fmt::Display for InvalidReminderTime {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let err = self as &error::Error;
            f.write_str(err.description())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct InvalidDescription;

    impl error::Error for InvalidDescription {
        fn description(&self) -> &str {
            "description cannot be empty"
        }
    }

    impl fmt::Display for InvalidDescription {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let err = self as &error::Error;
            f.write_str(err.description())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum CommandError {
        NotInitialized,
        AlreadyCreated,
    }

    impl error::Error for CommandError {
        fn description(&self) -> &str {
            match *self {
                CommandError::NotInitialized => "attempt to execute command before creation",
                CommandError::AlreadyCreated => "attempt to create when already created",
            }
        }
    }

    impl fmt::Display for CommandError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let err = self as &error::Error;
            f.write_str(err.description())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Created(domain::Description),
    TextUpdated(domain::Description),
    ReminderUpdated(Option<domain::Reminder>),
    Completed,
    Uncompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Create(domain::Description, Option<domain::Reminder>),
    UpdateText(domain::Description),
    SetReminder(domain::Reminder),
    CancelReminder,
    ToggleCompletion,
    MarkCompleted,
    ResetCompleted,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TodoStatus {
    Completed,
    NotCompleted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TodoData {
    pub description: domain::Description,
    pub reminder: Option<domain::Reminder>,
    pub status: TodoStatus,
}

impl TodoData {
    pub fn new(description: domain::Description, reminder: Option<domain::Reminder>, status: TodoStatus) -> Self {
        TodoData {
            description, reminder, status,
        }
    }

    pub fn with_description(description: domain::Description) -> Self {
        TodoData {
            description,
            reminder: None,
            status: TodoStatus::NotCompleted
        }
    }

    pub fn apply(&mut self, event: Event) {
        match event {
            Event::TextUpdated(description) => self.description = description,
            Event::ReminderUpdated(reminder) => self.reminder = reminder,
            Event::Completed => self.status = TodoStatus::Completed,
            Event::Uncompleted => self.status = TodoStatus::NotCompleted,
            Event::Created(_) => {},
        }
    }

    pub fn execute(&self, command: Command) -> Result<Events, error::CommandError> {
        match command {
            Command::UpdateText(description) => Ok(self.execute_update_description(description)),
            Command::SetReminder(reminder) => Ok(self.execute_set_reminder(reminder)),
            Command::CancelReminder => Ok(self.execute_cancel_reminder()),
            Command::ToggleCompletion => Ok(self.execute_toggle_completion()),
            Command::MarkCompleted => Ok(self.execute_mark_completed()),
            Command::ResetCompleted => Ok(self.execute_reset_completed()),
            Command::Create(description, reminder_opt) => self.execute_create(description, reminder_opt),
        }
    }

    pub fn execute_create(&self, description: domain::Description, reminder_opt: Option<domain::Reminder>) -> Result<Events, error::CommandError> {
        if self.description == description && self.reminder == reminder_opt {
            Ok(SmallVec::new())
        } else {
            Err(error::CommandError::AlreadyCreated)
        }
    }

    pub fn execute_update_description(&self, description: domain::Description) -> Events {
        let mut events = SmallVec::new();
        if self.description != description {
            events.push(Event::TextUpdated(description));
        }
        events
    }

    pub fn execute_set_reminder(&self, reminder: domain::Reminder) -> Events {
        let mut events = SmallVec::new();
        if let Some(existing) = self.reminder {
            if existing != reminder {
                events.push(Event::ReminderUpdated(Some(reminder)));
            }
        } else {
            events.push(Event::ReminderUpdated(Some(reminder)));
        }
        events
    }

    pub fn execute_cancel_reminder(&self) -> Events {
        let mut events = SmallVec::new();
        if self.reminder.is_some() {
            events.push(Event::ReminderUpdated(None));
        }
        events
    }

    pub fn execute_toggle_completion(&self) -> Events {
        let mut events = SmallVec::new();
        match self.status {
            TodoStatus::NotCompleted => events.push(Event::Completed),
            TodoStatus::Completed => events.push(Event::Uncompleted),
        }
        events
    }

    pub fn execute_mark_completed(&self) -> Events {
        let mut events = SmallVec::new();
        match self.status {
            TodoStatus::NotCompleted => events.push(Event::Completed),
            TodoStatus::Completed => {},
        }
        events
    }

    pub fn execute_reset_completed(&self) -> Events {
        let mut events = SmallVec::new();
        match self.status {
            TodoStatus::NotCompleted => {},
            TodoStatus::Completed => events.push(Event::Uncompleted),
        }
        events
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TodoState {
    Uninitialized,
    Created(TodoData),
}

impl Default for TodoState {
    fn default() -> Self {
        TodoState::Uninitialized
    }
}

type Events = <TodoAggregate as Aggregate>::Events;

impl TodoState {
    pub fn apply(&mut self, event: Event) {
        match *self {
            TodoState::Uninitialized => self.apply_to_uninitialized(event),
            TodoState::Created(ref mut data) => data.apply(event),
        }
    }

    pub fn apply_to_uninitialized(&mut self, event: Event) {
        match event {
            Event::Created(initial_description) =>
                *self = TodoState::Created(TodoData::with_description(initial_description)),
            _ => {}
        }
    }

    pub fn execute(&self, command: Command) -> Result<Events, error::CommandError> {
        match *self {
            TodoState::Uninitialized => self.execute_on_uninitialized(command),
            TodoState::Created(ref data) => data.execute(command),
        }
    }

    pub fn execute_on_uninitialized(&self, command: Command) -> Result<Events, error::CommandError> {
        match command {
            Command::Create(description, reminder_opt) => {
                let mut events = SmallVec::new();
                events.push(Event::Created(description));
                if let Some(reminder) = reminder_opt {
                    events.push(Event::ReminderUpdated(Some(reminder)));
                }
                Ok(events)
            }
            _ => Err(error::CommandError::NotInitialized),
        }
    }

    pub fn get_data(&self) -> Result<&TodoData, &'static str> {
        match *self {
            TodoState::Uninitialized => Err("uninitialized"),
            TodoState::Created(ref x) => Ok(x),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct TodoAggregate {
    applied_event_count: usize,
    state: TodoState,
}

impl TodoAggregate {
    pub fn inspect_state(&self) -> &TodoState {
        &self.state
    }

    pub fn applied_events(&self) -> usize {
        self.applied_event_count
    }
}

impl Aggregate for TodoAggregate {
    type Events = SmallVec<[Self::Event;1]>;
    type Event = Event;
    type Command = Command;
    type CommandError = error::CommandError;

    fn apply(&mut self, event: Self::Event) {
        println!("apply {:?}", event);
        self.applied_event_count += 1;
        self.state.apply(event);
    }

    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::CommandError> {
        println!("execute {:?}", command);
        self.state.execute(command)
    }
}

impl RestoreAggregate for TodoAggregate {
    type Snapshot = TodoState;

    fn restore(snapshot: Self::Snapshot) -> Self {
        TodoAggregate {
            applied_event_count: 0,
            state: snapshot,
        }
    }
}

impl SnapshotAggregate for TodoAggregate {
    type Snapshot = TodoState;

    fn to_snapshot(self) -> Self::Snapshot {
        self.state
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use std::time::{Duration, Instant};

    fn create_basic_aggregate() -> TodoAggregate {
        let time_0 = Instant::now();
        let time_1 = time_0 + Duration::from_secs(10000);

        let mut events = SmallVec::<[Event;6]>::new();
        events.push(Event::Completed);
        events.push(Event::Created(domain::Description::new("Hello!").unwrap()));
        events.push(Event::ReminderUpdated(Some(domain::Reminder::new(time_1, time_0).unwrap())));
        events.push(Event::TextUpdated(domain::Description::new("New text").unwrap()));
        events.push(Event::Created(domain::Description::new("Ignored!").unwrap()));
        events.push(Event::ReminderUpdated(None));

        let mut agg = TodoAggregate::default();
        for event in events {
            agg.apply(event);
        }
        agg
    }

    #[test]
    fn example_event_sequence() {
        let expected_data = TodoData {
            description: domain::Description::new("New text").unwrap(),
            reminder: None,
            status: TodoStatus::NotCompleted,
        };
        let expected_state = TodoState::Created(expected_data);

        let agg = create_basic_aggregate();

        assert_eq!(&expected_state, agg.inspect_state());
    }

    #[test]
    fn cancel_reminder_on_default_aggregate() {
        let agg = TodoAggregate::default();

        let cmd = Command::CancelReminder;

        let result = agg.execute(cmd).unwrap_err();

        assert_eq!(error::CommandError::NotInitialized, result);
    }

    #[test]
    fn cancel_reminder_on_basic_aggregate() {
        let agg = create_basic_aggregate();

        let cmd = Command::CancelReminder;

        let result = agg.execute(cmd).unwrap();

        assert_eq!(Events::new(), result);
    }

    #[test]
    fn set_reminder_on_basic_aggregate() {
        let agg = create_basic_aggregate();

        let time_0 = Instant::now();
        let time_1 = time_0 + Duration::from_secs(20000);
        let reminder = domain::Reminder::new(time_1, time_0).unwrap();
        let cmd = Command::SetReminder(reminder);

        let result = agg.execute(cmd).unwrap();

        let mut expected = Events::new();
        expected.push(Event::ReminderUpdated(Some(reminder)));
        assert_eq!(expected, result);
    }
}
