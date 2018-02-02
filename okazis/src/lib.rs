extern crate futures;

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedEvent<Offset, Event, Metadata> {
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
    fn read(&self, offset: Self::Offset) -> Self::ReadResult;
}

pub trait StateStore {}
