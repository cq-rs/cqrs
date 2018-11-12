extern crate cqrs;
extern crate cqrs_todo_core;
extern crate void;

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use cqrs::{EventNumber, Precondition, VersionedEvent, Version};
use cqrs::{EventSource, EventSink, Since};
use cqrs_todo_core::{TodoAggregate, Event};
use void::Void;

#[derive(Debug)]
struct EventMap(RefCell<HashMap<String, Vec<cqrs::VersionedEvent<cqrs_todo_core::Event>>>>);

impl EventSource<TodoAggregate> for EventMap {
    type Events = Vec<Result<VersionedEvent<Event>, Void>>;
    type Error = Void;

    fn read_events(&self, id: &str, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error> {
        let borrow = self.0.borrow();
        let stream = borrow.get(id);
        match (since, max_count) {
            (Since::BeginningOfStream, Some(max_count)) => Ok(stream.map(|e| e.into_iter().take(max_count.min(usize::max_value() as u64) as usize).map(|e| Ok(e.to_owned())).collect())),
            (Since::Event(event_number), Some(max_count)) => Ok(stream.map(|e| e.into_iter().skip(event_number.get() as usize).take(max_count.min(usize::max_value() as u64) as usize).map(|e| Ok(e.to_owned())).collect())),
            (Since::BeginningOfStream, None) => Ok(stream.map(|e| e.into_iter().map(|e| Ok(e.to_owned())).collect())),
            (Since::Event(event_number), None) => Ok(stream.map(|e| e.into_iter().skip(event_number.get() as usize).map(|e| Ok(e.to_owned())).collect())),
        }
    }
}

impl EventSink<TodoAggregate> for EventMap {
    type Error = Void;

    fn append_events(&self, id: &str, events: &[Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        let mut borrow = self.0.borrow_mut();
        let entry = borrow.entry(id.into());

        match entry {
            Entry::Occupied(_) if precondition == Some(Precondition::New) => panic!("Need error type here"),
            Entry::Vacant(_) => if let Some(Precondition::ExpectedVersion(_)) = precondition { panic!("Need error type here") }
            _ => {}
        }

        let stream = entry.or_insert_with(Vec::default);
        let mut sequence = Version::new(stream.len() as u64);
        match precondition {
            Some(Precondition::ExpectedVersion(evt)) => if evt != sequence { panic!("Need error type here") }
            Some(Precondition::New) => if sequence != Version::Initial { panic!("Need error type here") }
            _ => {}
        }

        sequence = sequence.incr();
        stream.reserve(events.len());
        let initial = sequence;
        for event in events {
            stream.push(VersionedEvent {
                sequence: sequence.event_number().unwrap(),
                event: event.to_owned(),
            });
            sequence = sequence.incr();
        }

        Ok(initial.event_number().unwrap())
    }
}

#[test]
fn main_test() {
    let em = EventMap(RefCell::new(HashMap::default()));
    let id = "test";

    assert_eq!(em.read_events(id, Since::BeginningOfStream, None), Ok(None));
    let event_num = em.append_events(id, &[cqrs_todo_core::Event::Completed], Some(Precondition::New)).unwrap();
    assert_eq!(event_num, EventNumber::MIN_VALUE);
    let event_num = em.append_events(id, &[cqrs_todo_core::Event::Uncompleted], Some(Precondition::ExpectedVersion(Version::Number(EventNumber::MIN_VALUE)))).unwrap();
    assert_eq!(event_num, EventNumber::MIN_VALUE.incr());

    let expected_events = vec![
        Ok(VersionedEvent {
            sequence: EventNumber::MIN_VALUE,
            event: cqrs_todo_core::Event::Completed,
        }),
        Ok(VersionedEvent {
            sequence: EventNumber::MIN_VALUE.incr(),
            event: cqrs_todo_core::Event::Uncompleted,
        }),
    ];

    assert_eq!(em.read_events(id, Since::BeginningOfStream, None), Ok(Some(expected_events.clone())));
    assert_eq!(em.read_events(id, Since::Event(EventNumber::MIN_VALUE), None), Ok(Some(expected_events[1..].to_owned())));
    assert_eq!(em.read_events(id, Since::Event(EventNumber::MIN_VALUE.incr()), None), Ok(Some(Vec::default())));
}