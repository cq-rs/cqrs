use std::fmt::Debug;
use cqrs::StateSnapshot;

pub trait Store<State> {
    type AggregateId: ?Sized;
    type Error: Debug;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: StateSnapshot<State>) -> Result<(), Self::Error>;
}

impl<State, AggregateId, Error> Store<State> for AsRef<Store<State, AggregateId=AggregateId, Error=Error>>
where
    AggregateId: ?Sized,
    Error: Debug,
{
    type AggregateId = AggregateId;
    type Error = Error;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: StateSnapshot<State>) -> Result<(), Self::Error> {
        self.as_ref().persist_snapshot(agg_id, snapshot)
    }
}


#[cfg(test)] use cqrs::error::Never;
#[cfg(test)]
assert_obj_safe!(snpsnk;
    Store<(), AggregateId=str, Error=Never>,
    Store<(), AggregateId=usize, Error=Never>
);
