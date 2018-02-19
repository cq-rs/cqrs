use cqrs::{Version, Since, VersionedEvent, Precondition};
use cqrs::error::{AppendEventsError, Never};
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
    pub(crate) fn append_events(&self, events: &[Event], precondition: Option<Precondition>) -> Result<(), AppendEventsError<Never>> {
        let mut stream = self.events.write().unwrap();

        match precondition {
            None => {}
            Some(Precondition::LastVersion(i)) if !stream.is_empty() && stream.len() == i + 1 => {}
            Some(Precondition::EmptyStream) if stream.is_empty() => {}
            Some(precondition) => return Err(AppendEventsError::PreconditionFailed(precondition)),
        }

        stream.extend_from_slice(events);
        Ok(())
    }

    pub(crate) fn read(&self, version: Since) -> Vec<VersionedEvent<Event>> {
        let events = self.events.read().unwrap();
        match version {
            Since::BeginningOfStream => {
                let mut version = Version::default();
                let mut evts = Vec::new();
                for event in events.iter() {
                    evts.push(VersionedEvent{ version, event: event.to_owned() });
                    version += 1;
                }
                evts
            }
            Since::Version(o) => {
                let next_version = o + 1;
                if o.number() >= events.len() {
                    Vec::default()
                } else {
                    let mut version = next_version;
                    let mut evts = Vec::new();
                    for event in events[next_version.number()..].iter() {
                        evts.push(VersionedEvent { version, event: event.to_owned() });
                        version += 1;
                    }
                    evts
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "event_stream_tests.rs"]
mod tests;