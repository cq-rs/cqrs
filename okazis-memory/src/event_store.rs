use okazis::EventStore;
use event_stream::MemoryEventStream;

pub struct MemoryEventStore<Event> {
    _phantom: ::std::marker::PhantomData<Event>,
}

impl<Event> Default for MemoryEventStore<Event> {
    fn default() -> Self {
        MemoryEventStore {
            _phantom: ::std::marker::PhantomData,
        }
    }
}

impl<Event> EventStore for MemoryEventStore<Event> {
    type StreamId = usize;
    type EventStream = MemoryEventStream<Event>;
    fn open_stream(&self, stream_id: Self::StreamId) -> Self::EventStream {
        MemoryEventStream::new()
    }
}

#[cfg(test)]
#[path = "./event_store_tests.rs"]
mod tests;
