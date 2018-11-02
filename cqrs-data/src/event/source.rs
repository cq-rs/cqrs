use std::fmt::Debug;
use cqrs::SequencedEvent;
use types::Since;

pub trait Source<Event> {
    type AggregateId: ?Sized;
    type Events: IntoIterator<Item=Result<SequencedEvent<Event>, Self::Error>>;
    type Error: Debug;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error>;
}

impl<Event, AggregateId, Events, Error> Source<Event> for AsRef<Source<Event, AggregateId=AggregateId, Events=Events, Error=Error>>
where
    AggregateId: ?Sized,
    Events: IntoIterator<Item=Result<SequencedEvent<Event>, Error>>,
    Error: Debug,
{
    type AggregateId = AggregateId;
    type Events = Events;
    type Error = Error;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Result<Option<<Self as Source<Event>>::Events>, <Self as Source<Event>>::Error> {
        self.as_ref().read_events(agg_id, since)
    }
}

#[cfg(test)] use cqrs::error::Never;
#[cfg(test)]
assert_obj_safe!(evtsrc;
    Source<(), AggregateId=str, Events=Vec<()>, Error=Never>,
    Source<(), AggregateId=usize, Events=Vec<()>, Error=Never>
);


