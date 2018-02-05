use okazis::{ReadOffset, EventStream};
use okazis::ReadOffset::*;
use std::sync::{RwLock, Arc};

#[derive(Clone, Copy, PartialEq, Hash, Debug)]
pub enum ReadError {
    ReadPastEndOfStream,
}

#[derive(Debug, Clone)]
pub struct MemoryEventStream<Event> {
    events: Arc<RwLock<Vec<Event>>>,
}

impl<Event> MemoryEventStream<Event> {
    pub(crate) fn new() -> Self {
        MemoryEventStream {
            events: Arc::new(RwLock::default()),
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
    fn append_events(&self, events: Vec<Self::Event>) {
        self.events.write().unwrap().extend(events);
    }
    fn read(&self, offset: ReadOffset<Self::Offset>) -> Self::ReadResult {
        let events = self.events.read().unwrap();
        match offset {
            BeginningOfStream => Ok(events[..].into()),
            Offset(o) if o < events.len() => Ok(events[(o + 1)..].into()),
            Offset(_) => Err(ReadError::ReadPastEndOfStream),
        }
    }
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;