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
    fn read(&self, agg_id: &Self::AggregateId, since_offset: Since<Self::Offset>) -> Self::ReadResult;
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

pub trait MetadataProvider {
    type Event;
    type Metadata;

    fn associate_metadata(&self, event: Self::Event) -> (Self::Event, Self::Metadata);
}

pub trait Aggregate: Default {
    type Event;
    type Command;
    type CommandError;
    fn apply(&mut self, event: &Self::Event);
    fn execute(&self, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::CommandError>;
}

pub enum Never {}

#[derive(Debug, Default, PartialEq, Hash, Clone, Copy)]
pub struct NullEventStore<Event, AggregateId, Offset, Metadata> {
    _phantom: ::std::marker::PhantomData<(Event, AggregateId, Offset, Metadata)>,
}

#[derive(Debug, PartialEq, Hash, Clone, Copy)]
pub struct DefaultMetadataProvider<Event, Metadata> {
    _phantom: ::std::marker::PhantomData<(Event, Metadata)>,
}

impl<Event, Metadata> Default for DefaultMetadataProvider<Event, Metadata> {
    fn default() -> Self {
        DefaultMetadataProvider { _phantom: ::std::marker::PhantomData }
    }
}

impl<Event, Metadata> MetadataProvider for DefaultMetadataProvider<Event, Metadata>
    where
        Metadata: Default,
{
    type Event = Event;
    type Metadata = Metadata;

    fn associate_metadata(&self, event: Self::Event) -> (Self::Event, Self::Metadata) {
        (event, Self::Metadata::default())
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
    type AppendResult = Result<Offset, Never>;
    type ReadResult = Result<Vec<PersistedEvent<Self::Offset, Self::Event, Metadata>>, Never>;

    fn append_events(&self, aggregate_id: &Self::AggregateId, events: &[(Self::Event, Self::Metadata)], condition: Precondition<Self::Offset>) -> Self::AppendResult {
        Ok(Offset::default())
    }
    fn read(&self, aggregate_id: &Self::AggregateId, offset: Since<Self::Offset>) -> Self::ReadResult {
        Ok(Vec::new())
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

    fn get_state(&self, agg_id: &Self::AggregateId) -> Self::StateResult {
        Ok(None)
    }
    fn put_state(&self, agg_id: &Self::AggregateId, version: Self::Version, state: Self::State) -> Self::PersistResult {
        Ok(())
    }
}

pub struct AggregateProcessor<Aggregate, EventStore, StateStore>
    where
        EventStore: self::EventStore<Event=Aggregate::Event>,
        StateStore: self::StateStore<State=Aggregate, AggregateId=EventStore::AggregateId, Version=EventStore::Offset>,
        Aggregate: self::Aggregate,
{
    event_store: EventStore,
    state_store: StateStore,
}

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
        EventStore: self::EventStore<AggregateId=AggregateId, Offset=Offset, Event=Event, Metadata=Metadata, ReadResult=Result<Option<Vec<PersistedEvent<Offset, Event, Metadata>>>, ReadStreamErr>, AppendResult=Result<Option<Offset>, WriteStreamErr>>,
        StateStore: self::StateStore<State=Aggregate, AggregateId=AggregateId, Version=Offset, StateResult=Result<Option<PersistedSnapshot<Offset, Aggregate>>, ReadStateErr>, PersistResult=Result<(), WriteStateErr>>,
        Aggregate: self::Aggregate<Event=Event>,
        Offset: Clone,
        Metadata: Default,
{
    pub fn execute_persist_and_snapshot(&self, agg_id: &AggregateId, cmd: Aggregate::Command) -> Result<usize, AggregateError<Aggregate::CommandError, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>>
    {
        // Retrieve Snapshot
        let saved_snapshot: Option<PersistedSnapshot<Offset, Aggregate>> =
            self.state_store.get_state(&agg_id)
                .map_err(|e| AggregateError::ReadState(e))?;

        // Instantiate aggregate with snapshot or default data
        let (mut current_version, mut state) =
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

        // Read events since the last snapshot was taken
        let prior_events: Vec<PersistedEvent<Offset, Event, Metadata>> =
            self.event_store.read(&agg_id, read_offset)
                .map_err(|e| AggregateError::ReadStream(e))?
                .unwrap_or(Vec::default());

        // Re-hydrate aggregate with existing events
        for e in prior_events {
            state.apply(&e.event);
            current_version = Some(e.offset);
        }

        // Apply command to aggregate
        let events =
            state.execute(cmd)
                .map_err(|e| AggregateError::BadCommand(e))?;

        let event_count = events.len();

        // Skip if no new events
        if event_count > 0 {
            // Apply the newly created events to the state
            for event in &events {
                state.apply(event);
            }

            // Apply metadata
            let events_with_metadata: Vec<_> =
                events.into_iter()
                    .map(|e| DefaultMetadataProvider::default().associate_metadata(e))
                    .collect();

            // Append new events to event store if underlying stream
            // has not changed and get new latest offset
            let new_offset: Option<Offset> =
                self.event_store.append_events(&agg_id, &events_with_metadata, precondition)
                    .map_err(|e| AggregateError::WriteStream(e))?;

            // Persist state snapshot
            if let Some(offset) = new_offset {
                self.state_store.put_state(&agg_id, offset, state);
            }
        }

        Ok(event_count)
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
        fn apply(&mut self, evt: &Self::Event) {}
        fn execute(&self, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::CommandError> {
            Ok(vec![MyEvent::Wow])
        }
    }

    #[test]
    fn maybe_this_works_() {
        let es : NullEventStore<_, usize, usize, ()> = NullEventStore { _phantom: ::std::marker::PhantomData };
        let ss : NullStateStore<_, usize, usize> = NullStateStore { _phantom: ::std::marker::PhantomData };
        let agg: AggregateProcessor<CoolAggregate, _, _> = AggregateProcessor {
            event_store: es,
            state_store: ss,
        };
        let result =
            agg.execute_persist_and_snapshot(&0usize, MyCommand::Much);
        assert!(result.is_ok());
    }
}