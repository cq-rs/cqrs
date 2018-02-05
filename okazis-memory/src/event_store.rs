use okazis::EventStore;
use event_stream::MemoryEventStream;

pub struct MemoryEventStore<Event> {
    event_stream: MemoryEventStream<Event>,
}

impl<Event> Default for MemoryEventStore<Event> {
    fn default() -> Self {
        MemoryEventStore {
            event_stream: MemoryEventStream::new(),
        }
    }
}

impl<Event> EventStore for MemoryEventStore<Event>
    where
        Event: Clone,
{
    type StreamId = usize;
    type EventStream = MemoryEventStream<Event>;
    fn open_stream(&self, stream_id: Self::StreamId) -> Self::EventStream {
        self.event_stream.clone()
    }
}

#[cfg(test)]
#[path = "./event_store_tests.rs"]
mod tests;
