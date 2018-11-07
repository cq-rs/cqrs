extern crate cqrs;
extern crate cqrs_data;
extern crate cqrs_todo_core;

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use cqrs::{EventNumber, Precondition, SequencedEvent, Version};
use cqrs::error::Never;
use cqrs_data::event;
use cqrs_data::Since;

#[derive(Debug)]
struct EventMap(RefCell<HashMap<String, Vec<cqrs::SequencedEvent<cqrs_todo_core::Event>>>>);

impl event::EventSource<cqrs_todo_core::Event> for EventMap {
    type AggregateId = str;
    type Events = Vec<Result<SequencedEvent<cqrs_todo_core::Event>, Never>>;
    type Error = cqrs::error::Never;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        let borrow = self.0.borrow();
        let stream = borrow.get(agg_id);
        match since {
            Since::BeginningOfStream => Ok(stream.map(|e| e.into_iter().map(|e| Ok(e.to_owned())).collect())),
            Since::Event(event_number) => Ok(stream.map(|e| e.into_iter().skip(event_number.incr().number()).map(|e| Ok(e.to_owned())).collect())),
        }
    }
}

impl event::EventSink<cqrs_todo_core::Event> for EventMap {
    type AggregateId = str;
    type Error = cqrs::error::Never;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[cqrs_todo_core::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        let mut borrow = self.0.borrow_mut();
        let entry = borrow.entry(agg_id.to_string());

        match entry {
            Entry::Occupied(_) if precondition == Some(Precondition::New) => panic!("Need error type here"),
            Entry::Vacant(_) => if let Some(Precondition::ExpectedVersion(_)) = precondition { panic!("Need error type here") }
            _ => {}
        }

        let stream = entry.or_insert_with(Vec::default);
        let sequence = EventNumber::new(stream.len()).unwrap();
        match precondition {
            Some(Precondition::ExpectedVersion(evt)) => if evt.incr() != Version::Number(sequence) { panic!("Need error type here") }
            Some(Precondition::New) => if sequence != EventNumber::default() { panic!("Need error type here") }
            _ => {}
        }

        stream.reserve(events.len());
        let initial = sequence;
        for event in events {
            stream.push(SequencedEvent {
                sequence,
                event: event.to_owned(),
            });
            sequence.incr();
        }

        Ok(initial)
    }
}

use cqrs_data::event::{EventSource, EventSink};

#[test]
fn main_test() {
    let em = EventMap(RefCell::new(HashMap::default()));
    let agg_id = "test".to_string();

    assert_eq!(em.read_events(&agg_id, Since::BeginningOfStream), Ok(None));
    let event_num = em.append_events(&agg_id, &[cqrs_todo_core::Event::Completed], Some(Precondition::New)).unwrap();
    assert_eq!(event_num, EventNumber::new(0));
    let event_num = em.append_events(&agg_id, &[cqrs_todo_core::Event::Uncompleted], Some(Precondition::ExpectedVersion(Version::Number(EventNumber::new(0))))).unwrap();
    assert_eq!(event_num, EventNumber::new(1));

    let expected_events = vec![
        Ok(SequencedEvent {
            sequence: EventNumber::new(1).unwrap(),
            event: cqrs_todo_core::Event::Completed,
        }),
        Ok(SequencedEvent {
            sequence: EventNumber::new(2).unwrap(),
            event: cqrs_todo_core::Event::Uncompleted,
        }),
    ];

    assert_eq!(em.read_events(&agg_id, Since::BeginningOfStream), Ok(Some(expected_events.clone())));
    assert_eq!(em.read_events(&agg_id, Since::Event(EventNumber::new(1).unwrap())), Ok(Some(expected_events[1..].to_owned())));
    assert_eq!(em.read_events(&agg_id, Since::Event(EventNumber::new(2).unwrap())), Ok(Some(Vec::default())));


}