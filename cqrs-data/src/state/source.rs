use std::fmt::Debug;
use cqrs::StateSnapshot;

pub trait Source<State> {
    type AggregateId: ?Sized;
    type Error: Debug;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<StateSnapshot<State>>, Self::Error>;
}

impl<State, AggregateId, Error> Source<State> for AsRef<Source<State, AggregateId=AggregateId, Error=Error>>
where
    AggregateId: ?Sized,
    Error: Debug,
{
    type AggregateId = AggregateId;
    type Error = Error;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Result<Option<StateSnapshot<State>>, Self::Error> {
        self.as_ref().get_snapshot(agg_id)
    }
}

#[cfg(test)] use cqrs::error::Never;
#[cfg(test)]
assert_obj_safe!(snpsrc;
    Source<(), AggregateId=str, Error=Never>,
    Source<(), AggregateId=usize, Error=Never>
);
