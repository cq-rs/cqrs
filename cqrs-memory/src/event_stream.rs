use cqrs::{Version, Since, PersistedEvent, AppendError, Precondition, PersistResult, Never};
use std::sync::{RwLock, Arc};

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
            Precondition::LastVersion(i) if !stream.is_empty() && stream.len() == i + 1 => {}
            Precondition::EmptyStream if stream.is_empty() => {}
            _ => return Err(AppendError::PreconditionFailed(condition))
        }

        stream.extend_from_slice(events);
        Ok(())
    }

    pub(crate) fn read(&self, version: Since) -> Vec<PersistedEvent<Event>> {
        fn read_out_events<Event: Clone>(initial_version: Version, evts: &[Event]) -> Vec<PersistedEvent<Event>> {
            let mut i = initial_version;
            let mut v = Vec::<PersistedEvent<Event>>::new();
            for e in evts {
                v.push(PersistedEvent { version: i, event: e.clone() });
                i += 1;
            }
            v
        }
        let events = self.events.read().unwrap();
        match version {
            Since::BeginningOfStream => read_out_events(Version::default(), &events[..]),
            Since::Version(o) if o < Version::new(events.len()) => read_out_events(o + 1, &events[(*(o + 1).as_ref())..]),
            Since::Version(_) => Vec::default(),
        }
    }
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;