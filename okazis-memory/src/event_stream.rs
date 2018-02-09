use okazis::{Since, PersistedEvent, AppendError, Precondition};
use std::sync::{RwLock, Arc};
use super::Never;

#[derive(Debug)]
pub(crate) struct MemoryEventStream<Event, Metadata> {
    events: Arc<RwLock<Vec<(Event, Metadata)>>>,
}

impl<Event, Metadata> Clone for MemoryEventStream<Event, Metadata> {
    fn clone(&self) -> Self {
        MemoryEventStream {
            events: Arc::clone(&self.events)
        }
    }
}

impl<Event, Metadata> Default for MemoryEventStream<Event, Metadata> {
    fn default() -> Self {
        MemoryEventStream {
            events: Arc::new(RwLock::default())
        }
    }
}


impl<Event, Metadata> MemoryEventStream<Event, Metadata>
    where
        Event: Clone,
        Metadata: Clone,
{
    pub(crate) fn append_events(&self, events: &[(Event, Metadata)], condition: Precondition<usize>) -> Result<Option<usize>, AppendError<usize, Never>> {
        let mut stream = self.events.write().unwrap();

        match condition {
            Precondition::Always => {}
            Precondition::LastOffset(i) if !stream.is_empty() && stream.len() - 1 == i => {}
            Precondition::EmptyStream if stream.is_empty() => {}
            _ => return Err(AppendError::PreconditionFailed(condition))
        }

        stream.extend_from_slice(events);
        if stream.is_empty() {
            Ok(None)
        } else {
            Ok(Some(stream.len() - 1))
        }
    }

    pub(crate) fn read(&self, offset: Since<usize>) -> Vec<PersistedEvent<usize, Event, Metadata>> {
        fn read_out_events<Event: Clone, Metadata: Clone>(initial_offset: usize, evts: &[(Event, Metadata)]) -> Vec<PersistedEvent<usize, Event, Metadata>> {
            let mut i = initial_offset;
            let mut v = Vec::<PersistedEvent<usize, Event, Metadata>>::new();
            for e in evts {
                v.push(PersistedEvent { offset: i, event: e.0.clone(), metadata: e.1.clone() });
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