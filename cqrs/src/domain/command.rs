use super::super::{EventAppend, SnapshotPersist, EventDecorator};
use super::{Aggregate, AggregateVersion};
use super::query::AggregateQuery;
use error::{CommandAggregateError, PersistAggregateError};
use std::borrow::Borrow;
use std::error;
use std::marker::PhantomData;

pub trait AggregateCommand<Agg: Aggregate> {
    type AggregateId;
    type Error: error::Error;

    fn execute(&self, agg_id: &Self::AggregateId, command: Agg::Command) -> Result<usize, Self::Error>;
}

pub trait DecoratedAggregateCommand<Agg: Aggregate, Decorator: EventDecorator<Event=Agg::Event>> {
    type AggregateId;
    type Error: error::Error;

    fn execute_with_decorator(&self, agg_id: &Self::AggregateId, command: Agg::Command, decorator: Decorator) -> Result<usize, Self::Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistAndSnapshotAggregateCommander<Agg, Query, EAppend, SPersist>
    where
        Agg: Aggregate,
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
        Agg: Aggregate,
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
        Agg: Aggregate,
        Agg::Events: Borrow<[Agg::Event]> + IntoIterator<Item=Agg::Event>,
        Query: AggregateQuery<Agg>,
        EAppend: EventAppend<AggregateId=Query::AggregateId>,
        SPersist: SnapshotPersist<AggregateId=Query::AggregateId, Snapshot=Agg::Snapshot>,
        Agg::CommandError: error::Error,
        Decorator: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>,
{
    type AggregateId = Query::AggregateId;
    type Error = CommandAggregateError<Agg::CommandError, Query::Error, PersistAggregateError<EAppend::Error, SPersist::Error>>;

    fn execute_with_decorator(&self, agg_id: &Self::AggregateId, command: Agg::Command, decorator: Decorator) -> Result<usize, Self::Error> {
        let mut state =
            self.query.rehydrate(agg_id)
                .map_err(CommandAggregateError::Load)?;

        let command_events =
            state.aggregate.execute(command)
                .map_err(CommandAggregateError::Command)?;

        let event_count = command_events.borrow().len();

        let decorated_events =
            decorator.decorate_events(command_events.borrow());

        self.appender.append_events(agg_id, &decorated_events, state.version.into())
            .map_err(PersistAggregateError::Events)
            .map_err(CommandAggregateError::Persist)?;

        println!("version: {:?}", state.version);

        for event in command_events.into_iter() {
            state.aggregate.apply(event);
            state.version += 1;
        }

        println!("version: {:?}", state.version);

        if let AggregateVersion::Version(snapshot_version) = state.version {
            self.persister.persist_snapshot(agg_id, snapshot_version, state.aggregate.snapshot())
                .map_err(PersistAggregateError::Snapshot)
                .map_err(CommandAggregateError::Persist)?;
        }

        Ok(event_count)
    }
}

