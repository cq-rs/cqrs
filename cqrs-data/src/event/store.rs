use std::error;
use cqrs::EventNumber;
use types::Expectation;

pub trait Store<'id, Event> {
    type AggregateId: 'id;
    type Error: error::Error;

    fn append_events(&self, agg_id: Self::AggregateId, events: &[Event], expect: Expectation) -> Result<EventNumber, Self::Error>;

    fn append_events_from_iterator<I>(&self, agg_id: Self::AggregateId, event_iter: I, expect: Expectation) -> Result<EventNumber, Self::Error>
        where
            I: IntoIterator<Item=Event>,
            Self: Sized,
    {
        let events: Vec<Event> = event_iter.into_iter().collect();
        self.append_events(agg_id, &events, expect)
    }
}

#[cfg(test)] use cqrs::error::Never;
#[cfg(test)]
assert_obj_safe!(evtsnk;
    Store<(), AggregateId=&'static str, Error=Never>,
    Store<(), AggregateId=usize, Error=Never>
);
