use okazis::EventStream;

#[derive(Clone, Copy, PartialEq, Hash, Debug)]
pub enum ReadError {
    ReadPastEndOfStream,
}

pub struct MemoryEventStream<Event> {
    events: Vec<Event>,
}

impl<Event> MemoryEventStream<Event> {
    pub(crate) fn new() -> Self {
        MemoryEventStream {
            events: Vec::default(),
        }
    }
}

impl<Event> EventStream for MemoryEventStream<Event>
    where
        Event: Clone,
{
    type Event = Event;
    type Offset = usize;
    type ReadResult = Result<Vec<Self::Event>, ReadError>;
    fn append_events(&mut self, events: Vec<Self::Event>) {
        self.events.extend(events);
    }
    fn read(&self, offset: Self::Offset) -> Self::ReadResult {
        if offset > self.events.len() {
            Err(ReadError::ReadPastEndOfStream)
        } else {
            Ok(self.events[offset..].into())
        }
    }
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;