use super::super::{EventAppend, SnapshotPersist, EventDecorator};
use super::{Aggregate, HydratedAggregate, SnapshotAggregate};
use super::query::AggregateQuery;
use error::{CommandAggregateError, ExecuteError, PersistAggregateError};
use trivial::NopEventDecorator;
use std::borrow::Borrow;
use std::error;
use std::marker::PhantomData;

pub trait Executor<Agg>
    where
        Agg: Aggregate,
{
    type AggregateId;
    type Error: error::Error;

    fn execute_on_default(command: Agg::Command) -> Result<AggregateWithNewEvents<Agg>, Agg::CommandError>;
    fn execute_on_current_or_default(&self, agg_id: &Self::AggregateId, command: Agg::Command) -> Result<AggregateWithNewEvents<Agg>, ExecuteError<Agg::CommandError, Self::Error>>;
    fn execute_on_current(&self, agg_id: &Self::AggregateId, command: Agg::Command) -> Result<AggregateWithNewEvents<Agg>, ExecuteError<Agg::CommandError, Self::Error>>;
}

pub struct AggregateWithNewEvents<Agg: Aggregate> {
    state: HydratedAggregate<Agg>,
    new_events: Agg::Events,
}

pub struct CommandExecutor<Agg, View>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
{
    view: View,
    _phantom: PhantomData<Agg>,
}

impl<Agg, View> CommandExecutor<Agg, View>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
{
    pub fn from_view(view: View) -> Self {
        CommandExecutor {
            view,
            _phantom: PhantomData,
        }
    }

    pub fn with_decorator<D: EventDecorator<Event=Agg::Event>>(self) -> DecoratedCommandExecutor<Agg, View, D> {
        DecoratedCommandExecutor {
            view: self.view,
            _phantom: PhantomData,
        }
    }
}

impl<Agg, View> Executor<Agg> for CommandExecutor<Agg, View>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
{
    type AggregateId = View::AggregateId;
    type Error = View::Error;

    fn execute_on_default(command: Agg::Command) -> Result<AggregateWithNewEvents<Agg>, Agg::CommandError> {
        let state: HydratedAggregate<Agg> = Default::default();

        let new_events = state.aggregate.execute(command)?;

        Ok(AggregateWithNewEvents { state, new_events })
    }

    fn execute_on_current_or_default(&self, agg_id: &Self::AggregateId, command: Agg::Command) -> Result<AggregateWithNewEvents<Agg>, ExecuteError<Agg::CommandError, Self::Error>> {
        let state =
            self.view.rehydrate(agg_id)
                .map_err(ExecuteError::Load)?
                .unwrap_or_default();

        let new_events =
            state.aggregate.execute(command)
                .map_err(ExecuteError::Command)?;

        Ok(AggregateWithNewEvents { state, new_events })
    }

    fn execute_on_current(&self, agg_id: &Self::AggregateId, command: Agg::Command) -> Result<AggregateWithNewEvents<Agg>, ExecuteError<Agg::CommandError, Self::Error>> {
        let state_opt =
            self.view.rehydrate(agg_id)
                .map_err(ExecuteError::Load)?;

        if let Some(state) = state_opt {
            let new_events =
                state.aggregate.execute(command)
                    .map_err(ExecuteError::Command)?;

            Ok(AggregateWithNewEvents { state, new_events })
        } else {
            Err(ExecuteError::AggregateNotFound)
        }
    }
}

pub struct DecoratedCommandExecutor<Agg, View, D>
    where
        Agg: Aggregate,
        View: AggregateQuery<Agg>,
        D: EventDecorator<Event=Agg::Event>
{
    view: View,
    _phantom: PhantomData<(Agg, D)>,
}

pub trait AggregateCommand<Agg: Aggregate> {
    type AggregateId;
    type Error: error::Error;

    fn execute(&self, agg_id: &Self::AggregateId, command: Agg::Command) -> Result<usize, Self::Error>;
}

pub trait DecoratedAggregateCommand<Agg: Aggregate, Decorator: EventDecorator<Event=Agg::Event>> {
    type AggregateId;
    type Error: error::Error;

    fn execute_with_decorator(&self, agg_id: &Self::AggregateId, command: Agg::Command, decorator: Decorator) -> Result<usize, Self::Error>;
    fn execute_new_with_decorator(&self, agg_id: &Self::AggregateId, command: Agg::Command, decorator: Decorator) -> Result<usize, Self::Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistAndSnapshotAggregateCommander<Agg, Query, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        Query: AggregateQuery<Agg>,
        EAppend: EventAppend<AggregateId=Query::AggregateId>,
        SPersist: SnapshotPersist<AggregateId=Query::AggregateId, Snapshot=Agg::Snapshot>,
{
    query: Query,
    appender: EAppend,
    persister: SPersist,
    _phantom_aggregate: PhantomData<Agg>,
}

impl<Agg, Query, EAppend, SPersist> PersistAndSnapshotAggregateCommander<Agg, Query, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        Query: AggregateQuery<Agg>,
        EAppend: EventAppend<AggregateId=Query::AggregateId>,
        SPersist: SnapshotPersist<AggregateId=Query::AggregateId, Snapshot=Agg::Snapshot>,
{
    pub fn new(query: Query, event_append: EAppend, snapshot_persist: SPersist) -> Self {
        PersistAndSnapshotAggregateCommander {
            query,
            appender: event_append,
            persister: snapshot_persist,
            _phantom_aggregate: PhantomData,
        }
    }
}

impl<Agg, Query, EAppend, SPersist, Decorator> DecoratedAggregateCommand<Agg, Decorator> for PersistAndSnapshotAggregateCommander<Agg, Query, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        Agg::Events: Borrow<[Agg::Event]> + IntoIterator<Item=Agg::Event>,
        Query: AggregateQuery<Agg>,
        EAppend: EventAppend<AggregateId=Query::AggregateId, Event=Agg::Event>,
        SPersist: SnapshotPersist<AggregateId=Query::AggregateId, Snapshot=Agg::Snapshot>,
        Agg::CommandError: error::Error,
        Decorator: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>,
{
    type AggregateId = Query::AggregateId;
    type Error = CommandAggregateError<Agg::CommandError, Query::Error, PersistAggregateError<EAppend::Error, SPersist::Error>>;

    fn execute_new_with_decorator(&self, agg_id: &Self::AggregateId, command: Agg::Command, decorator: Decorator) -> Result<usize, Self::Error> {
        let mut state = HydratedAggregate::<Agg>::default();

        let command_events =
            state.aggregate.execute(command)
                .map_err(CommandAggregateError::Command)?;

        let event_count = command_events.borrow().len();

        let decorated_events =
            decorator.decorate_events(command_events);

        self.appender.append_events(agg_id, &decorated_events, state.version.into())
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

        Ok(event_count)
    }

    fn execute_with_decorator(&self, agg_id: &Self::AggregateId, command: Agg::Command, decorator: Decorator) -> Result<usize, Self::Error> {
        let state_opt =
            self.query.rehydrate(agg_id)
                .map_err(CommandAggregateError::Load)?;

        if let Some(mut state) = state_opt {
            let command_events =
                state.aggregate.execute(command)
                    .map_err(CommandAggregateError::Command)?;

            let event_count = command_events.borrow().len();

            let decorated_events =
                decorator.decorate_events(command_events);

            self.appender.append_events(agg_id, &decorated_events, state.version.into())
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

            Ok(event_count)
        } else {
            Err(CommandAggregateError::AggregateNotFound)
        }
    }
}

