#![warn(
    unused_import_braces,
    unused_imports,
    unused_qualifications,
    missing_docs,
)]

#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
)]

extern crate cqrs_core;
extern crate chrono;
extern crate arrayvec;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate log;

use arrayvec::ArrayVec;
use cqrs_core::{Aggregate, Event};

pub mod domain {
    use chrono::{DateTime,Utc};
    use error::{InvalidDescription, InvalidReminderTime};
    use std::borrow::Borrow;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Reminder {
        time: DateTime<Utc>,
    }

    impl Reminder {
        pub fn new(reminder_time: DateTime<Utc>, current_time: DateTime<Utc>) -> Result<Reminder, InvalidReminderTime> {
            if reminder_time <= current_time {
                Err(InvalidReminderTime)
            } else {
                Ok(Reminder {
                    time: reminder_time,
                })
            }
        }

        pub fn get_time(&self) -> DateTime<Utc> {
            self.time
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            self.text.borrow()
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
            let err: &error::Error = self;
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
            let err: &error::Error = self;
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
            let err: &error::Error = self;
            f.write_str(err.description())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoMetadata {
    pub initiated_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoEvent {
    Created(domain::Description),
    TextUpdated(domain::Description),
    ReminderUpdated(Option<domain::Reminder>),
    Completed,
    Uncompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TodoCommand {
    Create(domain::Description, Option<domain::Reminder>),
    UpdateText(domain::Description),
    SetReminder(domain::Reminder),
    CancelReminder,
    ToggleCompletion,
    MarkCompleted,
    ResetCompleted,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoStatus {
    Completed,
    NotCompleted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoData {
    pub description: domain::Description,
    pub reminder: Option<domain::Reminder>,
    pub status: TodoStatus,
}

impl TodoData {
    fn new(description: domain::Description, reminder: Option<domain::Reminder>, status: TodoStatus) -> Self {
        TodoData {
            description, reminder, status,
        }
    }

    fn with_description(description: domain::Description) -> Self {
        TodoData {
            description,
            reminder: None,
            status: TodoStatus::NotCompleted
        }
    }

    fn apply(&mut self, event: TodoEvent) {
        match event {
            TodoEvent::TextUpdated(description) => self.description = description,
            TodoEvent::ReminderUpdated(reminder) => self.reminder = reminder,
            TodoEvent::Completed => self.status = TodoStatus::Completed,
            TodoEvent::Uncompleted => self.status = TodoStatus::NotCompleted,
            TodoEvent::Created(_) => {},
        }
    }

    fn execute(&self, command: TodoCommand) -> Result<Events, error::CommandError> {
        match command {
            TodoCommand::UpdateText(description) => Ok(self.execute_update_description(description)),
            TodoCommand::SetReminder(reminder) => Ok(self.execute_set_reminder(reminder)),
            TodoCommand::CancelReminder => Ok(self.execute_cancel_reminder()),
            TodoCommand::ToggleCompletion => Ok(self.execute_toggle_completion()),
            TodoCommand::MarkCompleted => Ok(self.execute_mark_completed()),
            TodoCommand::ResetCompleted => Ok(self.execute_reset_completed()),
            TodoCommand::Create(ref description, ref reminder_opt) => self.execute_create(description, reminder_opt.as_ref()),
        }
    }

    fn execute_create(&self, description: &domain::Description, reminder_opt: Option<&domain::Reminder>) -> Result<Events, error::CommandError> {
        if &self.description == description && self.reminder.as_ref() == reminder_opt {
            Ok(Events::new())
        } else {
            Err(error::CommandError::AlreadyCreated)
        }
    }

    fn execute_update_description(&self, description: domain::Description) -> Events {
        let mut events = Events::new();
        if self.description != description {
            events.push(TodoEvent::TextUpdated(description));
        }
        events
    }

    fn execute_set_reminder(&self, reminder: domain::Reminder) -> Events {
        let mut events = Events::new();
        if let Some(existing) = self.reminder {
            if existing != reminder {
                events.push(TodoEvent::ReminderUpdated(Some(reminder)));
            }
        } else {
            events.push(TodoEvent::ReminderUpdated(Some(reminder)));
        }
        events
    }

    fn execute_cancel_reminder(&self) -> Events {
        let mut events = Events::new();
        if self.reminder.is_some() {
            events.push(TodoEvent::ReminderUpdated(None));
        }
        events
    }

    fn execute_toggle_completion(&self) -> Events {
        let mut events = Events::new();
        match self.status {
            TodoStatus::NotCompleted => events.push(TodoEvent::Completed),
            TodoStatus::Completed => events.push(TodoEvent::Uncompleted),
        }
        events
    }

    fn execute_mark_completed(&self) -> Events {
        let mut events = Events::new();
        match self.status {
            TodoStatus::NotCompleted => events.push(TodoEvent::Completed),
            TodoStatus::Completed => {},
        }
        events
    }

    fn execute_reset_completed(&self) -> Events {
        let mut events = Events::new();
        match self.status {
            TodoStatus::NotCompleted => {},
            TodoStatus::Completed => events.push(TodoEvent::Uncompleted),
        }
        events
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TodoAggregate {
    Created(TodoData),
    Uninitialized,
}

impl Default for TodoAggregate {
    fn default() -> Self {
        TodoAggregate::Uninitialized
    }
}

type Events = ArrayVec<[TodoEvent; 2]>;

impl TodoAggregate {
    fn apply_event(&mut self, event: TodoEvent) {
        match *self {
            TodoAggregate::Uninitialized => self.apply_to_uninitialized(event),
            TodoAggregate::Created(ref mut data) => data.apply(event),
        }
    }

    fn apply_to_uninitialized(&mut self, event: TodoEvent) {
        if let TodoEvent::Created(initial_description) = event {
            *self = TodoAggregate::Created(TodoData::with_description(initial_description));
        }
    }

    fn execute_command(&self, command: TodoCommand) -> Result<Events, error::CommandError> {
        match *self {
            TodoAggregate::Uninitialized => self.execute_on_uninitialized(command),
            TodoAggregate::Created(ref data) => data.execute(command),
        }
    }

    fn execute_on_uninitialized(&self, command: TodoCommand) -> Result<Events, error::CommandError> {
        match command {
            TodoCommand::Create(description, reminder_opt) => {
                let mut events = Events::new();
                events.push(TodoEvent::Created(description));
                if let Some(reminder) = reminder_opt {
                    events.push(TodoEvent::ReminderUpdated(Some(reminder)));
                }
                Ok(events)
            }
            _ => Err(error::CommandError::NotInitialized),
        }
    }

    pub fn get_data(&self) -> Result<&TodoData, &'static str> {
        match *self {
            TodoAggregate::Uninitialized => Err("uninitialized"),
            TodoAggregate::Created(ref x) => Ok(x),
        }
    }
}

impl Aggregate for TodoAggregate {
    type Event = TodoEvent;
    type Command = TodoCommand;
    type Events = Events;
    type Error = error::CommandError;

    fn apply(&mut self, event: Self::Event) {
        log::trace!("apply {:?}", event);
        self.apply_event(event);
    }

    fn execute(&self, command: TodoCommand) -> Result<Self::Events, Self::Error> {
        log::trace!("execute {:?}", command);
        self.execute_command(command)
    }

    #[inline(always)]
    fn entity_type() -> &'static str where Self: Sized {
        "todo"
    }
}

impl Event for TodoEvent {
    fn event_type(&self) -> &'static str {
        match *self {
            TodoEvent::Created(_) => "todo_created",
            TodoEvent::ReminderUpdated(_) => "todo_reminder_updated",
            TodoEvent::TextUpdated(_) => "todo_text_updated",
            TodoEvent::Completed => "todo_completed",
            TodoEvent::Uncompleted => "todo_uncompleted",
        }
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use chrono::{Utc,TimeZone,Duration};

    fn create_basic_aggregate() -> TodoAggregate {
        let now = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder = now + Duration::seconds(10000);

        let events = ArrayVec::from([
            TodoEvent::Completed,
            TodoEvent::Created(domain::Description::new("Hello!").unwrap()),
            TodoEvent::ReminderUpdated(Some(domain::Reminder::new(reminder, now).unwrap())),
            TodoEvent::TextUpdated(domain::Description::new("New text").unwrap()),
            TodoEvent::Created(domain::Description::new("Ignored!").unwrap()),
            TodoEvent::ReminderUpdated(None),
        ]);

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
        let expected_state = TodoAggregate::Created(expected_data);

        let agg = create_basic_aggregate();

        assert_eq!(expected_state, agg);
    }

    #[test]
    fn cancel_reminder_on_default_aggregate() {
        let agg = TodoAggregate::default();

        let cmd = TodoCommand::CancelReminder;

        let result = agg.execute(cmd).unwrap_err();

        assert_eq!(error::CommandError::NotInitialized, result);
    }

    #[test]
    fn cancel_reminder_on_basic_aggregate() {
        let agg = create_basic_aggregate();

        let cmd = TodoCommand::CancelReminder;

        let result = agg.execute(cmd).unwrap();

        assert_eq!(Events::new(), result);
    }

    #[test]
    fn set_reminder_on_basic_aggregate() {
        let agg = create_basic_aggregate();

        let now = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder_time = now + Duration::seconds(20000);
        let reminder = domain::Reminder::new(reminder_time, now).unwrap();
        let cmd = TodoCommand::SetReminder(reminder);

        let result = agg.execute(cmd).unwrap();

        let mut expected = Events::new();
        expected.push(TodoEvent::ReminderUpdated(Some(reminder)));
        assert_eq!(expected, result);
    }
}
