use super::{Aggregate, RestoreAggregate, HydratedAggregate};
use super::super::{Since, EventSource, SnapshotSource, VersionedEvent};
use error::LoadAggregateError;
use std::borrow::Borrow;
use std::error;
use std::marker::PhantomData;

pub trait QueryableAggregate: Aggregate {
    fn events_view<ESource>(event_source: ESource) -> EventsView<Self, ESource>
        where
            ESource: EventSource<Event=<Self as Aggregate>::Event>,
            ESource::Events: Borrow<[VersionedEvent<ESource::Event>]> + IntoIterator<Item=VersionedEvent<ESource::Event>>,
    {
        EventsView {
            event_source,
            _phantom: PhantomData,
        }
    }
}

pub trait QueryableSnapshotAggregate: RestoreAggregate {
    fn snapshot_view<SSource>(snapshot_source: SSource) -> SnapshotView<Self, SSource>
        where
            SSource: SnapshotSource<Snapshot=<Self as RestoreAggregate>::Snapshot>,
    {
        SnapshotView {
            snapshot_source,
            _phantom: PhantomData,
        }
    }

    fn snapshot_with_events_view<ESource, SSource>(event_source: ESource, snapshot_source: SSource) -> SnapshotAndEventsView<Self, ESource, SSource>
        where
            ESource: EventSource<Event=<Self as Aggregate>::Event>,
            ESource::Events: Borrow<[VersionedEvent<ESource::Event>]> + IntoIterator<Item=VersionedEvent<ESource::Event>>,
            SSource: SnapshotSource<Snapshot=<Self as RestoreAggregate>::Snapshot, AggregateId=ESource::AggregateId>,
    {
        SnapshotAndEventsView {
            event_source,
            snapshot_source: Self::snapshot_view(snapshot_source),
            _phantom: PhantomData,
        }
    }
}

impl<T: Aggregate> QueryableAggregate for T {}
impl<T: RestoreAggregate> QueryableSnapshotAggregate for T {}

#[derive(Debug, Clone, PartialEq)]
pub struct EventsView<Agg, ESource>
    where
        Agg: Aggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: Borrow<[VersionedEvent<Agg::Event>]> + IntoIterator<Item=VersionedEvent<Agg::Event>>,
{
    event_source: ESource,
    _phantom: PhantomData<Agg>
}

impl<Agg, ESource> EventsView<Agg, ESource>
    where
        Agg: Aggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: Borrow<[VersionedEvent<Agg::Event>]> + IntoIterator<Item=VersionedEvent<Agg::Event>>,
{
    pub fn rehydrate(&self, agg_id: &ESource::AggregateId) -> Result<HydratedAggregate<Agg>, ESource::Error> {
        let events =
            self.event_source.read_events(agg_id, Since::BeginningOfStream)?;

        let mut snapshot = HydratedAggregate::default();

        if let Some(evts) = events {
            for event in evts.into_iter() {
                snapshot.apply(event);
            }
        }

        Ok(snapshot)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotView<Agg, SSource>
    where
        Agg: RestoreAggregate,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot>,
{
    snapshot_source: SSource,
    _phantom: PhantomData<Agg>
}

impl<Agg, SSource> SnapshotView<Agg, SSource>
    where
        Agg: RestoreAggregate,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot>,
{
    pub fn rehydrate(&self, agg_id: &SSource::AggregateId) -> Result<HydratedAggregate<Agg>, SSource::Error> {
        let aggregate = HydratedAggregate::restore(self.snapshot_source.get_snapshot(agg_id)?);
        Ok(aggregate)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotAndEventsView<Agg, ESource, SSource>
    where
        Agg: RestoreAggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: Borrow<[VersionedEvent<Agg::Event>]> + IntoIterator<Item=VersionedEvent<Agg::Event>>,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot, AggregateId=ESource::AggregateId>,
{
    event_source: ESource,
    snapshot_source: SnapshotView<Agg, SSource>,
    _phantom: PhantomData<Agg>
}

impl<Agg, ESource, SSource> SnapshotAndEventsView<Agg, ESource, SSource>
    where
        Agg: RestoreAggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: Borrow<[VersionedEvent<Agg::Event>]> + IntoIterator<Item=VersionedEvent<Agg::Event>>,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot, AggregateId=ESource::AggregateId>,
{
    pub fn rehydrate(&self, agg_id: &ESource::AggregateId) -> Result<HydratedAggregate<Agg>, LoadAggregateError<ESource::Error, SSource::Error>> {
        let mut snapshot =
            self.snapshot_source.rehydrate(agg_id)
                .map_err(LoadAggregateError::Snapshot)?;

        let events =
            self.event_source.read_events(agg_id, snapshot.version.into())
                .map_err(LoadAggregateError::Events)?;

        if let Some(evts) = events {
            for event in evts.into_iter() {
                snapshot.apply(event);
            }
        }

        Ok(snapshot)
    }
}

pub trait AggregateQuery<Agg: Aggregate> {
    type AggregateId;
    type Error: error::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error>;
}

impl<Agg, ESource> AggregateQuery<Agg> for EventsView<Agg, ESource>
    where
        Agg: Aggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: Borrow<[VersionedEvent<ESource::Event>]> + IntoIterator<Item=VersionedEvent<ESource::Event>>,
{
    type AggregateId = ESource::AggregateId;
    type Error = ESource::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error> {
        EventsView::rehydrate(&self, agg_id)
    }
}


impl<Agg, SSource> AggregateQuery<Agg> for SnapshotView<Agg, SSource>
    where
        Agg: RestoreAggregate,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot>,
{
    type AggregateId = SSource::AggregateId;
    type Error = SSource::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error> {
        SnapshotView::rehydrate(&self, agg_id)
    }
}


impl<Agg, ESource, SSource> AggregateQuery<Agg> for SnapshotAndEventsView<Agg, ESource, SSource>
    where
        Agg: RestoreAggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: Borrow<[VersionedEvent<ESource::Event>]> + IntoIterator<Item=VersionedEvent<ESource::Event>>,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot, AggregateId=ESource::AggregateId>,
{
    type AggregateId = ESource::AggregateId;
    type Error = LoadAggregateError<ESource::Error, SSource::Error>;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error> {
        SnapshotAndEventsView::rehydrate(&self, agg_id)
    }
}

