use okazis::{Version, Since, PersistedEvent, AppendError, Precondition, PersistResult};
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
    pub(crate) fn append_events(&self, events: &[Event], condition: Precondition) -> PersistResult<AppendError<Never>> {
        let mut stream = self.events.write().unwrap();

        match condition {
            Precondition::Always => {}
            Precondition::LastVersion(i) if !stream.is_empty() && stream.len() - 1 == i.0 => {}
            Precondition::EmptyStream if stream.is_empty() => {}
            _ => return Err(AppendError::PreconditionFailed(condition))
        }

        stream.extend_from_slice(events);
        Ok(())
    }

    pub(crate) fn read(&self, version: Since) -> Vec<PersistedEvent<Event>> {
        fn read_out_events<Event: Clone>(initial_version: usize, evts: &[Event]) -> Vec<PersistedEvent<Event>> {
            let mut i = initial_version;
            let mut v = Vec::<PersistedEvent<Event>>::new();
            for e in evts {
                v.push(PersistedEvent { version: Version(i), event: e.clone() });
                i += 1;
            }
            v
        }
        let events = self.events.read().unwrap();
        match version {
            Since::BeginningOfStream => read_out_events(0, &events[..]),
            Since::Version(o) if o < Version(events.len()) => read_out_events(o.0 + 1, &events[(o.0 + 1)..]),
            Since::Version(_) => Vec::default(),
        }
    }
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;