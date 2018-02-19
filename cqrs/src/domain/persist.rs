use super::{Aggregate, SnapshotAggregate, HydratedAggregate, AggregatePrecondition};
use super::query::AggregateQuery;
use super::command::Executor;
use error::{PersistAggregateError, CommandAggregateError};
use super::super::{EventAppend, EventDecorator, SnapshotPersist, Precondition};
use trivial::{NullEventStore,NullSnapshotStore};

use std::borrow::Borrow;
use std::error;
use std::marker::PhantomData;

// Add View as first argument

pub trait PersistableAggregate: Aggregate {
    fn persist_events<View, EAppend>(view: View, ea: EAppend) -> EventsOnly<Self, View, EAppend>
        where
            View: AggregateQuery<Self>,
            EAppend: EventAppend<AggregateId=View::AggregateId>,
    {
        EventsOnly {
            view,
            appender: ea,
            _phantom: PhantomData,
        }
    }
}

pub trait PersistableSnapshotAggregate: SnapshotAggregate {
    fn persist_snapshot<View, SPersist>(view: View, sp: SPersist) -> SnapshotOnly<Self, View, SPersist>
        where
            View: AggregateQuery<Self>,
            SPersist: SnapshotPersist<Snapshot=<Self as SnapshotAggregate>::Snapshot, AggregateId=View::AggregateId>,
    {
        EventsAndSnapshot {
            view,
            appender: Default::default(),
            persister: sp,
            _phantom: PhantomData,
        }
    }

    fn persist_events_and_snapshot<View, EAppend, SPersist>(view: View, ea: EAppend, sp: SPersist) -> EventsAndSnapshot<Self, View, EAppend, SPersist>
        where
            View: AggregateQuery<Self>,
            EAppend: EventAppend<AggregateId=View::AggregateId>,
            SPersist: SnapshotPersist<Snapshot=<Self as SnapshotAggregate>::Snapshot, AggregateId=View::AggregateId>,
    {
        EventsAndSnapshot {
            view,
            appender: ea,
            persister: sp,
            _phantom: PhantomData,
        }
    }
}

impl<Agg: Aggregate> PersistableAggregate for Agg {}
impl<Agg: SnapshotAggregate> PersistableSnapshotAggregate for Agg {}

pub struct EventsOnly<Agg, View, EAppend>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
        EAppend: EventAppend<AggregateId=View::AggregateId>,
{
    view: View,
    appender: EAppend,
    _phantom: PhantomData<Agg>,
}

fn verify_precondition<Agg: Aggregate, ViewError: error::Error, AppendError: error::Error>(state_opt: &Option<HydratedAggregate<Agg>>, precondition: AggregatePrecondition) -> Result<(), CommandAggregateError<Agg::CommandError, ViewError, AppendError>> {
    if let Some(ref state) = *state_opt {
        match precondition {
            AggregatePrecondition::ExpectedVersion(v) if v == state.get_version() => Ok(()),
            _ => Err(CommandAggregateError::PreconditionFailed(precondition)),
        }
    } else if precondition == AggregatePrecondition::New {
        Ok(())
    } else {
        Err(CommandAggregateError::PreconditionFailed(precondition))
    }
}

impl<Agg, View, EAppend> EventsOnly<Agg, View, EAppend>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
        EAppend: EventAppend<Event=Agg::Event, AggregateId=View::AggregateId>,
{
    pub fn execute_and_persist(&self, agg_id: &View::AggregateId, command: Agg::Command, precondition: ::Precondition) -> Result<(), ::error::CommandAggregateError<Agg::CommandError, View::Error, EAppend::Error>> {
        Ok(())
    }
}

impl<Agg, View, EAppend> EventsOnly<Agg, View, EAppend>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
        EAppend: EventAppend<AggregateId=View::AggregateId>,
        Agg::Events: Borrow<[Agg::Event]> + IntoIterator<Item=Agg::Event>
{
    pub fn execute_and_persist_with_decorator<D: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>>(&self, agg_id: &View::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>, decorator: D) -> Result<(), ::error::CommandAggregateError<Agg::CommandError, View::Error, EAppend::Error>> {
        let state_opt =
            self.view.rehydrate(agg_id)
                .map_err(CommandAggregateError::Load)?;

        if let Some(precondition) = precondition {
            verify_precondition(&state_opt, precondition)?;
        }

        let mut state = state_opt.unwrap_or_default();

        let command_events =
            state.aggregate.execute(command)
                .map_err(CommandAggregateError::Command)?;

        let decorated_events =
            decorator.decorate_events(command_events);

        self.appender.append_events(agg_id, &decorated_events, Some(state.version.into()))
//            .map_err(PersistAggregateError::Events)
            .map_err(CommandAggregateError::Persist)?;

/*
        for event in command_events.into_iter() {
            state.aggregate.apply(event);
            state.version += 1;
        }

        if let Some(snapshot) = state.to_snapshot() {
            self.persister.persist_snapshot(agg_id, snapshot)
                .map_err(PersistAggregateError::Snapshot)
                .map_err(CommandAggregateError::Persist)?;
        }
*/
        Ok(())
    }
}

struct PseudoSnapshotAggregate<Agg: Aggregate>(Agg);

type SnapshotOnly<Agg: SnapshotAggregate, View: AggregateQuery<Agg>, SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=View::AggregateId>> = EventsAndSnapshot<Agg, View, ::trivial::NullEventStore<Agg::Event, SPersist::AggregateId>, SPersist>;

pub struct EventsAndSnapshot<Agg, View, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        View: AggregateQuery<Agg>,
        EAppend: EventAppend<AggregateId=View::AggregateId>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=View::AggregateId>,
{
    view: View,
    appender: EAppend,
    persister: SPersist,
    _phantom: PhantomData<Agg>,
}

impl<Agg, View, EAppend, SPersist> EventsAndSnapshot<Agg, View, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        View: AggregateQuery<Agg>,
        EAppend: EventAppend<AggregateId=View::AggregateId, Event=Agg::Event>,
        Agg::Events: Borrow<[Agg::Event]> + IntoIterator<Item=Agg::Event>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=View::AggregateId>,
{
    pub fn execute_and_persist_with_decorator<D: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>>(&self, agg_id: &View::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>, decorator: D) -> Result<(), ::error::CommandAggregateError<Agg::CommandError, View::Error, ::error::PersistAggregateError<EAppend::Error, SPersist::Error>>> {
        let state_opt =
            self.view.rehydrate(agg_id)
                .map_err(CommandAggregateError::Load)?;

        if let Some(precondition) = precondition {
            verify_precondition(&state_opt, precondition)?;
        }

        let mut state = state_opt.unwrap_or_default();

        let command_events =
            state.aggregate.execute(command)
                .map_err(CommandAggregateError::Command)?;

        let decorated_events =
            decorator.decorate_events(command_events);

        self.appender.append_events(agg_id, &decorated_events, Some(state.version.into()))
            .map_err(PersistAggregateError::Events)
            .map_err(CommandAggregateError::Persist)?;

        for event in decorated_events.into_iter() {
            state.aggregate.apply(event);
            state.version += 1;
        }

        if let Some(snapshot) = state.to_snapshot() {
            self.persister.persist_snapshot(agg_id, snapshot)
                .map_err(PersistAggregateError::Snapshot)
                .map_err(CommandAggregateError::Persist)?;
        }

        Ok(())
    }
}
pub trait ExecuteAndPersist {

}