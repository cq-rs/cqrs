#![allow(dead_code)]

use std::marker::PhantomData;

use cqrs::EventSourced as _;
use cqrs_codegen::{Aggregate, Event, EventSourced};

#[derive(Aggregate, Default)]
#[aggregate(type = "aggregate")]
struct Aggregate {
    id: i32,
    event1_applied: bool,
    event2_applied: bool,
    event3_applied: bool,
    event4_applied: bool,
}

impl cqrs::EventSourced<Event1> for Aggregate {
    fn apply(&mut self, _ev: &Event1) {
        self.event1_applied = true;
    }
}

impl cqrs::EventSourced<Event2> for Aggregate {
    fn apply(&mut self, _ev: &Event2) {
        self.event2_applied = true;
    }
}

impl<T> cqrs::EventSourced<Event3<T>> for Aggregate {
    fn apply(&mut self, _ev: &Event3<T>) {
        self.event3_applied = true;
    }
}

impl<T> cqrs::EventSourced<Event4<T>> for Aggregate {
    fn apply(&mut self, _ev: &Event4<T>) {
        self.event4_applied = true;
    }
}

#[derive(Default, Event)]
#[event(type = "event.1")]
struct Event1;

#[derive(Default, Event)]
#[event(type = "event.2")]
struct Event2;

#[derive(Default, Event)]
#[event(type = "event.3")]
struct Event3<T>(PhantomData<*const T>);

#[derive(Default, Event)]
#[event(type = "event.4")]
struct Event4<T>(PhantomData<*const T>);

#[test]
fn derives_for_enum() {
    #[derive(Event, EventSourced)]
    #[event_sourced(aggregate = "Aggregate")]
    enum Event {
        Event1(Event1),
        Event2 { event: Event2 },
    }

    let mut aggregate = Aggregate::default();
    aggregate.apply(&Event::Event1(Event1));
    aggregate.apply(&Event::Event2 { event: Event2 });

    assert!(aggregate.event1_applied);
    assert!(aggregate.event2_applied);
}

#[test]
fn derives_for_generic_enum() {
    #[derive(Event, EventSourced)]
    #[event_sourced(aggregate = "Aggregate")]
    enum Event<E1, E2, T> {
        Event1(E1),
        Event2 { event: E2 },
        Event3(Event3<T>),
        Event4 { event: Event4<T> },
    }

    let mut aggregate = Aggregate::default();
    aggregate.apply(&Event::<Event1, Event2, i32>::Event1(Event1));
    aggregate.apply(&Event::<Event1, Event2, i32>::Event2 { event: Event2 });
    aggregate.apply(&Event::<Event1, Event2, i32>::Event3(Event3::default()));
    aggregate.apply(&Event::<Event1, Event2, i32>::Event4 {
        event: Event4::default(),
    });

    assert!(aggregate.event1_applied);
    assert!(aggregate.event2_applied);
    assert!(aggregate.event3_applied);
    assert!(aggregate.event4_applied);
}
