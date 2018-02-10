#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Since<Offset> {
    BeginningOfStream,
    Offset(Offset),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Precondition<Offset> {
    Always,
    LastOffset(Offset),
    NewStream,
    EmptyStream,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum AppendError<Offset, Err> {
    PreconditionFailed(Precondition<Offset>),
    WriteError(Err),
}

impl<Offset> Default for Precondition<Offset> {
    fn default() -> Self {
        Precondition::Always
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedEvent<Offset, Event, Metadata>
{
    pub offset: Offset,
    pub event: Event,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedSnapshot<Version, State> {
    pub version: Version,
    pub data: State,
}

pub trait EventStore {
    type AggregateId;
    type Event;
    type Metadata;
    type Offset;
    type AppendResult;
    type ReadResult;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[(Self::Event, Self::Metadata)], condition: Precondition<Self::Offset>) -> Self::AppendResult;
    fn read(&self, agg_id: &Self::AggregateId, since: Since<Self::Offset>) -> Self::ReadResult;
}

pub trait StateStore {
    type AggregateId;
    type State;
    type Version;
    type StateResult;
    type PersistResult;

    fn get_state(&self, agg_id: &Self::AggregateId) -> Self::StateResult;
    fn put_state(&self, agg_id: &Self::AggregateId, version: Self::Version, state: Self::State) -> Self::PersistResult;
}

pub trait EventDecorator {
    type Event;
    type DecoratedEvent;

    fn decorate(&self, event: Self::Event) -> Self::DecoratedEvent;
    fn decorate_events(&self, events: Vec<Self::Event>) -> Vec<Self::DecoratedEvent> {
        events.into_iter()
            .map(|e| self.decorate(e))
            .collect()
    }
}

pub trait Aggregate: Default {
    type Event;
    type Command;
    type CommandError;
    fn apply(&mut self, event: Self::Event);
    fn execute(&self, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::CommandError>;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Never {}

#[derive(Debug, Default, PartialEq, Hash, Clone, Copy)]
pub struct NullEventStore<Event, AggregateId, Offset, Metadata> {
    _phantom: ::std::marker::PhantomData<(Event, AggregateId, Offset, Metadata)>,
}

#[derive(Debug, PartialEq, Hash, Clone, Copy)]
pub struct DefaultMetadataDecorator<Event, Metadata> {
    _phantom: ::std::marker::PhantomData<(Event, Metadata)>,
}

impl<Event, Metadata> Default for DefaultMetadataDecorator<Event, Metadata> {
    fn default() -> Self {
        DefaultMetadataDecorator { _phantom: ::std::marker::PhantomData }
    }
}

impl<Event, Metadata> EventDecorator for DefaultMetadataDecorator<Event, Metadata>
    where
        Metadata: Default,
{
    type Event = Event;
    type DecoratedEvent = (Event, Metadata);

    fn decorate(&self, event: Self::Event) -> Self::DecoratedEvent {
        (event, Default::default())
    }
}

impl<Event, AggregateId, Offset, Metadata> EventStore for NullEventStore<Event, AggregateId, Offset, Metadata>
    where
        Offset: Default,
{
    type AggregateId = AggregateId;
    type Event = Event;
    type Metadata = Metadata;
    type Offset = Offset;
    type AppendResult = Result<(), Never>;
    type ReadResult = Result<Option<Vec<PersistedEvent<Self::Offset, Self::Event, Metadata>>>, Never>;

    fn append_events(&self, _aggregate_id: &Self::AggregateId, _events: &[(Self::Event, Self::Metadata)], _condition: Precondition<Self::Offset>) -> Self::AppendResult {
        Ok(())
    }
    fn read(&self, _aggregate_id: &Self::AggregateId, _offset: Since<Self::Offset>) -> Self::ReadResult {
        Ok(Some(Vec::new()))
    }
}

pub struct NullStateStore<State, AggregateId, Version> {
    _phantom: ::std::marker::PhantomData<(State, AggregateId, Version)>,
}

impl<State, AggregateId, Version> StateStore for NullStateStore<State, AggregateId, Version>
{
    type AggregateId = AggregateId;
    type State = State;
    type Version = Version;
    type StateResult = Result<Option<PersistedSnapshot<Self::Version, Self::State>>, Never>;
    type PersistResult = Result<(), Never>;

    fn get_state(&self, _agg_id: &Self::AggregateId) -> Self::StateResult {
        Ok(None)
    }
    fn put_state(&self, _agg_id: &Self::AggregateId, _version: Self::Version, _state: Self::State) -> Self::PersistResult {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SnapshotDecision {
    Persist,
    Skip,
}

pub trait SnapshotDecider<AggregateId> {
    fn should_snapshot(&self, agg_id: &AggregateId, new_event_count: usize) -> SnapshotDecision;
}

#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
struct AlwaysPersistSnapshot;

impl<AggregateId> SnapshotDecider<AggregateId> for AlwaysPersistSnapshot {
    #[inline]
    fn should_snapshot(&self, _agg_id: &AggregateId, _new_event_count: usize) -> SnapshotDecision {
        SnapshotDecision::Persist
    }
}

#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
struct NeverPersistSnapshot;

impl<AggregateId> SnapshotDecider<AggregateId> for NeverPersistSnapshot {
    #[inline]
    fn should_snapshot(&self, _agg_id: &AggregateId, _new_event_count: usize) -> SnapshotDecision {
        SnapshotDecision::Skip
    }
}

#[derive(Debug, Hash, PartialEq, Clone)]
pub struct AggregateProcessor<Aggregate, EventStore, StateStore>
    where
        EventStore: self::EventStore<Event=Aggregate::Event>,
        StateStore: self::StateStore<State=Aggregate, AggregateId=EventStore::AggregateId, Version=EventStore::Offset>,
        Aggregate: self::Aggregate,
{
    event_store: EventStore,
    state_store: StateStore,
}

#[derive(Debug, Hash, PartialEq, Clone)]
pub enum AggregateError<CmdErr, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr> {
    BadCommand(CmdErr),
    ReadStream(ReadStreamErr),
    ReadState(ReadStateErr),
    WriteStream(WriteStreamErr),
    WriteState(WriteStateErr),
}

impl<Aggregate, AggregateId, EventStore, StateStore, Offset, Event, Metadata, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>
AggregateProcessor<Aggregate, EventStore, StateStore>
    where
        EventStore: self::EventStore<
            AggregateId=AggregateId,
            Offset=Offset,
            Event=Event,
            Metadata=Metadata,
            ReadResult=Result<Option<Vec<PersistedEvent<Offset, Event, Metadata>>>, ReadStreamErr>,
            AppendResult=Result<(), WriteStreamErr>>,
        StateStore: self::StateStore<
            State=Aggregate,
            AggregateId=AggregateId,
            Version=Offset,
            StateResult=Result<Option<PersistedSnapshot<Offset, Aggregate>>, ReadStateErr>,
            PersistResult=Result<(), WriteStateErr>>,
        Aggregate: self::Aggregate<Event=Event>,
        Offset: Clone,
        Metadata: Default,
{
    pub fn execute_and_persist<D, S>(&self, agg_id: &AggregateId, cmd: Aggregate::Command, decorator: D, snapshot_decider: S) -> Result<usize, AggregateError<Aggregate::CommandError, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>>
        where
            D: EventDecorator<Event=Event, DecoratedEvent=(Event, Metadata)>,
            S: SnapshotDecider<AggregateId>,
    {
        let saved_snapshot = self.get_last_snapshot(&agg_id)?;

        // Instantiate aggregate with snapshot or default data
        let (current_version, mut state) =
            if let Some(snapshot) = saved_snapshot {
                (Some(snapshot.version), snapshot.data)
            } else {
                (None, Aggregate::default())
            };

        let (read_offset, precondition) =
            if let Some(ref v) = current_version {
                (Since::Offset(v.clone()), Precondition::LastOffset(v.clone()))
            } else {
                (Since::BeginningOfStream, Precondition::EmptyStream)
            };

        self.rehydrate(&agg_id, &mut state, read_offset.clone())?;

        // Apply command to aggregate
        let events =
            state.execute(cmd)
                .map_err(|e| AggregateError::BadCommand(e))?;

        let event_count = events.len();

        // Skip if no new events
        if event_count > 0 {
            let decorated_events = decorator.decorate_events(events);

            // Append new events to event store if underlying stream
            // has not changed
            self.event_store.append_events(&agg_id, &decorated_events, precondition)
                .map_err(|e| AggregateError::WriteStream(e))?;

            if snapshot_decider.should_snapshot(&agg_id, event_count) == SnapshotDecision::Persist {
                self.refresh_and_persist_snapshot(&agg_id, state, read_offset)?;
            }
        }

        Ok(event_count)
    }

    fn rehydrate(&self, agg_id: &AggregateId, agg: &mut Aggregate, since: Since<Offset>) -> Result<Option<Offset>, AggregateError<Aggregate::CommandError, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>> {
        let read_events =
            self.event_store.read(agg_id, since)
                .map_err(|e| AggregateError::ReadStream(e))?;

        let mut latest_offset = None;
        if let Some(events) = read_events {
            // Re-hydrate aggregate
            for e in events {
                agg.apply(e.event);
                latest_offset = Some(e.offset);
            }
        }

        Ok(latest_offset)
    }

    fn refresh_and_persist_snapshot(&self, agg_id: &AggregateId, agg: Aggregate, last_version: Since<Offset>) -> Result<(), AggregateError<Aggregate::CommandError, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>> {
        let mut agg = agg;
        let last_event_offset = self.rehydrate(&agg_id, &mut agg, last_version)?;

        if let Some(version) = last_event_offset {
            self.state_store.put_state(&agg_id, version, agg)
                .map_err(|e| AggregateError::WriteState(e))?;
        }

        Ok(())
    }

    fn get_last_snapshot(&self, agg_id: &AggregateId) -> Result<Option<PersistedSnapshot<Offset, Aggregate>>, AggregateError<Aggregate::CommandError, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>> {
        self.state_store.get_state(&agg_id)
            .map_err(|e| AggregateError::ReadState(e))
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;

    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    enum MyEvent {
        Wow
    }

    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
    enum MyCommand {
        Much
    }

    #[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
    struct CoolAggregate;

    impl Aggregate for CoolAggregate {
        type Event = MyEvent;
        type Command = MyCommand;
        type CommandError = Never;
        fn apply(&mut self, _evt: Self::Event) {}
        fn execute(&self, _cmd: Self::Command) -> Result<Vec<Self::Event>, Self::CommandError> {
            Ok(vec![MyEvent::Wow])
        }
    }

    #[test]
    fn maybe_this_works_() {
        let es: NullEventStore<MyEvent, usize, usize, ()> =
            NullEventStore { _phantom: ::std::marker::PhantomData };
        let ss: NullStateStore<CoolAggregate, usize, usize> =
            NullStateStore { _phantom: ::std::marker::PhantomData };

        let agg: AggregateProcessor<CoolAggregate, _, _> = AggregateProcessor {
            event_store: es,
            state_store: ss,
        };

        let result =
            agg.execute_and_persist(
                &0usize,
                MyCommand::Much,
                DefaultMetadataDecorator::default(),
                AlwaysPersistSnapshot::default());
        assert_eq!(result, Ok(1usize));
    }
}