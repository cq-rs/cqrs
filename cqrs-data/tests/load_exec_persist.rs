extern crate cqrs;
extern crate cqrs_data;
extern crate cqrs_todo_core;

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use cqrs::{Aggregate, EventNumber, SequencedEvent};
use cqrs_data::event;
use cqrs_data::state;
use cqrs_data::{Expectation, Since};

#[derive(Debug)]
struct EventMap(RefCell<HashMap<String, Vec<cqrs::SequencedEvent<cqrs_todo_core::Event>>>>);

impl<'id> event::Source<'id, cqrs_todo_core::Event> for EventMap {
    type AggregateId = &'id str;
    type Events = Vec<Result<SequencedEvent<cqrs_todo_core::Event>, Never>>;
    type Error = cqrs::error::Never;

    fn read_events(&self, agg_id: Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        let borrow = self.0.borrow();
        let stream = borrow.get(agg_id);
        match since {
            Since::BeginningOfStream => Ok(stream.map(|e| Ok(e.to_owned()))),
            Since::Event(event_number) => Ok(stream.map(|e| Ok(e[event_number.incr().number()..].to_owned()))),
        }
    }
}

impl<'id> event::Store<'id, cqrs_todo_core::Event> for EventMap {
    type AggregateId = &'id str;
    type Error = cqrs::error::Never;

    fn append_events(&self, agg_id: Self::AggregateId, events: &[cqrs_todo_core::Event], expect: Expectation) -> Result<EventNumber, Self::Error> {
        let mut borrow = self.0.borrow_mut();
        let entry = borrow.entry(agg_id.to_string());

        match entry {
            Entry::Occupied(_) if expect == Expectation::New => panic!("Need error type here"),
            Entry::Vacant(_) => if let Expectation::LastEvent(_) = expect { panic!("Need error type here") }
            _ => {}
        }

        let stream = entry.or_insert_with(Vec::default);
        let sequence = EventNumber::new(stream.len());
        match expect {
            Expectation::LastEvent(evt) => if evt.incr() != sequence { panic!("Need error type here") }
            Expectation::Empty => if sequence != EventNumber::default() { panic!("Need error type here") }
            _ => {}
        }

        stream.reserve(events.len());
        let initial = sequence;
        for event in events {
            stream.push(SequencedEvent {
                sequence_number: sequence,
                event: event.to_owned(),
            });
            sequence.incr();
        }

        Ok(initial)
    }
}

use cqrs_data::event::{Source, Store};

#[test]
fn main_test() {
    let em = EventMap(RefCell::new(HashMap::default()));
    let agg_id = "test".to_string();

    assert_eq!(em.read_events(&agg_id, Since::BeginningOfStream), Ok(None));
    let event_num = em.append_events(&agg_id, &[cqrs_todo_core::Event::Completed], Expectation::New).unwrap();
    assert_eq!(event_num, EventNumber::new(0));
    let event_num = em.append_events(&agg_id, &[cqrs_todo_core::Event::Uncompleted], Expectation::LastEvent(EventNumber::new(0))).unwrap();
    assert_eq!(event_num, EventNumber::new(1));

    let expected_events = vec![
        SequencedEvent {
            sequence_number: EventNumber::new(0),
            event: cqrs_todo_core::Event::Completed,
        },
        SequencedEvent {
            sequence_number: EventNumber::new(1),
            event: cqrs_todo_core::Event::Uncompleted,
        },
    ];

    assert_eq!(em.read_events(&agg_id, Since::BeginningOfStream), Ok(Some(expected_events.clone())));
    assert_eq!(em.read_events(&agg_id, Since::Event(EventNumber::new(0))), Ok(Some(expected_events[1..].to_owned())));
    assert_eq!(em.read_events(&agg_id, Since::Event(EventNumber::new(1))), Ok(Some(Vec::default())));


}