use types::Since;

pub trait Source {
    type AggregateId: ?Sized;
    type Result;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Self::Result;
}

#[cfg(test)]
assert_obj_safe!(evtsrc; Source<AggregateId=str, Result=()>, Source<AggregateId=usize, Result=()>);


