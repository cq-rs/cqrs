use okazis::{Since, PersistedEvent, AppendError, Precondition, PersistResult};
use std::sync::{RwLock, Arc};
use super::Never;

#[derive(Debug)]
pub(crate) struct MemoryEventStream<Event> {
    events: Arc<RwLock<Vec<Event>>>,
}

impl<Event> Clone for MemoryEventStream<Event> {
    fn clone(&self) -> Self {
        MemoryEventStream {
            events: Arc::clone(&self.events)
        }
    }
}

impl<Event> Default for MemoryEventStream<Event> {
    fn default() -> Self {
        MemoryEventStream {
            events: Arc::new(RwLock::default())
        }
    }
}


impl<Event> MemoryEventStream<Event>
    where
        Event: Clone,
{
    pub(crate) fn append_events(&self, events: &[Event], condition: Precondition<usize>) -> PersistResult<AppendError<usize, Never>> {
        let mut stream = self.events.write().unwrap();

        match condition {
            Precondition::Always => {}
            Precondition::LastOffset(i) if !stream.is_empty() && stream.len() - 1 == i => {}
            Precondition::EmptyStream if stream.is_empty() => {}
            _ => return Err(AppendError::PreconditionFailed(condition))
        }

        stream.extend_from_slice(events);
        Ok(())
    }

    pub(crate) fn read(&self, offset: Since<usize>) -> Vec<PersistedEvent<usize, Event>> {
        fn read_out_events<Event: Clone>(initial_offset: usize, evts: &[Event]) -> Vec<PersistedEvent<usize, Event>> {
            let mut i = initial_offset;
            let mut v = Vec::<PersistedEvent<usize, Event>>::new();
            for e in evts {
                v.push(PersistedEvent { offset: i, event: e.clone() });
                i += 1;
            }
            v
        }
        let events = self.events.read().unwrap();
        match offset {
            Since::BeginningOfStream => read_out_events(0, &events[..]),
            Since::Offset(o) if o < events.len() => read_out_events(o + 1, &events[(o + 1)..]),
            Since::Offset(_) => Vec::default(),
        }
    }
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;