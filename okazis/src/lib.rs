extern crate futures;

pub enum ReadOffset<Offset> {
    BeginningOfStream,
    Offset(Offset),
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedEvent<Offset, Event, Metadata>
{
    pub offset: Offset,
    pub payload: Event,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedState<Offset, State> {
    pub offset: Offset,
    pub state: State,
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

pub trait Aggregate {
    type Event;
    type Command;
    type CommandError;
    fn apply(&self, event: Self::Event) -> Self;
    fn execute(&self, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::CommandError>;
}