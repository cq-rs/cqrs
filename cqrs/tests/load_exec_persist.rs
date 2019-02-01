use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
};

use cqrs::{
    AggregateId, EventNumber, EventSink, EventSource, Precondition, Since, Version, VersionedEvent,
};
use cqrs_todo_core::{TodoAggregate, TodoEvent, TodoIdRef, TodoMetadata};
use void::Void;

#[derive(Debug)]
struct EventMap(RefCell<HashMap<String, Vec<cqrs::VersionedEvent<cqrs_todo_core::TodoEvent>>>>);

impl EventSource<TodoAggregate, TodoEvent> for EventMap {
    type Error = Void;
    type Events = Vec<Result<VersionedEvent<TodoEvent>, Void>>;

    fn read_events<I>(
        &self,
        id: &I,
        since: Since,
        max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<Aggregate = TodoAggregate>,
    {
        let borrow = self.0.borrow();
        let stream = borrow.get(id.as_ref());
        match (since, max_count) {
            (Since::BeginningOfStream, Some(max_count)) => Ok(stream.map(|e| {
                e.into_iter()
                    .take(max_count.min(usize::max_value() as u64) as usize)
                    .map(|e| Ok(e.to_owned()))
                    .collect()
            })),
            (Since::Event(event_number), Some(max_count)) => Ok(stream.map(|e| {
                e.into_iter()
                    .skip(event_number.get() as usize)
                    .take(max_count.min(usize::max_value() as u64) as usize)
                    .map(|e| Ok(e.to_owned()))
                    .collect()
            })),
            (Since::BeginningOfStream, None) => {
                Ok(stream.map(|e| e.into_iter().map(|e| Ok(e.to_owned())).collect()))
            },
            (Since::Event(event_number), None) => Ok(stream.map(|e| {
                e.into_iter()
                    .skip(event_number.get() as usize)
                    .map(|e| Ok(e.to_owned()))
                    .collect()
            })),
        }
    }
}

impl EventSink<TodoAggregate, TodoEvent, TodoMetadata> for EventMap {
    type Error = Void;

    fn append_events<I>(
        &self,
        id: &I,
        events: &[TodoEvent],
        precondition: Option<Precondition>,
        _metadata: TodoMetadata,
    ) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<Aggregate = TodoAggregate>,
    {
        let mut borrow = self.0.borrow_mut();
        let entry = borrow.entry(id.as_ref().into());

        match entry {
            Entry::Occupied(_) if precondition == Some(Precondition::New) => {
                panic!("Need error type here")
            },
            Entry::Vacant(_) => {
                if let Some(Precondition::ExpectedVersion(_)) = precondition {
                    panic!("Need error type here")
                }
            },
            _ => {},
        }

        let stream = entry.or_insert_with(Vec::default);
        let mut sequence = Version::new(stream.len() as u64);
        match precondition {
            Some(Precondition::ExpectedVersion(evt)) => {
                if evt != sequence {
                    panic!("Need error type here")
                }
            },
            Some(Precondition::New) => {
                if sequence != Version::Initial {
                    panic!("Need error type here")
                }
            },
            _ => {},
        }

        sequence.incr();
        stream.reserve(events.len());
        let initial = sequence;
        for event in events {
            stream.push(VersionedEvent {
                sequence: sequence.event_number().unwrap(),
                event: event.to_owned(),
            });
            sequence.incr();
        }

        Ok(initial.event_number().unwrap())
    }
}

#[test]
fn main_test() {
    let em = EventMap(RefCell::new(HashMap::default()));
    let id = TodoIdRef("test");
    let metadata = TodoMetadata {
        initiated_by: String::from("test"),
    };

    assert_eq!(
        em.read_events(&id, Since::BeginningOfStream, None),
        Ok(None)
    );
    let event_num = em
        .append_events(
            &id,
            &[cqrs_todo_core::TodoEvent::Completed(
                cqrs_todo_core::events::Completed {},
            )],
            Some(Precondition::New),
            metadata.clone(),
        )
        .unwrap();
    assert_eq!(event_num, EventNumber::MIN_VALUE);
    let event_num = em
        .append_events(
            &id,
            &[cqrs_todo_core::TodoEvent::Uncompleted(
                cqrs_todo_core::events::Uncompleted {},
            )],
            Some(Precondition::ExpectedVersion(Version::Number(
                EventNumber::MIN_VALUE,
            ))),
            metadata,
        )
        .unwrap();
    assert_eq!(event_num, EventNumber::MIN_VALUE.next());

    let expected_events = vec![
        Ok(VersionedEvent {
            sequence: EventNumber::MIN_VALUE,
            event: cqrs_todo_core::TodoEvent::Completed(cqrs_todo_core::events::Completed {}),
        }),
        Ok(VersionedEvent {
            sequence: EventNumber::MIN_VALUE.next(),
            event: cqrs_todo_core::TodoEvent::Uncompleted(cqrs_todo_core::events::Uncompleted {}),
        }),
    ];

    assert_eq!(
        em.read_events(&id, Since::BeginningOfStream, None),
        Ok(Some(expected_events.clone()))
    );
    assert_eq!(
        em.read_events(&id, Since::Event(EventNumber::MIN_VALUE), None),
        Ok(Some(expected_events[1..].to_owned()))
    );
    assert_eq!(
        em.read_events(&id, Since::Event(EventNumber::MIN_VALUE.next()), None),
        Ok(Some(Vec::default()))
    );
}
