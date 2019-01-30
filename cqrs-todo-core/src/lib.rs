//! # cqrs-todo-core
//!
//! `cqrs-todo-core` is a demonstration crate, showing how to construct an aggregate, with the associated events
//! and commands, using the CQRS system.

#![warn(unused_import_braces, unused_imports, unused_qualifications)]
#![deny(
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_must_use,
    missing_docs
)]

extern crate arrayvec;
extern crate chrono;
extern crate cqrs_core;
#[cfg(test)]
extern crate cqrs_proptest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate log;
#[cfg(test)]
extern crate pretty_assertions;
#[cfg(test)]
extern crate proptest;

use cqrs_core::{
    Aggregate, AggregateEvent, AggregateId, DeserializableEvent, Event, SerializableEvent,
};

pub mod commands;
pub mod domain;
pub mod error;
pub mod events;

/// An aggregate representing the view of a to-do item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoAggregate {
    /// A to-do item that has been properly initialized.
    Created(TodoData),

    /// An uninitialized to-do item.
    Uninitialized,
}

impl Default for TodoAggregate {
    fn default() -> Self {
        TodoAggregate::Uninitialized
    }
}

impl TodoAggregate {
    /// Get the underlying to-do data if the aggregate has been initialized.
    pub fn get_data(&self) -> Option<&TodoData> {
        match *self {
            TodoAggregate::Uninitialized => None,
            TodoAggregate::Created(ref x) => Some(x),
        }
    }
}

impl Aggregate for TodoAggregate {
    type Event = TodoEvent;

    #[inline(always)]
    fn aggregate_type() -> &'static str
    where
        Self: Sized,
    {
        "todo"
    }
}

/// An identifier for an item to be done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TodoId(pub String);

impl AsRef<str> for TodoId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AggregateId for TodoId {
    type Aggregate = TodoAggregate;
}

/// An identifier for an item to be done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TodoIdRef<'a>(pub &'a str);

impl<'a> AsRef<str> for TodoIdRef<'a> {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'a> AggregateId for TodoIdRef<'a> {
    type Aggregate = TodoAggregate;
}

/// Metadata about events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoMetadata {
    /// The actor that caused this event to be added to the event stream.
    pub initiated_by: String,
}

/// Data relating to a to-do item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoData {
    /// The to-do item description.
    pub description: domain::Description,

    /// The reminder time for this to-do item.
    pub reminder: Option<domain::Reminder>,

    /// The current status of this item.
    pub status: TodoStatus,
}

impl TodoData {
    fn with_description(description: domain::Description) -> Self {
        TodoData {
            description,
            reminder: None,
            status: TodoStatus::NotCompleted,
        }
    }
}

/// The completion status of a to-do item.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoStatus {
    /// The item has been completed.
    Completed,

    /// The item has not been completed.
    NotCompleted,
}

/// A combined roll-up of the events that can be applied to a [TodoAggregate].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoEvent {
    /// Created
    Created(events::Created),

    /// Description updated
    DescriptionUpdated(events::DescriptionUpdated),

    /// Reminder updated
    ReminderUpdated(events::ReminderUpdated),

    /// Item completed
    Completed(events::Completed),

    /// Item completion undone
    Uncompleted(events::Uncompleted),
}

impl Event for TodoEvent {
    fn event_type(&self) -> &'static str {
        match *self {
            TodoEvent::Created(ref evt) => evt.event_type(),
            TodoEvent::DescriptionUpdated(ref evt) => evt.event_type(),
            TodoEvent::ReminderUpdated(ref evt) => evt.event_type(),
            TodoEvent::Completed(ref evt) => evt.event_type(),
            TodoEvent::Uncompleted(ref evt) => evt.event_type(),
        }
    }
}

impl AggregateEvent for events::Created {
    type Aggregate = TodoAggregate;

    fn apply_to(self, aggregate: &mut Self::Aggregate) {
        if TodoAggregate::Uninitialized == *aggregate {
            *aggregate =
                TodoAggregate::Created(TodoData::with_description(self.initial_description))
        }
    }
}

impl AggregateEvent for events::DescriptionUpdated {
    type Aggregate = TodoAggregate;

    fn apply_to(self, aggregate: &mut Self::Aggregate) {
        if let TodoAggregate::Created(ref mut data) = aggregate {
            data.description = self.new_description;
        }
    }
}

impl AggregateEvent for events::ReminderUpdated {
    type Aggregate = TodoAggregate;

    fn apply_to(self, aggregate: &mut Self::Aggregate) {
        if let TodoAggregate::Created(ref mut data) = aggregate {
            data.reminder = self.new_reminder;
        }
    }
}

impl AggregateEvent for events::Completed {
    type Aggregate = TodoAggregate;

    fn apply_to(self, aggregate: &mut Self::Aggregate) {
        if let TodoAggregate::Created(ref mut data) = aggregate {
            data.status = TodoStatus::Completed;
        }
    }
}

impl AggregateEvent for events::Uncompleted {
    type Aggregate = TodoAggregate;

    fn apply_to(self, aggregate: &mut Self::Aggregate) {
        if let TodoAggregate::Created(ref mut data) = aggregate {
            data.status = TodoStatus::NotCompleted;
        }
    }
}
impl AggregateEvent for TodoEvent {
    type Aggregate = TodoAggregate;

    fn apply_to(self, aggregate: &mut Self::Aggregate) {
        match self {
            TodoEvent::Created(evt) => evt.apply_to(aggregate),
            TodoEvent::DescriptionUpdated(evt) => evt.apply_to(aggregate),
            TodoEvent::ReminderUpdated(evt) => evt.apply_to(aggregate),
            TodoEvent::Completed(evt) => evt.apply_to(aggregate),
            TodoEvent::Uncompleted(evt) => evt.apply_to(aggregate),
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
            TodoEvent::DescriptionUpdated(ref inner) => {
                serde_json::to_writer(buffer, inner)?;
            },
            TodoEvent::Completed(ref inner) => {
                serde_json::to_writer(buffer, inner)?;
            },
            TodoEvent::Uncompleted(ref inner) => {
                serde_json::to_writer(buffer, inner)?;
            },
        }
        Ok(())
    }
}

impl DeserializableEvent for TodoEvent {
    type Error = serde_json::Error;

    fn deserialize_event_from_buffer(
        data: &[u8],
        event_type: &str,
    ) -> Result<Option<Self>, Self::Error> {
        let deserialized = match event_type {
            "todo_created" => TodoEvent::Created(serde_json::from_slice(data)?),
            "todo_reminder_updated" => TodoEvent::ReminderUpdated(
                serde_json::from_slice(data)?,
            ),
            "todo_description_updated" => TodoEvent::DescriptionUpdated(
                serde_json::from_slice(data)?,
            ),
            "todo_completed" => TodoEvent::Completed(serde_json::from_slice(data)?),
            "todo_uncompleted" => TodoEvent::Uncompleted(serde_json::from_slice(data)?),
            _ => return Ok(None),
        };
        Ok(Some(deserialized))
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use arrayvec::ArrayVec;
    use chrono::{Duration, TimeZone, Utc};
    use pretty_assertions::assert_eq;

    fn create_basic_aggregate() -> TodoAggregate {
        let now = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder = now + Duration::seconds(10000);

        let events = ArrayVec::from([
            TodoEvent::Completed(events::Completed {}),
            TodoEvent::Created(events::Created {
                initial_description: domain::Description::new("Hello!").unwrap(),
            }),
            TodoEvent::ReminderUpdated(events::ReminderUpdated {
                new_reminder: Some(domain::Reminder::new(reminder, now).unwrap()),
            }),
            TodoEvent::DescriptionUpdated(events::DescriptionUpdated {
                new_description: domain::Description::new("New text").unwrap(),
            }),
            TodoEvent::Created(events::Created {
                initial_description: domain::Description::new("Ignored!").unwrap(),
            }),
            TodoEvent::ReminderUpdated(events::ReminderUpdated { new_reminder: None }),
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

        let cmd = commands::CancelReminder;

        let result = agg.execute(cmd).unwrap_err();

        assert_eq!(error::CommandError::NotInitialized, result);
    }

    #[test]
    fn cancel_reminder_on_basic_aggregate() {
        let agg = create_basic_aggregate();

        let cmd = commands::CancelReminder;

        let result = agg.execute(cmd).unwrap();

        assert_eq!(ArrayVec::new(), result);
    }

    #[test]
    fn set_reminder_on_basic_aggregate() {
        let agg = create_basic_aggregate();

        let now = Utc.ymd(1970, 1, 1).and_hms(0, 0, 0);
        let reminder_time = now + Duration::seconds(20000);
        let new_reminder = domain::Reminder::new(reminder_time, now).unwrap();
        let cmd = commands::SetReminder { new_reminder };

        let result = agg.execute(cmd).unwrap();

        let mut expected = ArrayVec::new();
        expected.push(TodoEvent::ReminderUpdated(events::ReminderUpdated {
            new_reminder: Some(new_reminder),
        }));
        assert_eq!(expected, result);
    }

    #[test]
    fn ensure_created_event_stays_same() -> Result<(), serde_json::Error> {
        let initial_description = domain::Description::new("test description").unwrap();
        run_snapshot_test(
            "created_event",
            TodoEvent::Created(events::Created {
                initial_description,
            }),
        )
    }

    #[test]
    fn ensure_reminder_updated_event_stays_same() -> Result<(), serde_json::Error> {
        let current_time = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);
        let reminder_time = Utc.ymd(2100, 1, 1).and_hms(0, 0, 0);
        let reminder = domain::Reminder::new(reminder_time, current_time).unwrap();
        run_snapshot_test(
            "reminder_updated_event",
            TodoEvent::ReminderUpdated(events::ReminderUpdated {
                new_reminder: Some(reminder),
            }),
        )
    }

    #[test]
    fn ensure_reminder_removed_event_stays_same() -> Result<(), serde_json::Error> {
        run_snapshot_test(
            "reminder_updated_none_event",
            TodoEvent::ReminderUpdated(events::ReminderUpdated { new_reminder: None }),
        )
    }

    #[test]
    fn ensure_text_updated_event_stays_same() -> Result<(), serde_json::Error> {
        let new_description = domain::Description::new("alt test description").unwrap();
        run_snapshot_test(
            "description_updated_event",
            TodoEvent::DescriptionUpdated(events::DescriptionUpdated { new_description }),
        )
    }

    #[test]
    fn ensure_completed_event_stays_same() -> Result<(), serde_json::Error> {
        run_snapshot_test(
            "completed_event",
            TodoEvent::Completed(events::Completed {}),
        )
    }

    #[test]
    fn ensure_uncompleted_event_stays_same() -> Result<(), serde_json::Error> {
        run_snapshot_test(
            "uncompleted_event",
            TodoEvent::Uncompleted(events::Uncompleted {}),
        )
    }

    fn run_snapshot_test<E: SerializableEvent>(
        name: &'static str,
        event: E,
    ) -> Result<(), E::Error> {
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
    fn roundtrip_created() {
        let original = TodoEvent::Created(events::Created {
            initial_description: domain::Description::new("test description").unwrap(),
        });
        let roundtrip = cqrs_proptest::roundtrip_through_serialization(&original);
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn roundtrip_reminder_updated() {
        let original = TodoEvent::ReminderUpdated(events::ReminderUpdated {
            new_reminder: Some(
                domain::Reminder::new(
                    Utc.ymd(2100, 1, 1).and_hms(0, 0, 0),
                    Utc.ymd(2000, 1, 1).and_hms(0, 0, 0),
                )
                .unwrap(),
            ),
        });
        let roundtrip = cqrs_proptest::roundtrip_through_serialization(&original);
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn roundtrip_reminder_updated_none() {
        let original = TodoEvent::ReminderUpdated(events::ReminderUpdated { new_reminder: None });
        let roundtrip = cqrs_proptest::roundtrip_through_serialization(&original);
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn roundtrip_description_updated() {
        let original = TodoEvent::DescriptionUpdated(events::DescriptionUpdated {
            new_description: domain::Description::new("alt test description").unwrap(),
        });
        let roundtrip = cqrs_proptest::roundtrip_through_serialization(&original);
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn roundtrip_completed() {
        let original = TodoEvent::Completed(events::Completed {});
        let roundtrip = cqrs_proptest::roundtrip_through_serialization(&original);
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn roundtrip_uncompleted() {
        let original = TodoEvent::Uncompleted(events::Uncompleted {});
        let roundtrip = cqrs_proptest::roundtrip_through_serialization(&original);
        assert_eq!(original, roundtrip);
    }

    mod property_tests {
        use super::*;
        use cqrs_proptest::AggregateFromEventSequence;
        use pretty_assertions::assert_eq;
        use proptest::{prelude::*, prop_oneof, proptest, proptest_helper};
        use std::fmt;

        impl Arbitrary for domain::Description {
            type Parameters = proptest::string::StringParam;
            type Strategy = BoxedStrategy<Self>;

            fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
                let s: &'static str = args.into();
                s.prop_filter_map("invalid description", |d| domain::Description::new(d).ok())
                    .boxed()
            }
        }

        impl Arbitrary for domain::Reminder {
            type Parameters = ();
            type Strategy = BoxedStrategy<Self>;

            fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
                let current_time = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);

                (2000..2500_i32, 1..=366_u32, 0..86400_u32)
                    .prop_filter_map("invalid date", move |(y, o, s)| {
                        let time = chrono::NaiveTime::from_num_seconds_from_midnight(s, 0);
                        let date = Utc.yo_opt(y, o).single()?.and_time(time)?;
                        domain::Reminder::new(date, current_time).ok()
                    })
                    .boxed()
            }
        }

        impl Arbitrary for events::Created {
            type Parameters = ();
            type Strategy = BoxedStrategy<Self>;

            fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
                any::<domain::Description>()
                    .prop_map(|initial_description| events::Created {
                        initial_description,
                    })
                    .boxed()
            }
        }

        impl Arbitrary for events::ReminderUpdated {
            type Parameters = ();
            type Strategy = BoxedStrategy<Self>;

            fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
                any::<Option<domain::Reminder>>()
                    .prop_map(|new_reminder| events::ReminderUpdated { new_reminder })
                    .boxed()
            }
        }

        impl Arbitrary for events::DescriptionUpdated {
            type Parameters = ();
            type Strategy = BoxedStrategy<Self>;

            fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
                any::<domain::Description>()
                    .prop_map(|new_description| events::DescriptionUpdated { new_description })
                    .boxed()
            }
        }

        impl Arbitrary for events::Completed {
            type Parameters = ();
            type Strategy = Just<Self>;

            fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
                Just(events::Completed {})
            }
        }

        impl Arbitrary for events::Uncompleted {
            type Parameters = ();
            type Strategy = Just<Self>;

            fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
                Just(events::Uncompleted {})
            }
        }

        impl Arbitrary for TodoEvent {
            type Parameters = ();
            type Strategy = BoxedStrategy<Self>;

            fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
                prop_oneof![
                    any::<events::Created>().prop_map(TodoEvent::Created),
                    any::<events::ReminderUpdated>().prop_map(TodoEvent::ReminderUpdated),
                    any::<events::DescriptionUpdated>().prop_map(TodoEvent::DescriptionUpdated),
                    any::<events::Completed>().prop_map(TodoEvent::Completed),
                    any::<events::Uncompleted>().prop_map(TodoEvent::Uncompleted),
                ]
                .boxed()
            }
        }

        fn verify_serializable_roundtrips_through_serialization<
            V: serde::Serialize + for<'de> serde::Deserialize<'de> + Eq + fmt::Debug,
        >(
            original: V,
        ) {
            let data = serde_json::to_string(&original).expect("serialization");
            let roundtrip: V = serde_json::from_str(&data).expect("deserialization");
            assert_eq!(original, roundtrip);
        }

        type ArbitraryTodoAggregate = AggregateFromEventSequence<TodoAggregate>;

        proptest! {
            #[test]
            fn can_create_arbitrary_aggregate(_agg in any::<ArbitraryTodoAggregate>()) {
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
