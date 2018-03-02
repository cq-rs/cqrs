use types::Since;

pub trait Source<'a> {
    type AggregateId: 'a;
    type Result: 'a;

    fn read_events(&'a self, agg_id: Self::AggregateId, since: Since) -> Self::Result;
}

#[cfg(test)]
assert_obj_safe!(evtsrc; Source<'static, AggregateId=&'static str, Result=()>, Source<'static, AggregateId=usize, Result=()>);


