use super::{Aggregate, SnapshotAggregate};
use super::query::AggregateQuery;
use super::command::Executor;
use error::{PersistAggregateError, CommandAggregateError};
use super::super::{EventAppend, EventDecorator, SnapshotPersist, Precondition};
use trivial::{NullEventStore,NullSnapshotStore};

use std::borrow::Borrow;
use std::marker::PhantomData;

// Add View as first argument

pub trait PersistableAggregate: Aggregate {
    fn persist_events<View, EAppend>(view: View, ea: EAppend) -> EventsOnly<Self, View, EAppend>
        where
            View: AggregateQuery<Self>,
            EAppend: EventAppend<Event=<Self as Aggregate>::Event, AggregateId=View::AggregateId>,
    {
        EventsOnly {
            view,
            appender: ea,
            _phantom: PhantomData,
        }
    }
}

pub trait PersistableSnapshotAggregate: SnapshotAggregate {
    fn persist_snapshot<SPersist>(sp: SPersist) -> SnapshotOnly<Self, SPersist>
        where
            SPersist: SnapshotPersist<Snapshot=<Self as SnapshotAggregate>::Snapshot>,
    {
        EventsAndSnapshot {
            appender: Default::default(),
            persister: sp,
            _phantom: PhantomData,
        }
    }

    fn persist_events_and_snapshot<EAppend, SPersist>(ea: EAppend, sp: SPersist) -> EventsAndSnapshot<Self, EAppend, SPersist>
        where
            EAppend: EventAppend<Event=<Self as Aggregate>::Event>,
            SPersist: SnapshotPersist<Snapshot=<Self as SnapshotAggregate>::Snapshot, AggregateId=EAppend::AggregateId>,
    {
        EventsAndSnapshot {
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
    pub fn execute_and_persist_with_decorator<D: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>>(&self, agg_id: &View::AggregateId, command: Agg::Command, precondition: ::Precondition, decorator: D) -> Result<(), ::error::CommandAggregateError<Agg::CommandError, View::Error, EAppend::Error>> {
        let state_opt =
            self.view.rehydrate(agg_id)
                .map_err(CommandAggregateError::Load)?;

        if let Some(mut state) = state_opt {
            let command_events =
                state.aggregate.execute(command)
                    .map_err(CommandAggregateError::Command)?;

            let decorated_events =
                decorator.decorate_events(command_events);

            self.appender.append_events(agg_id, &decorated_events, state.version.into())
//                .map_err(PersistAggregateError::Events)
                .map_err(CommandAggregateError::Persist)?;

/*            for event in command_events.into_iter() {
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
        } else {
            Err(CommandAggregateError::AggregateNotFound)
        }
    }
}

struct PseudoSnapshotAggregate<Agg: Aggregate>(Agg);

type SnapshotOnly<Agg: Aggregate, SPersist: SnapshotPersist> = EventsAndSnapshot<Agg, ::trivial::NullEventStore<Agg::Event, SPersist::AggregateId>, SPersist>;

pub struct EventsAndSnapshot<Agg, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        EAppend: EventAppend<Event=Agg::Event>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=EAppend::AggregateId>,
{
    appender: EAppend,
    persister: SPersist,
    _phantom: PhantomData<Agg>,
}

pub trait ExecuteAndPersist {

}