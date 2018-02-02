use okazis::EventStream;

pub struct MemoryEventStream<Event> {
    _phantom: ::std::marker::PhantomData<Event>,
}

impl<Event> MemoryEventStream<Event> {
    pub(crate) fn new() -> Self {
        MemoryEventStream {
            _phantom: ::std::marker::PhantomData,
        }
    }
}

impl<Event> EventStream for MemoryEventStream<Event> {
    type Event = Event;
    type Offset = usize;
    type ReadResult = Vec<Self::Event>;
    fn append_events(&self, events: Vec<Self::Event>) {
    }
    fn read(&self, offset: Self::Offset) -> Self::ReadResult {
        Vec::default()
    }
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;