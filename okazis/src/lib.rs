extern crate futures;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadOffset<Offset> {
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

impl<Offset> Default for Precondition<Offset> {
    fn default() -> Self {
        Precondition::Always
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedEvent<Offset, Event, Metadata>
{
    pub offset: Offset,
    pub data: Event,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedSnapshot<Offset, State> {
    pub offset: Offset,
    pub data: State,
}

pub trait EventStore {
    type StreamId;
    type EventStream: EventStream;
    fn open_stream(&self, stream_id: Self::StreamId) -> Self::EventStream;
}

pub trait EventStream {
    type Event;
    type Offset;
    type ReadResult;
    fn append_events(&self, events: Vec<Self::Event>);
    fn read(&self, offset: ReadOffset<Self::Offset>) -> Self::ReadResult;
}

pub trait StateStore {
    type StateId;
    type State;
    type Offset;
    type StateResult;
    fn get_state(&self, stream_id: Self::StateId) -> Self::StateResult;
    fn put_state(&self, stream_id: Self::StateId, offset: Self::Offset, state: Self::State);
}

pub trait Aggregate: Default {
    type Event;
    type Command;
    type CommandError;
    fn apply(&mut self, event: Self::Event);
    fn execute(&self, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::CommandError>;
}

#[derive(Debug, Default, PartialEq, Hash, Clone, Copy)]
pub struct NullEventStore<Event, StreamId, Offset, Metadata> {
    _phantom: ::std::marker::PhantomData<(Event, StreamId, Offset, Metadata)>,
}

#[derive(Debug, Default, PartialEq, Hash, Clone, Copy)]
pub struct NullEventStream<Event, Offset, Metadata> {
    _phantom: ::std::marker::PhantomData<(Event, Offset, Metadata)>,
}

impl<Event, StreamId, Offset, Metadata> EventStore for NullEventStore<Event, StreamId, Offset, Metadata>
{
    type StreamId = StreamId;
    type EventStream = NullEventStream<Event, Offset, Metadata>;
    fn open_stream(&self, stream_id: Self::StreamId) -> Self::EventStream {
        NullEventStream { _phantom: ::std::marker::PhantomData }
    }
}

pub enum Never { }

impl<Event, Offset, Metadata> EventStream for NullEventStream<Event, Offset, Metadata>
{
    type Event = Event;
    type Offset = Offset;
    type ReadResult = Result<Vec<PersistedEvent<Self::Offset, Self::Event, Metadata>>, Never>;
    fn append_events(&self, events: Vec<Self::Event>) {}
    fn read(&self, offset: ReadOffset<Self::Offset>) -> Self::ReadResult {
        Ok(Vec::new())
    }
}

pub struct NullStateStore<State, StateId, Offset> {
    _phantom: ::std::marker::PhantomData<(State, StateId, Offset)>,
}

impl<State, StateId, Offset> StateStore for NullStateStore<State, StateId, Offset>
    where
        State: Default,
{
    type StateId = StateId;
    type State = State;
    type Offset = Offset;
    type StateResult = Result<Option<PersistedSnapshot<Self::Offset, Self::State>>, Never>;
    fn get_state(&self, state_id: Self::StateId) -> Self::StateResult {
        Ok(None)
    }
    fn put_state(&self, state_id: Self::StateId, offset: Self::Offset, state: Self::State) {}
}

pub struct AggregateProcessor<Agg, EStore, EStream, Store>
    where
        EStore: EventStore<EventStream = EStream>,
        EStream: EventStream<Event = Agg::Event>,
        Store: StateStore<State = Agg, StateId = EStore::StreamId, Offset = EStream::Offset>,
        Agg: Aggregate,
{
    event_store: EStore,
    state_store: Store,
}

pub enum AggregateError<CmdErr,ReadStreamErr,ReadStateErr> {
    BadCommand(CmdErr),
    ReadStream(ReadStreamErr),
    ReadState(ReadStateErr),
}

impl<Agg, AggId, EStore, EStream, Store, Event, Offset, ReadStreamErr, ReadStateErr> AggregateProcessor<Agg, EStore, EStream, Store>
    where
        EStore: EventStore<EventStream = EStream, StreamId = AggId>,
        EStream: EventStream<Offset = Offset, Event = Event, ReadResult = Result<Vec<PersistedEvent<Offset, Event, ()>>, ReadStreamErr>>,
        Store: StateStore<State=Agg, StateId=AggId, Offset=Offset, StateResult=Result<Option<PersistedSnapshot<Offset, Agg>>, ReadStateErr>>,
        Agg: Aggregate<Event = Event>,
        Offset: Clone,
        Event: Clone,
        AggId: Clone,
{
    pub fn execute_persist_and_snapshot(&self, agg_id: EStore::StreamId, cmd: Agg::Command) -> Result<(),AggregateError<Agg::CommandError, ReadStreamErr, ReadStateErr>>
    {
        let saved_snapshot: Option<PersistedSnapshot<EStream::Offset, Agg>> = self.state_store.get_state(agg_id.clone()).map_err(|e| AggregateError::ReadState(e))?;
        let stream = self.event_store.open_stream(agg_id.clone());
        let (last_offset, state) = {
            let (read_offset, mut snapshot) =
                if let Some(ps) = saved_snapshot {
                    (ReadOffset::Offset(ps.offset), ps.data)
                } else {
                    (ReadOffset::BeginningOfStream, Agg::default())
                };
            let mut last_offset =
                match read_offset {
                    ReadOffset::Offset(ref offset) => Some(offset.clone()),
                    _ => None,
                };
            let prior_events: Vec<PersistedEvent<EStream::Offset, Agg::Event, ()>> =
                stream.read(read_offset).map_err(|e| AggregateError::ReadStream(e))?;
            for e in prior_events {
                snapshot.apply(e.data);
                last_offset = Some(e.offset);
            }
            (last_offset, snapshot)
        };

        let events = state.execute(cmd).map_err(|e| AggregateError::BadCommand(e))?;

        if !events.is_empty() {
            let e_copy = events.clone();
            /*let persisted_events = */stream.append_events(/* Precondition::LastOffset(last_offset), */ events);
            let mut state = state;
            let mut last_offset = last_offset;
            let persisted_events: Vec<PersistedEvent<Offset, Event, ()>> = Vec::new();
            for e in persisted_events {
                state.apply(e.data);
                last_offset = /* Precondition... */ Some(e.offset);
            }
            if let Some(offset) = last_offset {
                self.state_store.put_state(agg_id, offset, state);
            }
        }

        Ok(())
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
        fn apply(&mut self, evt: Self::Event) {}
        fn execute(&self, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::CommandError> {
            Ok(vec![MyEvent::Wow])
        }
    }

    #[test]
    fn maybe_this_works_() {
        let es : NullEventStore<_, usize, usize, ()> = NullEventStore { _phantom: ::std::marker::PhantomData };
        let ss : NullStateStore<_, usize, usize> = NullStateStore { _phantom: ::std::marker::PhantomData };
        let agg : AggregateProcessor<CoolAggregate, _, _, _> = AggregateProcessor {
            event_store: es,
            state_store: ss,
        };
        let result = agg.execute_persist_and_snapshot(0usize, MyCommand::Much);
        assert!(result.is_ok());
    }
}