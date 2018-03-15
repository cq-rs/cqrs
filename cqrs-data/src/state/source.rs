use std::error;
use cqrs::StateSnapshot;

pub trait Source<'id, State> {
    type AggregateId: 'id;
    type Error: error::Error;

    fn get_snapshot(&self, agg_id: Self::AggregateId) -> Result<Option<StateSnapshot<State>>, Self::Error>;
}

#[cfg(test)] use cqrs::error::Never;
#[cfg(test)]
assert_obj_safe!(snpsrc;
    Source<(), AggregateId=&'static str, Error=Never>,
    Source<(), AggregateId=usize, Error=Never>
);
