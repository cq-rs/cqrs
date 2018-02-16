use super::{Aggregate, AggregateVersion, HydratedAggregate};
use super::super::{Since, EventSource, SnapshotSource, VersionedEvent};
use error::LoadAggregateError;
use std::borrow::Borrow;
use std::error;

pub trait AggregateQuery<Agg: Aggregate> {
    type AggregateId;
    type Error: error::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventsOnlyAggregateView<ESource: EventSource> {
    source: ESource,
}

impl<ESource: EventSource> EventsOnlyAggregateView<ESource> {
    pub fn new(event_source: ESource) -> Self {
        EventsOnlyAggregateView {
            source: event_source,
        }
    }
}

impl<Agg, Event, ESource> AggregateQuery<Agg> for EventsOnlyAggregateView<ESource>
    where
        Agg: Aggregate<Event=Event>,
        Agg::Events: IntoIterator<Item=Event>,
        ESource: EventSource<Event=Event>,
        ESource::Events: Borrow<[VersionedEvent<Event>]> + IntoIterator<Item=VersionedEvent<Event>>,
{
    type AggregateId = ESource::AggregateId;
    type Error = ESource::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error> {
        let events =
            self.source.read_events(agg_id, Since::BeginningOfStream)?;

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
pub struct SnapshotPlusEventsAggregateView<ESource, SSource>
    where
        ESource: EventSource,
        SSource: SnapshotSource<AggregateId=ESource::AggregateId>,
{
    event_source: ESource,
    snapshot_source: SnapshotOnlyAggregateView<SSource>,
}

impl<ESource: EventSource, SSource: SnapshotSource> SnapshotPlusEventsAggregateView<ESource, SSource>
    where
        ESource: EventSource,
        SSource: SnapshotSource<AggregateId=ESource::AggregateId>,
{
    pub fn new(event_source: ESource, snapshot_source: SSource) -> Self {
        SnapshotPlusEventsAggregateView {
            event_source,
            snapshot_source: SnapshotOnlyAggregateView::new(snapshot_source),
        }
    }
}

impl<Agg, Event, ESource, SSource> AggregateQuery<Agg> for SnapshotPlusEventsAggregateView<ESource, SSource>
    where
        Agg: Aggregate<Event=Event>,
        ESource: EventSource<Event=Event>,
        ESource::Events: Borrow<[VersionedEvent<Event>]> + IntoIterator<Item=VersionedEvent<Event>>,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot, AggregateId=ESource::AggregateId>,
{
    type AggregateId = ESource::AggregateId;
    type Error = LoadAggregateError<ESource::Error, SSource::Error>;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error> {
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

#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotOnlyAggregateView<Source: SnapshotSource> {
    source: Source,
}

impl<Source: SnapshotSource> SnapshotOnlyAggregateView<Source> {
    pub fn new(source: Source) -> Self {
        SnapshotOnlyAggregateView {
            source,
        }
    }
}

impl<Agg, Source> AggregateQuery<Agg> for SnapshotOnlyAggregateView<Source>
    where
        Agg: Aggregate,
        Source: SnapshotSource<Snapshot=Agg::Snapshot>,
{
    type AggregateId = Source::AggregateId;
    type Error = Source::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error> {
        let snapshot = self.source.get_snapshot(agg_id)?;
        if let Some(s) = snapshot {
            Ok(HydratedAggregate {
                aggregate: Agg::from_snapshot(s.snapshot),
                version: AggregateVersion::Version(s.version),
            })
        } else {
            Ok(HydratedAggregate {
                aggregate: Agg::default(),
                version: AggregateVersion::Initial,
            })
        }
    }
}
