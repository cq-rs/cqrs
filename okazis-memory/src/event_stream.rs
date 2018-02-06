use okazis::{ReadOffset, PersistedEvent, EventStream};
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
    type ReadResult = Result<Vec<PersistedEvent<Self::Offset, Self::Event, ()>>, ReadError>;
    fn append_events(&self, events: Vec<Self::Event>) {
        self.events.write().unwrap().extend(events);
    }
    fn read(&self, offset: ReadOffset<Self::Offset>) -> Self::ReadResult {
        fn read_out_events<Event: Clone>(initial_offset: usize, evts: &[Event]) -> Vec<PersistedEvent<usize, Event, ()>> {
            let mut i = initial_offset;
            let mut v = Vec::<PersistedEvent<usize, Event, ()>>::new();
            for e in evts {
                v.push(PersistedEvent { offset: i, payload: e.clone(), metadata: () });
                i += 1;
            }
            v
        }
        let events = self.events.read().unwrap();
        match offset {
            BeginningOfStream => Ok(read_out_events(0, &events[..])),
            Offset(o) if o < events.len() => Ok(read_out_events(o + 1, &events[(o + 1)..])),
            Offset(_) => Err(ReadError::ReadPastEndOfStream),
        }
    }
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;