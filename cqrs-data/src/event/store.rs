use std::fmt::Debug;
use cqrs::{EventNumber, Precondition};

pub trait EventSink<A: cqrs::Aggregate>: Sized {
    type Error: Debug;

    fn append_events<Id: AsRef<str> + Into<String>>(&self, id: Id, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error>;

    fn append_events_from_iterator<Id, I>(&self, id: Id, event_iter: I, precondition: Option<Precondition>) -> Result<EventNumber, Self::Error>
        where
            Id: AsRef<str> + Into<String>,
            I: IntoIterator<Item=A::Event>,
    {
        let events: Vec<A::Event> = event_iter.into_iter().collect();
        self.append_events(id, &events, precondition)
    }
}