use std::marker::PhantomData;
use cqrs::{Aggregate, HydratedAggregate, Precondition};
use events;
use snapshots;
use trivial;

pub struct SyncAggregateStore<Agg, E, S>
    where
        E: events::Store,
        S: snapshots::Store,
{
    event_store: E,
    snapshot_store: S,
    _phantom: PhantomData<Agg>,
}

impl<A, E, S, EvtErr, SnpErr> SyncAggregateStore<A, E, S>
    where
        A: Aggregate,
        E: events::Store<Event=A::Event, Result=Result<(), EvtErr>>,
        S: snapshots::Store<AggregateId=E::AggregateId, Result=Result<(), SnpErr>, Snapshot=A>,
        EvtErr: ::std::error::Error,
        SnpErr: ::std::error::Error,
{
    pub fn new(event_store: E, snapshot_store: S) -> Self {
        SyncAggregateStore {
            event_store,
            snapshot_store,
            _phantom: PhantomData
        }
    }

    pub fn persist(&self, agg_id: &E::AggregateId, mut aggregate: HydratedAggregate<A>, new_events: Vec<E::Event>, precondition: Option<Precondition>) -> Result<(), String> {
        self.event_store.append_events(agg_id, &new_events, precondition).map_err(|e| e.to_string())?;

        for evt in new_events {
            aggregate.apply_raw(evt, None);
        }

        self.snapshot_store.persist_snapshot(agg_id, aggregate.snapshot()).map_err(|e| e.to_string())?;

        Ok(())
    }
}

impl<A, E, EvtErr> SyncAggregateStore<A, E, trivial::NullSnapshotStore<A, E::AggregateId>>
    where
        A: Aggregate,
        E: events::Store<Event=A::Event, Result=Result<(), EvtErr>>,
        EvtErr: ::std::error::Error,
{
    pub fn new_for_events_only(event_store: E) -> Self {
        SyncAggregateStore {
            event_store,
            snapshot_store: trivial::NullSnapshotStore::<A, E::AggregateId>::default(),
            _phantom: PhantomData,
        }
    }
}

impl<A, S, SnpErr> SyncAggregateStore<A, trivial::NullEventStore<A::Event, S::AggregateId>, S>
    where
        A: Aggregate,
        S: snapshots::Store<Snapshot=A, Result=Result<(), SnpErr>>,
        SnpErr: ::std::error::Error,
{
    pub fn new_for_snapshot_only(snapshot_store: S) -> Self {
        SyncAggregateStore {
            event_store: trivial::NullEventStore::<A::Event, S::AggregateId>::default(),
            snapshot_store,
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use cqrs::{Aggregate};

    #[derive(Default, Debug)]
    struct Empty(usize);

    impl Aggregate for Empty {
        type Event = usize;
        type Command = ();
        type Events = Vec<usize>;
        type Error = ::cqrs::error::Never;

        fn apply(&mut self, event: Self::Event) {
            self.0 += event;
        }

        fn execute(&self, _: Self::Command) -> Result<Self::Events, Self::Error> {
            Ok((0..self.0).collect())
        }
    }

    #[test]
    fn can_create() {
        let store: SyncAggregateStore<Empty,_, _> =
            SyncAggregateStore::new(
                trivial::NullEventStore::<usize, usize>::default(),
                trivial::NullSnapshotStore::<Empty, usize>::default()
            );

        let mut agg = HydratedAggregate::default();
        agg.apply_raw(75, None);

        let res = store.persist(&0, agg, vec![1, 3, 5, 3], None);
        assert!(res.is_ok());
    }

    #[test]
    fn can_create_events() {
        let store: SyncAggregateStore<Empty,_, _> =
            SyncAggregateStore::new_for_events_only(
                trivial::NullEventStore::<usize, usize>::default(),
            );

        let mut agg = HydratedAggregate::default();
        agg.apply_raw(75, None);

        let res = store.persist(&0, agg, vec![1, 3, 5, 3], None);
        assert!(res.is_ok());
    }

    #[test]
    fn can_create_snapshots() {
        let store: SyncAggregateStore<Empty,_, _> =
            SyncAggregateStore::new_for_snapshot_only(
                trivial::NullSnapshotStore::<Empty, usize>::default(),
            );

        let mut agg = HydratedAggregate::default();
        agg.apply_raw(75, None);

        let res = store.persist(&0, agg, vec![1, 3, 5, 3], None);
        assert!(res.is_ok());
    }
}
