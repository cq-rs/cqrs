use std::error;
use cqrs::StateSnapshot;

pub trait Store<'id, State> {
    type AggregateId: 'id;
    type Error: error::Error;

    fn persist_snapshot(&self, agg_id: Self::AggregateId, snapshot: StateSnapshot<State>) -> Result<(), Self::Error>;
}

#[cfg(test)] use cqrs::error::Never;
#[cfg(test)]
assert_obj_safe!(snpsnk;
    Store<(), AggregateId=&'static str, Error=Never>,
    Store<(), AggregateId=usize, Error=Never>
);
