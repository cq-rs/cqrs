use super::{Precondition, Since, AggregateVersion, Version};
use super::{EventSource, EventAppend, SnapshotSource, SnapshotPersist, EventDecorator};
use super::{PersistedSnapshot, PersistedEvent};
use domain::{Aggregate, SnapshotChoice};
use std::rc::Rc;
use std::marker::PhantomData;
use std::error;
use std::fmt;

pub trait AggregateCommand<Agg: Aggregate> {
    type AggregateId;
    type Error: error::Error;

    fn execute(&self, agg_id: &Self::AggregateId, command: Agg::Command) -> Result<usize, Self::Error>;
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum LoadAggregateError<EErr, SErr> {
    Snapshot(SErr),
    Events(EErr),
}

impl<EErr, SErr> fmt::Display for LoadAggregateError<EErr, SErr>
    where
        EErr: error::Error,
        SErr: error::Error,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = self as &error::Error;
        f.write_str(err.description())?;
        f.write_str(": ")?;
        write!(f, "{}", err.cause().unwrap())
    }
}

impl<EErr, SErr> error::Error for LoadAggregateError<EErr, SErr>
    where
        EErr: error::Error,
        SErr: error::Error,
{
    fn description(&self) -> &str {
        match *self {
            LoadAggregateError::Snapshot(_) => "loading snapshot",
            LoadAggregateError::Events(_) => "loading events",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            LoadAggregateError::Snapshot(ref e) => Some(e),
            LoadAggregateError::Events(ref e) => Some(e),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct HydratedAggregate<Agg: Aggregate> {
    pub version: AggregateVersion,
    pub aggregate: Agg,
}

impl<Agg: Aggregate<Event=Event>, Event> HydratedAggregate<Agg> {
    pub fn apply(&mut self, event: PersistedEvent<Event>) {
        self.aggregate.apply(event.event);
        self.version = event.version.into();
    }

    #[inline]
    pub fn is_initial(&self) -> bool {
        self.version == AggregateVersion::Initial
    }
}

pub trait AggregateQuery<Agg: Aggregate> {
    type AggregateId;
    type Error: error::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error>;
}

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

impl<Agg, ESource> AggregateQuery<Agg> for EventsOnlyAggregateView<ESource>
    where
        Agg: Aggregate,
        ESource: EventSource<Event=Agg::Event>,
{
    type AggregateId = ESource::AggregateId;
    type Error = ESource::Error;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error> {
        let events =
            self.source.read_events(&agg_id, Since::BeginningOfStream)?;

        let mut snapshot = HydratedAggregate::default();

        if let Some(evts) = events {
            for event in evts {
                snapshot.apply(event);
            }
        }

        Ok(snapshot)
    }
}

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

impl<Agg, ESource, SSource> AggregateQuery<Agg> for SnapshotPlusEventsAggregateView<ESource, SSource>
    where
        Agg: Aggregate,
        ESource: EventSource<Event=Agg::Event>,
        SSource: SnapshotSource<Snapshot=Agg::Snapshot, AggregateId=ESource::AggregateId>,
{
    type AggregateId = ESource::AggregateId;
    type Error = LoadAggregateError<ESource::Error, SSource::Error>;

    fn rehydrate(&self, agg_id: &Self::AggregateId) -> Result<HydratedAggregate<Agg>, Self::Error> {
        let mut snapshot =
            self.snapshot_source.rehydrate(&agg_id)
                .map_err(|e| LoadAggregateError::Snapshot(e))?;

        let events =
            self.event_source.read_events(&agg_id, snapshot.version.into())
                .map_err(|e| LoadAggregateError::Events(e))?;

        if let Some(evts) = events {
            for event in evts {
                snapshot.apply(event);
            }
        }

        Ok(snapshot)
    }
}

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
                aggregate: Agg::from_snapshot(s.data),
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

#[derive(Debug, Hash, PartialEq, Clone)]
pub struct AggregateStore<Agg, ES, EA, SS, SP>
    where
        ES: EventSource<Event=Agg::Event>,
        EA: EventAppend<AggregateId=ES::AggregateId, Event=Agg::Event>,
        SS: SnapshotSource<AggregateId=ES::AggregateId, Snapshot=Agg::Snapshot>,
        SP: SnapshotPersist<AggregateId=ES::AggregateId, Snapshot=Agg::Snapshot>,
        Agg: Aggregate
{
    event_source: ES,
    event_append: EA,
    snapshot_source: SS,
    snapshot_persist: SP,
    _phantom: PhantomData<Agg>,
}

#[derive(Debug, Hash, PartialEq, Clone)]
pub enum AggregateError<CmdErr, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr> {
    BadCommand(CmdErr),
    ReadStream(ReadStreamErr),
    ReadState(ReadStateErr),
    WriteStream(WriteStreamErr),
    WriteState(WriteStateErr),
}

impl<Agg, ES, SS> AggregateStore<Agg, ES, ES, SS, SS>
    where
        ES: EventSource<Event=Agg::Event> + EventAppend<AggregateId=<ES as EventSource>::AggregateId, Event=<ES as EventSource>::Event>,
        SS: SnapshotSource<Snapshot=Agg::Snapshot, AggregateId=<ES as EventSource>::AggregateId> + SnapshotPersist<AggregateId=<ES as EventSource>::AggregateId, Snapshot=<SS as SnapshotSource>::Snapshot>,
        Agg: Aggregate,
{
    pub fn new(event_store: ES, snapshot_store: SS) -> AggregateStore<Agg, Rc<ES>, Rc<ES>, Rc<SS>, Rc<SS>> {
        let es = Rc::new(event_store);
        let ss = Rc::new(snapshot_store);
        AggregateStore {
            event_source: Rc::clone(&es),
            event_append: es,
            snapshot_source: Rc::clone(&ss),
            snapshot_persist: ss,
            _phantom: PhantomData,
        }
    }
}

impl<Agg, ES, EA, SS, SP> AggregateStore<Agg, ES, EA, SS, SP>
    where
        ES: EventSource<Event=Agg::Event>,
        EA: EventAppend<AggregateId=ES::AggregateId, Event=Agg::Event>,
        SS: SnapshotSource<AggregateId=ES::AggregateId, Snapshot=Agg::Snapshot>,
        SP: SnapshotPersist<AggregateId=ES::AggregateId, Snapshot=Agg::Snapshot>,
        Agg: Aggregate,
        Agg::Event: Clone + Sized,
{
    pub fn execute_and_persist<D>(&self, agg_id: &ES::AggregateId, cmd: Agg::Command, decorator: D) -> Result<usize, AggregateError<Agg::CommandError, ES::Error, SS::Error, EA::Error, SP::Error>>
        where
            D: EventDecorator<Event=Agg::Event, DecoratedEvent=Agg::Event>,
    {
        let saved_snapshot = self.get_last_snapshot(&agg_id)?;

        // Instantiate aggregate with snapshot or default data
        let (snapshot_version, mut state) =
            if let Some(snapshot) = saved_snapshot {
                (Some(snapshot.version), Agg::from_snapshot(snapshot.data))
            } else {
                (None, Agg::default())
            };

        let (read_since, mut version) =
            if let Some(v) = snapshot_version {
                (Since::Version(v), Some(v))
            } else {
                (Since::BeginningOfStream, None)
            };

        if let Some(event_version) = self.rehydrate(&agg_id, &mut state, read_since)? {
            version = Some(event_version);
        }

        // Apply command to aggregate
        let events =
            state.execute(cmd)
                .map_err(|e| AggregateError::BadCommand(e))?;

        let event_count = events.len();

        // Skip if no new events
        if event_count > 0 {
            let decorated_events = decorator.decorate_events(events);

            let precondition =
                if let Some(v) = version {
                    Precondition::LastVersion(v)
                } else {
                    Precondition::EmptyStream
                };

            // Append new events to event store if underlying stream
            // has not changed
            self.event_append.append_events(&agg_id, &decorated_events, precondition)
                .map_err(|e| AggregateError::WriteStream(e))?;

            for e in decorated_events {
                state.apply(e)
            }

            if state.should_snapshot() == SnapshotChoice::Persist {
                let new_snapshot_version =
                    if let Some(v) = version {
                        v + event_count
                    } else {
                        Version::new(event_count - 1)
                    };

                self.snapshot_persist.persist_snapshot(&agg_id, new_snapshot_version, state.snapshot())
                    .map_err(|e| AggregateError::WriteState(e))?;
            }
        }

        Ok(event_count)
    }

    fn rehydrate(&self, agg_id: &ES::AggregateId, agg: &mut Agg, since: Since) -> Result<Option<Version>, AggregateError<Agg::CommandError, ES::Error, SS::Error, EA::Error, SP::Error>> {
        let read_events =
            self.event_source.read_events(agg_id, since)
                .map_err(|e| AggregateError::ReadStream(e))?;

        let mut latest_version = None;
        if let Some(events) = read_events {
            // Re-hydrate aggregate
            for e in events {
                agg.apply(e.event);
                latest_version = Some(e.version);
            }
        }

        Ok(latest_version)
    }

    fn get_last_snapshot(&self, agg_id: &ES::AggregateId) -> Result<Option<PersistedSnapshot<SS::Snapshot>>, AggregateError<Agg::CommandError, ES::Error, SS::Error, EA::Error, SP::Error>> {
        self.snapshot_source.get_snapshot(&agg_id)
            .map_err(|e| AggregateError::ReadState(e))
    }
}

