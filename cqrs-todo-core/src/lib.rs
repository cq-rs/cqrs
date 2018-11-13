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
extern crate smallvec;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate log;

use smallvec::SmallVec;
use cqrs_core::{Aggregate, PersistableAggregate, SerializableEvent, EventDeserializeError};
use std::io;

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

    fn apply(&mut self, event: Event) {
        match event {
            Event::TextUpdated(description) => self.description = description,
            Event::ReminderUpdated(reminder) => self.reminder = reminder,
            Event::Completed => self.status = TodoStatus::Completed,
            Event::Uncompleted => self.status = TodoStatus::NotCompleted,
            Event::Created(_) => {},
        }
    }

    fn execute(&self, command: Command) -> Result<Events, error::CommandError> {
        match command {
            Command::UpdateText(description) => Ok(self.execute_update_description(description)),
            Command::SetReminder(reminder) => Ok(self.execute_set_reminder(reminder)),
            Command::CancelReminder => Ok(self.execute_cancel_reminder()),
            Command::ToggleCompletion => Ok(self.execute_toggle_completion()),
            Command::MarkCompleted => Ok(self.execute_mark_completed()),
            Command::ResetCompleted => Ok(self.execute_reset_completed()),
            Command::Create(ref description, ref reminder_opt) => self.execute_create(description, reminder_opt.as_ref()),
        }
    }

    fn execute_create(&self, description: &domain::Description, reminder_opt: Option<&domain::Reminder>) -> Result<Events, error::CommandError> {
        if &self.description == description && self.reminder.as_ref() == reminder_opt {
            Ok(SmallVec::new())
        } else {
            Err(error::CommandError::AlreadyCreated)
        }
    }

    fn execute_update_description(&self, description: domain::Description) -> Events {
        let mut events = SmallVec::new();
        if self.description != description {
            events.push(Event::TextUpdated(description));
        }
        events
    }

    fn execute_set_reminder(&self, reminder: domain::Reminder) -> Events {
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

    fn execute_cancel_reminder(&self) -> Events {
        let mut events = SmallVec::new();
        if self.reminder.is_some() {
            events.push(Event::ReminderUpdated(None));
        }
        events
    }

    fn execute_toggle_completion(&self) -> Events {
        let mut events = SmallVec::new();
        match self.status {
            TodoStatus::NotCompleted => events.push(Event::Completed),
            TodoStatus::Completed => events.push(Event::Uncompleted),
        }
        events
    }

    fn execute_mark_completed(&self) -> Events {
        let mut events = SmallVec::new();
        match self.status {
            TodoStatus::NotCompleted => events.push(Event::Completed),
            TodoStatus::Completed => {},
        }
        events
    }

    fn execute_reset_completed(&self) -> Events {
        let mut events = SmallVec::new();
        match self.status {
            TodoStatus::NotCompleted => {},
            TodoStatus::Completed => events.push(Event::Uncompleted),
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

type Events = SmallVec<[Event; 1]>;

impl TodoAggregate {
    fn apply_event(&mut self, event: Event) {
        match *self {
            TodoAggregate::Uninitialized => self.apply_to_uninitialized(event),
            TodoAggregate::Created(ref mut data) => data.apply(event),
        }
    }

    fn apply_to_uninitialized(&mut self, event: Event) {
        if let Event::Created(initial_description) = event {
            *self = TodoAggregate::Created(TodoData::with_description(initial_description));
        }
    }

    fn execute_command(&self, command: Command) -> Result<Events, error::CommandError> {
        match *self {
            TodoAggregate::Uninitialized => self.execute_on_uninitialized(command),
            TodoAggregate::Created(ref data) => data.execute(command),
        }
    }

    fn execute_on_uninitialized(&self, command: Command) -> Result<Events, error::CommandError> {
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
            TodoAggregate::Uninitialized => Err("uninitialized"),
            TodoAggregate::Created(ref x) => Ok(x),
        }
    }
}

impl Aggregate for TodoAggregate {
    type Event = Event;
    type Command = Command;
    type Events = Events;
    type Error = error::CommandError;

    fn apply(&mut self, event: Self::Event) {
        trace!("apply {:?}", event);
        self.apply_event(event);
    }

    fn execute(&self, command: Command) -> Result<Self::Events, Self::Error> {
        trace!("execute {:?}", command);
        self.execute_command(command)
    }

    #[inline(always)]
    fn entity_type() -> &'static str where Self: Sized {
        "todo"
    }
}

impl PersistableAggregate for TodoAggregate {
    type SnapshotError = serde_json::Error;

    fn snapshot_to_writer<W: io::Write>(&self, writer: W) -> io::Result<()> {
        serde_json::to_writer(writer, &self)?;
        Ok(())
    }

    fn restore(snapshot: &[u8]) -> Result<Self, <Self as PersistableAggregate>::SnapshotError> {
        Ok(serde_json::from_slice(snapshot)?)
    }

    fn restore_from_reader<R: io::Read>(reader: R) -> Result<Self, <Self as PersistableAggregate>::SnapshotError> {
        Ok(serde_json::from_reader(reader)?)
    }
}

impl SerializableEvent for Event {
    type PayloadError = serde_json::Error;

    fn event_type(&self) -> &'static str {
        match *self {
            Event::Created(_) => "todo_created",
            Event::ReminderUpdated(_) => "todo_reminder_updated",
            Event::TextUpdated(_) => "todo_text_updated",
            Event::Completed => "todo_completed",
            Event::Uncompleted => "todo_uncompleted",
        }
    }

    fn deserialize(event_type: &str, payload: &[u8]) -> Result<Self, EventDeserializeError<Self>> {
        let event = match event_type {
            "todo_created" => Event::Created(serde_json::from_slice(payload).map_err(EventDeserializeError::InvalidPayload)?),
            "todo_reminder_updated" => Event::ReminderUpdated(serde_json::from_slice(payload).map_err(EventDeserializeError::InvalidPayload)?),
            "todo_text_updated" => Event::TextUpdated(serde_json::from_slice(payload).map_err(EventDeserializeError::InvalidPayload)?),
            "todo_completed" => Event::Completed,
            "todo_uncompleted" => Event::Uncompleted,
            _ => Err(EventDeserializeError::new_unknown_event_type(event_type))?,
        };

        Ok(event)
    }

    fn deserialize_from_reader<R: io::Read>(event_type: &str, reader: R) -> Result<Self, EventDeserializeError<Self>> {
        let event = match event_type {
            "todo_created" => Event::Created(serde_json::from_reader(reader).map_err(EventDeserializeError::InvalidPayload)?),
            "todo_reminder_updated" => Event::ReminderUpdated(serde_json::from_reader(reader).map_err(EventDeserializeError::InvalidPayload)?),
            "todo_text_updated" => Event::TextUpdated(serde_json::from_reader(reader).map_err(EventDeserializeError::InvalidPayload)?),
            "todo_completed" => Event::Completed,
            "todo_uncompleted" => Event::Uncompleted,
            _ => Err(EventDeserializeError::new_unknown_event_type(event_type))?,
        };

        Ok(event)
    }

    fn serialize_to_writer<W: io::Write>(&self, writer: W) -> io::Result<()> {
        match *self {
            Event::Created(ref x) => serde_json::to_writer(writer, x)?,
            Event::ReminderUpdated(ref x) => serde_json::to_writer(writer, x)?,
            Event::TextUpdated(ref x) => serde_json::to_writer(writer, x)?,
            Event::Completed => serde_json::to_writer(writer, &())?,
            Event::Uncompleted => serde_json::to_writer(writer, &())?,
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use chrono::{Utc,TimeZone,Duration};

    fn create_basic_aggregate() -> TodoAggregate {
        let now = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder = now + Duration::seconds(10000);

        let mut events = SmallVec::<[Event;6]>::new();
        events.push(Event::Completed);
        events.push(Event::Created(domain::Description::new("Hello!").unwrap()));
        events.push(Event::ReminderUpdated(Some(domain::Reminder::new(reminder, now).unwrap())));
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
        let expected_state = TodoAggregate::Created(expected_data);

        let agg = create_basic_aggregate();

        assert_eq!(expected_state, agg);
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

        let now = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder_time = now + Duration::seconds(20000);
        let reminder = domain::Reminder::new(reminder_time, now).unwrap();
        let cmd = Command::SetReminder(reminder);

        let result = agg.execute(cmd).unwrap();

        let mut expected = Events::new();
        expected.push(Event::ReminderUpdated(Some(reminder)));
        assert_eq!(expected, result);
    }
}
