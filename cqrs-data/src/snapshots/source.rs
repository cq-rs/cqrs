
pub trait Source {
    type AggregateId: ?Sized;
    type Result;

    fn get_snapshot(&self, agg_id: &Self::AggregateId) -> Self::Result;
}

#[cfg(test)]
assert_obj_safe!(snpsrc; Source<AggregateId=str, Result=()>, Source<AggregateId=usize, Result=()>);
