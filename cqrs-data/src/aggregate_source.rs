use std::marker::PhantomData;
use cqrs::{Aggregate, HydratedAggregate, SequencedEvent, Version, VersionedSnapshot};
use events;
use snapshots;
use trivial;
use types::Since;

pub struct SyncAggregateSource<A, E, S>
    where
        A: Aggregate,
        E: events::Source,
        S: snapshots::Source,
{
    event_source: E,
    snapshot_source: S,
    _phantom: PhantomData<A>
}

impl<A, E, S, Events, EvtErr, SnpErr> SyncAggregateSource<A, E, S>
    where
        A: Aggregate + Default,
        Events: IntoIterator<Item=SequencedEvent<A::Event>>,
        E: events::Source<Result=Result<Option<Events>, EvtErr>>,
        S: snapshots::Source<AggregateId=E::AggregateId, Result=Result<Option<VersionedSnapshot<A>>, SnpErr>>,
        EvtErr: ::std::error::Error,
        SnpErr: ::std::error::Error,
{
    pub fn new(event_source: E, snapshot_source: S) -> Self {
        SyncAggregateSource {
            event_source,
            snapshot_source,
            _phantom: PhantomData,
        }
    }

    pub fn rehydrate(&self, agg_id: &E::AggregateId) -> Result<Option<HydratedAggregate<A>>, String> {
        let snp_opt = self.snapshot_source.get_snapshot(agg_id).map_err(|e| e.to_string())?;

        if let Some(snp) = snp_opt {
            let since =
                match snp.version {
                    Version::Initial => Since::BeginningOfStream,
                    Version::Number(evt_nbr) => Since::Event(evt_nbr),
                };

            let evts_opt = self.event_source.read_events(agg_id, since).map_err(|e| e.to_string())?;

            let mut agg = HydratedAggregate::restore(snp);

            if let Some(evts) = evts_opt {
                for evt in evts {
                    agg.apply(evt);
                }
            }

            Ok(Some(agg))
        } else {
            let evts_opt = self.event_source.read_events(agg_id, Since::BeginningOfStream).map_err(|e| e.to_string())?;

            if let Some(evts) = evts_opt {
                let mut agg = HydratedAggregate::default();
                for evt in evts {
                    agg.apply(evt);
                }
                Ok(Some(agg))
            } else {
                Ok(None)
            }
        }
    }
}

impl<A, E, Events, EvtErr> SyncAggregateSource<A, E, trivial::NullSnapshotStore<A, E::AggregateId>>
    where
        A: Aggregate + Default,
        Events: IntoIterator<Item=SequencedEvent<A::Event>>,
        E: events::Source<Result=Result<Option<Events>, EvtErr>>,
        EvtErr: ::std::error::Error,
{
    pub fn new_with_events_only(event_source: E) -> Self {
        SyncAggregateSource {
            event_source,
            snapshot_source: trivial::NullSnapshotStore::<A, E::AggregateId>::default(),
            _phantom: PhantomData,
        }
    }
}

impl<A, S, SnpErr> SyncAggregateSource<A, trivial::NullEventStore<A::Event, S::AggregateId>, S>
    where
        A: Aggregate + Default,
        S: snapshots::Source<Result=Result<Option<VersionedSnapshot<A>>, SnpErr>>,
        SnpErr: ::std::error::Error,
{
    pub fn new_with_snapshots_only(snapshot_source: S) -> Self {
        SyncAggregateSource {
            event_source: trivial::NullEventStore::<A::Event, S::AggregateId>::default(),
            snapshot_source,
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use cqrs::{Aggregate};

    #[derive(Default, Debug, PartialEq)]
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
        let source =
            SyncAggregateSource::<Empty, _, _>::new(
                trivial::NullEventStore::<usize, usize>::default(),
                trivial::NullSnapshotStore::<Empty, usize>::default()
            );

        assert_eq!(source.rehydrate(&0), Ok(None));
    }

    #[test]
    fn can_create_events() {
        let source =
            SyncAggregateSource::<Empty, _, _>::new_with_events_only(
                trivial::NullEventStore::<usize, usize>::default(),
            );

        assert_eq!(source.rehydrate(&0), Ok(None));
    }

    #[test]
    fn can_create_snapshots() {
        let source =
            SyncAggregateSource::<Empty, _, _>::new_with_snapshots_only(
                trivial::NullSnapshotStore::<Empty, usize>::default()
            );

        assert_eq!(source.rehydrate(&0), Ok(None));
    }
}
