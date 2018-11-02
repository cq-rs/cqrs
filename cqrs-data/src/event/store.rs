use std::fmt::Debug;
use cqrs::{EventNumber, Precondition};

pub trait Store<Event> {
    type AggregateId: ?Sized;
    type Error: Debug;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error>;

    fn append_events_from_iterator<I>(&self, agg_id: &Self::AggregateId, event_iter: I, precondition: Option<Precondition>) -> Result<EventNumber, Self::Error>
        where
            I: IntoIterator<Item=Event>,
            Self: Sized,
    {
        let events: Vec<Event> = event_iter.into_iter().collect();
        self.append_events(agg_id, &events, precondition)
    }
}

impl<Event, AggregateId, Error> Store<Event> for AsRef<Store<Event, AggregateId=AggregateId, Error=Error>>
where
    AggregateId: ?Sized,
    Error: Debug,
{
    type AggregateId = AggregateId;
    type Error = Error;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        self.as_ref().append_events(agg_id, events, precondition)
    }
}

#[cfg(test)] use cqrs::error::Never;
#[cfg(test)]
assert_obj_safe!(evtsnk;
    Store<(), AggregateId=str, Error=Never>,
    Store<(), AggregateId=usize, Error=Never>
);
