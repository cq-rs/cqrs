use std::error;
use cqrs::SequencedEvent;
use types::Since;

pub trait Source<'id, Event> {
    type AggregateId: 'id;
    type Events: IntoIterator<Item=Result<SequencedEvent<Event>, Self::Error>>;
    type Error: error::Error;

    fn read_events(&self, agg_id: Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error>;
}

#[cfg(test)] use cqrs::error::Never;
#[cfg(test)]
assert_obj_safe!(evtsrc;
    Source<(), AggregateId=&'static str, Events=Vec<()>, Error=Never>,
    Source<(), AggregateId=usize, Events=Vec<()>, Error=Never>
);


