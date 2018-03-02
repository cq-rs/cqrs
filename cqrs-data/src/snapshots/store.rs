use cqrs;

pub trait Store {
    type AggregateId: ?Sized;
    type Snapshot;
    type Result;

    fn persist_snapshot(&self, agg_id: &Self::AggregateId, snapshot: cqrs::VersionedSnapshot<Self::Snapshot>) -> Self::Result;
}

#[cfg(test)]
assert_obj_safe!(snpsnk; Store<AggregateId=str, Snapshot=(), Result=()>, Store<AggregateId=usize, Snapshot=(), Result=()>);
