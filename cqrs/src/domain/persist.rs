use super::{Aggregate, SnapshotAggregate, HydratedAggregate, AggregateVersion, AggregatePrecondition};
use super::query::AggregateQuery;
use super::execute::Executor;
use error::{PersistAggregateError, ExecuteAndPersistError, ExecuteError};
use super::super::{EventAppend, EventDecorator, SnapshotPersist, Precondition};
use trivial::{NullEventStore};

use std::borrow::Borrow;
use std::error;
use std::marker::PhantomData;

// Add View as first argument

pub trait PersistableAggregate: Aggregate {
    fn persist_events<Exec, EAppend>(executor: Exec, ea: EAppend) -> EventsOnly<Self, Exec, EAppend>
        where
            Exec: Executor<Self>,
            EAppend: EventAppend<AggregateId=Exec::AggregateId>,
    {
        EventsOnly {
            executor,
            appender: ea,
            _phantom: PhantomData,
        }
    }
}

pub trait PersistableSnapshotAggregate: SnapshotAggregate {
    fn persist_snapshot<Exec, SPersist>(executor: Exec, sp: SPersist) -> SnapshotOnly<Self, Exec, SPersist>
        where
            Exec: Executor<Self>,
            SPersist: SnapshotPersist<Snapshot=<Self as SnapshotAggregate>::Snapshot, AggregateId=Exec::AggregateId>,
    {
        EventsAndSnapshot {
            executor,
            appender: Default::default(),
            persister: sp,
            _phantom: PhantomData,
        }
    }

    fn persist_events_and_snapshot<Exec, EAppend, SPersist>(executor: Exec, ea: EAppend, sp: SPersist) -> EventsAndSnapshot<Self, Exec, EAppend, SPersist>
        where
            Exec: Executor<Self>,
            EAppend: EventAppend<AggregateId=Exec::AggregateId>,
            SPersist: SnapshotPersist<Snapshot=<Self as SnapshotAggregate>::Snapshot, AggregateId=Exec::AggregateId>,
    {
        EventsAndSnapshot {
            executor,
            appender: ea,
            persister: sp,
            _phantom: PhantomData,
        }
    }
}

impl<Agg: Aggregate> PersistableAggregate for Agg {}
impl<Agg: SnapshotAggregate> PersistableSnapshotAggregate for Agg {}

pub struct EventsOnly<Agg, Exec, EAppend>
    where
        Agg: Aggregate,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId>,
{
    executor: Exec,
    appender: EAppend,
    _phantom: PhantomData<Agg>,
}

impl<Agg, Exec, EAppend> EventsOnly<Agg, Exec, EAppend>
    where
        Agg: Aggregate,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId>,
        Agg::Events: IntoIterator<Item=Agg::Event>,
{
    pub fn execute_and_persist_with_decorator<D: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>>(&self, agg_id: &Exec::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>, decorator: D) -> Result<(), ExecuteAndPersistError<ExecuteError<Agg::CommandError, Exec::Error>, EAppend::Error>> {
        let execute_result =
            self.executor.execute(agg_id, command, precondition)?;

        let decorated_events =
            decorator.decorate_events(execute_result.command_events);

        let hydrated_aggregate = execute_result.hydrated_aggregate;

        let append_precondition =
            precondition.and_then(|p| {
                match p {
                    AggregatePrecondition::ExpectedVersion(AggregateVersion::Version(v)) => Some(Precondition::LastVersion(v)),
                    AggregatePrecondition::ExpectedVersion(AggregateVersion::Initial) => Some(Precondition::EmptyStream),
                    AggregatePrecondition::New => Some(Precondition::NewStream),
                    AggregatePrecondition::Exists => None,
                }
            }).unwrap_or_else(|| hydrated_aggregate.version.into());

        self.appender.append_events(agg_id, &decorated_events, Some(append_precondition))
            .map_err(ExecuteAndPersistError::Persist)?;

        Ok(())
    }
}

struct PseudoSnapshotAggregate<Agg: Aggregate>(Agg);

type SnapshotOnly<Agg: SnapshotAggregate, Exec: Executor<Agg>, SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>> = EventsAndSnapshot<Agg, Exec, NullEventStore<Agg::Event, SPersist::AggregateId>, SPersist>;

pub struct EventsAndSnapshot<Agg, Exec, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>,
{
    executor: Exec,
    appender: EAppend,
    persister: SPersist,
    _phantom: PhantomData<Agg>,
}

impl<Agg, Exec, EAppend, SPersist> EventsAndSnapshot<Agg, Exec, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        Agg::Event: Clone,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>,
{
    pub fn with_type_changing_decorator<D: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>>(self) -> EventsAndSnapshotWithTypeChangingDecorator<Agg, Exec, EAppend, SPersist, D> {
        EventsAndSnapshotWithTypeChangingDecorator {
            inner: self,
            _phantom: PhantomData,
        }
    }
}

impl<Agg, Exec, EAppend, SPersist> EventsAndSnapshot<Agg, Exec, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId, Event=Agg::Event>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>,
{
    pub fn with_decorator<D: EventDecorator<Event=Agg::Event, DecoratedEvent=Agg::Event>>(self) -> EventsAndSnapshotWithDecorator<Agg, Exec, EAppend, SPersist, D> {
        EventsAndSnapshotWithDecorator {
            inner: self,
            _phantom: PhantomData,
        }
    }

    pub fn without_decorator(self) -> EventsAndSnapshotWithDecorator<Agg, Exec, EAppend, SPersist, ::trivial::NopEventDecorator<Agg::Event>> {
        EventsAndSnapshotWithDecorator {
            inner: self,
            _phantom: PhantomData,
        }
    }
}

impl<Agg, Exec, EAppend, SPersist> EventsAndSnapshot<Agg, Exec, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        Agg::Events: IntoIterator<Item=Agg::Event>,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId, Event=Agg::Event>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>,
{
    pub fn execute_and_persist_with_decorator<D: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>>(&self, agg_id: &Exec::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>, decorator: D) -> Result<(), ExecuteAndPersistError<ExecuteError<Agg::CommandError, Exec::Error>, PersistAggregateError<EAppend::Error, SPersist::Error>>> {
        let execute_result =
            self.executor.execute(agg_id, command, precondition)?;

        let decorated_events =
            decorator.decorate_events(execute_result.command_events);

        let hydrated_aggregate = execute_result.hydrated_aggregate;

        let append_precondition =
            precondition.and_then(|p| {
                match p {
                    AggregatePrecondition::ExpectedVersion(AggregateVersion::Version(v)) => Some(Precondition::LastVersion(v)),
                    AggregatePrecondition::ExpectedVersion(AggregateVersion::Initial) => Some(Precondition::EmptyStream),
                    AggregatePrecondition::New => Some(Precondition::NewStream),
                    AggregatePrecondition::Exists => None,
                }
            }).unwrap_or_else(|| hydrated_aggregate.version.into());

        self.appender.append_events(agg_id, &decorated_events, Some(append_precondition))
            .map_err(PersistAggregateError::Events)?;

        let mut new_aggregate = hydrated_aggregate;

        for event in decorated_events.into_iter() {
            new_aggregate.aggregate.apply(event);
            new_aggregate.version += 1;
        }

        if let Some(snapshot) = new_aggregate.to_snapshot() {
            self.persister.persist_snapshot(agg_id, snapshot)
                .map_err(PersistAggregateError::Snapshot)?;
        }

        Ok(())
    }
}

impl<Agg, Exec, EAppend, SPersist> EventsAndSnapshot<Agg, Exec, EAppend, SPersist>
    where
        Agg: SnapshotAggregate,
        Agg::Events: IntoIterator<Item=Agg::Event>,
        Agg::Event: Clone,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>,
{
    pub fn execute_and_persist_with_type_changing_decorator<D: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>>(&self, agg_id: &Exec::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>, decorator: D) -> Result<(), ExecuteAndPersistError<ExecuteError<Agg::CommandError, Exec::Error>, PersistAggregateError<EAppend::Error, SPersist::Error>>> {
        let execute_result =
            self.executor.execute(agg_id, command, precondition)?;

        let mut command_events = Vec::new();
        let mut decorated_events = Vec::new();

        for event in execute_result.command_events.into_iter() {
            command_events.push(event.clone());
            decorated_events.push(decorator.decorate(event));
        }

        let hydrated_aggregate = execute_result.hydrated_aggregate;

        let append_precondition =
            precondition.and_then(|p| {
                match p {
                    AggregatePrecondition::ExpectedVersion(AggregateVersion::Version(v)) => Some(Precondition::LastVersion(v)),
                    AggregatePrecondition::ExpectedVersion(AggregateVersion::Initial) => Some(Precondition::EmptyStream),
                    AggregatePrecondition::New => Some(Precondition::NewStream),
                    AggregatePrecondition::Exists => None,
                }
            }).unwrap_or_else(|| hydrated_aggregate.version.into());

        self.appender.append_events(agg_id, &decorated_events, Some(append_precondition))
            .map_err(PersistAggregateError::Events)?;

        let mut new_aggregate = hydrated_aggregate;

        for event in command_events.into_iter() {
            new_aggregate.aggregate.apply(event);
            new_aggregate.version += 1;
        }

        if let Some(snapshot) = new_aggregate.to_snapshot() {
            self.persister.persist_snapshot(agg_id, snapshot)
                .map_err(PersistAggregateError::Snapshot)?;
        }

        Ok(())
    }
}

pub struct EventsAndSnapshotWithTypeChangingDecorator<Agg, Exec, EAppend, SPersist, Decorator>
    where
        Agg: SnapshotAggregate,
        Agg::Event: Clone,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>,
        Decorator: EventDecorator<Event=Agg::Event, DecoratedEvent=EAppend::Event>,
{
    inner: EventsAndSnapshot<Agg, Exec, EAppend, SPersist>,
    _phantom: PhantomData<Decorator>,
}
pub struct EventsAndSnapshotWithDecorator<Agg, Exec, EAppend, SPersist, Decorator>
    where
        Agg: SnapshotAggregate,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId, Event=Agg::Event>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>,
        Decorator: EventDecorator<Event=Agg::Event, DecoratedEvent=Agg::Event>,
{
    inner: EventsAndSnapshot<Agg, Exec, EAppend, SPersist>,
    _phantom: PhantomData<Decorator>,
}

pub trait AggregateCommand<Agg, Decorator>
    where
        Agg: Aggregate,
        Decorator: EventDecorator<Event=Agg::Event>,
{
    type AggregateId;
    type Error: error::Error;

    fn execute_and_persist_with_decorator(&self, agg_id: &Self::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>, decorator: Decorator) -> Result<(), Self::Error>;
}

impl<Agg, Exec, EAppend, SPersist, Decorator> AggregateCommand<Agg, Decorator> for EventsAndSnapshotWithTypeChangingDecorator<Agg, Exec, EAppend, SPersist, Decorator>
    where
        Agg: SnapshotAggregate,
        Agg::Events: IntoIterator<Item=Agg::Event>,
        Agg::Event: Clone,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId, Event=Agg::Event>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>,
        Decorator: EventDecorator<Event=Agg::Event, DecoratedEvent=Agg::Event>,
{
    type AggregateId = Exec::AggregateId;
    type Error = ExecuteAndPersistError<ExecuteError<Agg::CommandError, Exec::Error>, PersistAggregateError<EAppend::Error, SPersist::Error>>;

    fn execute_and_persist_with_decorator(&self, agg_id: &Self::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>, decorator: Decorator) -> Result<(), Self::Error> {
        self.inner.execute_and_persist_with_decorator(agg_id, command, precondition, decorator)
    }
}

impl<Agg, Exec, EAppend, SPersist, Decorator> AggregateCommand<Agg, Decorator> for EventsAndSnapshotWithDecorator<Agg, Exec, EAppend, SPersist, Decorator>
    where
        Agg: SnapshotAggregate,
        Agg::Events: IntoIterator<Item=Agg::Event>,
        Exec: Executor<Agg>,
        EAppend: EventAppend<AggregateId=Exec::AggregateId, Event=Agg::Event>,
        SPersist: SnapshotPersist<Snapshot=Agg::Snapshot, AggregateId=Exec::AggregateId>,
        Decorator: EventDecorator<Event=Agg::Event, DecoratedEvent=Agg::Event>,
{
    type AggregateId = Exec::AggregateId;
    type Error = ExecuteAndPersistError<ExecuteError<Agg::CommandError, Exec::Error>, PersistAggregateError<EAppend::Error, SPersist::Error>>;

    fn execute_and_persist_with_decorator(&self, agg_id: &Self::AggregateId, command: Agg::Command, precondition: Option<AggregatePrecondition>, decorator: Decorator) -> Result<(), Self::Error> {
        self.inner.execute_and_persist_with_decorator(agg_id, command, precondition, decorator)
    }
}
