use okazis::EventStream;
use std::sync::{RwLock, PoisonError};

#[derive(Clone, Copy, PartialEq, Hash, Debug)]
pub enum ReadError {
    PoisonedStream,
    ReadPastEndOfStream,
}

impl<T> From<PoisonError<T>> for ReadError {
    fn from(_err: PoisonError<T>) -> Self {
        ReadError::PoisonedStream
    }
}

pub struct MemoryEventStream<Event> {
    events: RwLock<Vec<Event>>,
}

impl<Event> MemoryEventStream<Event> {
    pub(crate) fn new() -> Self {
        MemoryEventStream {
            events: RwLock::default(),
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
        let mut lock = self.events.write().unwrap();
        lock.extend(events);
    }
    fn read(&self, offset: Self::Offset) -> Self::ReadResult {
        let lock = self.events.read()?;
        if offset > lock.len() {
            Err(ReadError::ReadPastEndOfStream)
        } else {
            Ok(lock[offset..].into())
        }
    }
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;