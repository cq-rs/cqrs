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
#[cfg(test)]
extern crate cqrs_proptest;
extern crate chrono;
extern crate arrayvec;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
extern crate pretty_assertions;
#[cfg(test)]
extern crate proptest;
extern crate log;

use arrayvec::ArrayVec;
use cqrs_core::{Aggregate, Event, SerializableEvent, DeserializableEvent};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl SerializableEvent for TodoEvent {
    type Error = serde_json::Error;

    fn serialize_event_to_buffer(&self, buffer: &mut Vec<u8>) -> Result<(), Self::Error> {
        buffer.clear();
        buffer.reserve(128);
        match *self {
            TodoEvent::Created(ref inner) => {
                serde_json::to_writer(buffer, inner)?;
            },
            TodoEvent::ReminderUpdated(ref inner) => {
                serde_json::to_writer(buffer, inner)?;
            },
            TodoEvent::TextUpdated(ref inner) => {
                serde_json::to_writer(buffer, inner)?;
            },
            TodoEvent::Completed => {
                *buffer = vec![b'{', b'}'];
            },
            TodoEvent::Uncompleted => {
                *buffer = vec![b'{', b'}'];
            },
        }
        Ok(())
    }
}

impl DeserializableEvent for TodoEvent {
    type Error = serde_json::Error;

    fn deserialize_event_from_buffer(data: &[u8], event_type: &str) -> Result<Option<Self>, Self::Error> {
        match event_type {
            "todo_created" => Ok(Some(TodoEvent::Created(serde_json::from_slice(data)?))),
            "todo_reminder_updated" => Ok(Some(TodoEvent::ReminderUpdated(serde_json::from_slice(data)?))),
            "todo_text_updated" => Ok(Some(TodoEvent::TextUpdated(serde_json::from_slice(data)?))),
            "todo_completed" => Ok(Some(TodoEvent::Completed)),
            "todo_uncompleted" => Ok(Some(TodoEvent::Uncompleted)),
            _ => Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use chrono::{Utc,TimeZone,Duration};
    use pretty_assertions::assert_eq;

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

    #[test]
    fn ensure_created_event_stays_same() -> Result<(), serde_json::Error> {
        let description = domain::Description::new("test description").unwrap();
        run_snapshot_test("created_event", TodoEvent::Created(description))
    }

    #[test]
    fn ensure_reminder_updated_event_stays_same() -> Result<(), serde_json::Error> {
        let current_time = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);
        let reminder_time = Utc.ymd(2100, 1, 1).and_hms(0, 0, 0);
        let reminder = domain::Reminder::new(reminder_time, current_time).unwrap();
        run_snapshot_test("reminder_updated_event", TodoEvent::ReminderUpdated(Some(reminder)))
    }

    #[test]
    fn ensure_reminder_removed_event_stays_same() -> Result<(), serde_json::Error> {
        run_snapshot_test("reminder_updated_none_event", TodoEvent::ReminderUpdated(None))
    }

    #[test]
    fn ensure_text_updated_event_stays_same() -> Result<(), serde_json::Error> {
        let new_description = domain::Description::new("alt test description").unwrap();
        run_snapshot_test("text_updated_event", TodoEvent::TextUpdated(new_description))
    }

    #[test]
    fn ensure_completed_event_stays_same() -> Result<(), serde_json::Error> {
        run_snapshot_test("completed_event", TodoEvent::Completed)
    }

    #[test]
    fn ensure_uncompleted_event_stays_same() -> Result<(), serde_json::Error> {
        run_snapshot_test("uncompleted_event", TodoEvent::Uncompleted)
    }

    fn run_snapshot_test<E: SerializableEvent>(name: &'static str, event: E) -> Result<(), E::Error> {
        let mut buffer = Vec::default();
        event.serialize_event_to_buffer(&mut buffer)?;

        #[derive(Serialize)]
        struct RawEventWithType {
            event_type: &'static str,
            raw: String,
        }

        let data = RawEventWithType {
            event_type: event.event_type(),
            raw: String::from_utf8(buffer).unwrap(),
        };

        insta::assert_json_snapshot_matches!(name, data);
        Ok(())
    }

    #[test]
    fn roundtrip_created() -> Result<(), serde_json::Error> {
        let original = TodoEvent::Created(domain::Description::new("test description").unwrap());
        let mut buffer = Vec::default();
        original.serialize_event_to_buffer(&mut buffer)?;

        let roundtrip = TodoEvent::deserialize_event_from_buffer(&buffer, original.event_type())?;
        assert_eq!(Some(original), roundtrip);

        Ok(())
    }

    #[test]
    fn roundtrip_reminder_updated() -> Result<(), serde_json::Error> {
        let original = TodoEvent::ReminderUpdated(Some(domain::Reminder::new(Utc.ymd(2100, 1, 1).and_hms(0, 0, 0), Utc.ymd(2000, 1, 1).and_hms(0, 0, 0)).unwrap()));
        let mut buffer = Vec::default();
        original.serialize_event_to_buffer(&mut buffer)?;

        let roundtrip = TodoEvent::deserialize_event_from_buffer(&buffer, original.event_type())?;
        assert_eq!(Some(original), roundtrip);

        Ok(())
    }

    #[test]
    fn roundtrip_reminder_updated_none() -> Result<(), serde_json::Error> {
        let original = TodoEvent::ReminderUpdated(None);
        let mut buffer = Vec::default();
        original.serialize_event_to_buffer(&mut buffer)?;

        let roundtrip = TodoEvent::deserialize_event_from_buffer(&buffer, original.event_type())?;
        assert_eq!(Some(original), roundtrip);

        Ok(())
    }

    #[test]
    fn roundtrip_text_updated() -> Result<(), serde_json::Error> {
        let original = TodoEvent::TextUpdated(domain::Description::new("alt test description").unwrap());
        let mut buffer = Vec::default();
        original.serialize_event_to_buffer(&mut buffer)?;

        let roundtrip = TodoEvent::deserialize_event_from_buffer(&buffer, original.event_type())?;
        assert_eq!(Some(original), roundtrip);

        Ok(())
    }

    #[test]
    fn roundtrip_completed() -> Result<(), serde_json::Error> {
        let original = TodoEvent::Completed;
        let mut buffer = Vec::default();
        original.serialize_event_to_buffer(&mut buffer)?;

        let roundtrip = TodoEvent::deserialize_event_from_buffer(&buffer, original.event_type())?;
        assert_eq!(Some(original), roundtrip);

        Ok(())
    }

    #[test]
    fn roundtrip_uncompleted() -> Result<(), serde_json::Error> {
        let original = TodoEvent::Uncompleted;
        let mut buffer = Vec::default();
        original.serialize_event_to_buffer(&mut buffer)?;

        let roundtrip = TodoEvent::deserialize_event_from_buffer(&buffer, original.event_type())?;
        assert_eq!(Some(original), roundtrip);

        Ok(())
    }

    mod property_tests {
        use super::*;
        use cqrs_core::{Aggregate, Event};
        use cqrs_proptest::AggregateFromEventSequence;
        use proptest::prelude::*;
        use proptest::{proptest_helper, prop_oneof, proptest};
        use pretty_assertions::assert_eq;
        use std::fmt;

        impl Arbitrary for domain::Description {
            type Parameters = proptest::string::StringParam;

            fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
                let s: &'static str = args.into();
                s.prop_filter_map("invalid description", |d| {
                    domain::Description::new(d).ok()
                }).boxed()
            }

            type Strategy = BoxedStrategy<Self>;
        }

        impl Arbitrary for domain::Reminder {
            type Parameters = ();

            fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
                let current_time = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);

                (2000..2500_i32, 1..=366_u32, 0..86400_u32).prop_filter_map("invalid date", move |(y, o, s)| {
                    let time = chrono::NaiveTime::from_num_seconds_from_midnight(s, 0);
                    let date = Utc.yo_opt(y, o).single()?.and_time(time)?;
                    domain::Reminder::new(date, current_time).ok()
                }).boxed()
            }

            type Strategy = BoxedStrategy<Self>;
        }

        impl Arbitrary for TodoEvent {
            type Parameters = ();

            fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
                prop_oneof! [
                    any::<domain::Description>().prop_map(TodoEvent::Created),
                    any::<Option<domain::Reminder>>().prop_map(TodoEvent::ReminderUpdated),
                    any::<domain::Description>().prop_map(TodoEvent::TextUpdated),
                    Just(TodoEvent::Completed),
                    Just(TodoEvent::Uncompleted),
                ].boxed()
            }

            type Strategy = BoxedStrategy<Self>;
        }

        fn verify_serializable_roundtrips_through_serialization<V: serde::Serialize + for<'de> serde::Deserialize<'de> + Eq + fmt::Debug>(original: V) {
            let data = serde_json::to_string(&original).expect("serialization");
            let roundtrip: V = serde_json::from_str(&data).expect("deserialization");
            assert_eq!(original, roundtrip);
        }

        type ArbitraryTodoAggregate = AggregateFromEventSequence<TodoAggregate>;

        proptest! {
            #[test]
            fn can_create_arbitrary_aggregate(agg in any::<ArbitraryTodoAggregate>()) {
            }

            #[test]
            fn arbitrary_aggregate_roundtrips_through_serialization(arg in any::<ArbitraryTodoAggregate>()) {
                verify_serializable_roundtrips_through_serialization(dbg!(arg.into_aggregate()));
            }

            #[test]
            fn arbitrary_event_roundtrips_through_serialization(event in any::<TodoEvent>()) {
                let roundtrip = cqrs_proptest::roundtrip_through_serialization(&event);
                assert_eq!(event, roundtrip);
            }
        }
    }
}
