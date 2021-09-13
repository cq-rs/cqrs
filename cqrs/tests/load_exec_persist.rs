use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
};

use cqrs::{Aggregate, AggregateId, EventNumber, EventSink, EventSource, Precondition, Since, Version, VersionedEvent};
use cqrs_todo_core::{TodoAggregate, TodoEvent, TodoIdRef, TodoMetadata};
use void::Void;

#[derive(Debug)]
struct EventMap(RefCell<HashMap<String, Vec<cqrs::VersionedEvent<cqrs_todo_core::TodoEvent>>>>);

impl EventSource<TodoAggregate, TodoEvent> for EventMap {
    type Error = Void;
    type Events = Vec<VersionedEvent<TodoEvent>>;

    fn _read_events<I>(
        &self,
        id: Option<&I>,
        since: Since,
        max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<TodoAggregate>,
    {
        let table = self.0.borrow();

        let stream = match id {
            Some(id) => {
                let r = vec![table.get(id.as_str())];
                r
            },
            None => {
                let r = table.values().map(|v| Some(v)).collect();
                r
            },
        }.into_iter().collect::<Option<Vec<_>>>();

        match stream.map(|s|s.into_iter().flatten()) {
            Some(stream) => match (since, max_count) {
                (Since::BeginningOfStream, Some(max_count)) => Ok(Some(
                    stream
                        .into_iter()
                        .take(max_count.min(usize::max_value() as u64) as usize)
                        .map(ToOwned::to_owned)
                        .collect()
                )),
                (Since::Event(event_number), Some(max_count)) => Ok(Some(
                    stream
                        .into_iter()
                        .skip(event_number.get() as usize)
                        .take(max_count.min(usize::max_value() as u64) as usize)
                        .map(ToOwned::to_owned)
                        .collect()
                )),
                (Since::BeginningOfStream, None) => {
                    Ok(Some(stream.into_iter().map(ToOwned::to_owned).collect()))
                }
                (Since::Event(event_number), None) => Ok(Some(
                    stream
                        .into_iter()
                        .skip(event_number.get() as usize)
                        .map(ToOwned::to_owned)
                        .collect()
                )),
            },
            None => Ok(None),
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
        I: AggregateId<TodoAggregate>,
    {
        let mut borrow = self.0.borrow_mut();
        let entry = borrow.entry(id.as_str().into());

        match entry {
            Entry::Occupied(_) if precondition == Some(Precondition::New) => {
                panic!("Need error type here")
            }
            Entry::Vacant(_) => {
                if let Some(Precondition::ExpectedVersion(_)) = precondition {
                    panic!("Need error type here")
                }
            }
            _ => {}
        }

        let stream = entry.or_insert_with(Vec::default);
        let mut sequence = Version::new(stream.len() as u64);
        match precondition {
            Some(Precondition::ExpectedVersion(evt)) => {
                if evt != sequence {
                    panic!("Need error type here")
                }
            }
            Some(Precondition::New) => {
                if sequence != Version::Initial {
                    panic!("Need error type here")
                }
            }
            _ => {}
        }

        sequence.incr();
        stream.reserve(events.len());
        let initial = sequence;
        for event in events {
            stream.push(VersionedEvent {
                sequence: sequence.event_number().unwrap(),
                event: event.to_owned(),
                aggregate_id: id.as_str().to_owned(),
                aggregate_type: TodoAggregate::aggregate_type().to_owned()
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
        VersionedEvent {
            sequence: EventNumber::MIN_VALUE,
            event: cqrs_todo_core::TodoEvent::Completed(cqrs_todo_core::events::Completed {}),
            aggregate_id: id.as_str().to_owned(),
            aggregate_type: TodoAggregate::aggregate_type().to_owned()
        },
        VersionedEvent {
            sequence: EventNumber::MIN_VALUE.next(),
            event: cqrs_todo_core::TodoEvent::Uncompleted(cqrs_todo_core::events::Uncompleted {}),
            aggregate_id: id.as_str().to_owned(),
            aggregate_type: TodoAggregate::aggregate_type().to_owned()
        },
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
