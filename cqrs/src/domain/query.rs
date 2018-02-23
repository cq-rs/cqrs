use super::{Aggregate, RestoreAggregate, HydratedAggregate};
use super::super::{Since, EventSource, SnapshotSource, VersionedEvent};
use error::{LoadAggregateError};
use std::borrow::Borrow;
use std::error;
use std::marker::PhantomData;

pub trait QueryableAggregate: Aggregate {
    fn events_view<ESource>(event_source: ESource) -> EventsView<Self, ESource>
        where
            ESource: EventSource<Event=<Self as Aggregate>::Event>,
            ESource::Events: IntoIterator<Item=VersionedEvent<ESource::Event>>,
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
            ESource::Events: IntoIterator<Item=VersionedEvent<ESource::Event>>,
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
        ESource::Events: IntoIterator<Item=VersionedEvent<Agg::Event>>,
{
    event_source: ESource,
    _phantom: PhantomData<Agg>
}

impl<Agg, ESource> EventsView<Agg, ESource>
    where
        Agg: Aggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: IntoIterator<Item=VersionedEvent<Agg::Event>>,
{
    pub fn rehydrate(&self, agg_id: &ESource::AggregateId) -> Result<Option<HydratedAggregate<Agg>>, ESource::Error> {
        let events =
            self.event_source.read_events(agg_id, Since::BeginningOfStream)?;

        if let Some(evts) = events {
            let mut snapshot = HydratedAggregate::default();
            for event in evts.into_iter() {
                snapshot.apply(event);
            }
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
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
    pub fn rehydrate(&self, agg_id: &SSource::AggregateId) -> Result<Option<HydratedAggregate<Agg>>, SSource::Error> {
        let aggregate =
            self.snapshot_source.get_snapshot(agg_id)?
                .map(|s| s.into());
        Ok(aggregate)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotAndEventsView<Agg, ESource, SSource>
    where
        Agg: RestoreAggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: IntoIterator<Item=VersionedEvent<Agg::Event>>,
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
        ESource::Events: IntoIterator<Item=VersionedEvent<Agg::Event>>,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot, AggregateId=ESource::AggregateId>,
{
    pub fn rehydrate(&self, agg_id: &ESource::AggregateId) -> Result<Option<HydratedAggregate<Agg>>, LoadAggregateError<ESource::Error, SSource::Error>> {
        let snapshot_opt =
            self.snapshot_source.rehydrate(agg_id)
                .map_err(LoadAggregateError::Snapshot)?;

        let since =
            if let Some(ref s) = snapshot_opt {
                s.version.into()
            } else {
                Since::BeginningOfStream
            };

        let events_opt =
            self.event_source.read_events(agg_id, since)
                .map_err(LoadAggregateError::Events)?;

        if snapshot_opt.is_none() && events_opt.is_none() {
            Ok(None)
        } else {
            let mut aggregate = snapshot_opt.unwrap_or_default();

            if let Some(events) = events_opt {
                for event in events.into_iter() {
                    aggregate.apply(event);
                }
            }

            Ok(Some(aggregate))
        }
    }
}

pub trait AggregateQuery<Agg: Aggregate>: Sized {
    type AggregateId: ?Sized;
    type Error: error::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<Option<HydratedAggregate<Agg>>, Self::Error>;
}

impl<Agg, ESource> AggregateQuery<Agg> for EventsView<Agg, ESource>
    where
        Agg: Aggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: IntoIterator<Item=VersionedEvent<ESource::Event>>,
{
    type AggregateId = ESource::AggregateId;
    type Error = ESource::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<Option<HydratedAggregate<Agg>>, Self::Error> {
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

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<Option<HydratedAggregate<Agg>>, Self::Error> {
        SnapshotView::rehydrate(&self, agg_id)
    }
}


impl<Agg, ESource, SSource> AggregateQuery<Agg> for SnapshotAndEventsView<Agg, ESource, SSource>
    where
        Agg: RestoreAggregate,
        ESource: EventSource<Event=Agg::Event>,
        ESource::Events: IntoIterator<Item=VersionedEvent<ESource::Event>>,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot, AggregateId=ESource::AggregateId>,
{
    type AggregateId = ESource::AggregateId;
    type Error = LoadAggregateError<ESource::Error, SSource::Error>;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<Option<HydratedAggregate<Agg>>, Self::Error> {
        SnapshotAndEventsView::rehydrate(&self, agg_id)
    }
}

