use cqrs::{EventNumber, Precondition, SequencedEvent};
use cqrs::error::{AppendEventsError, Never};
use cqrs_data::Since;
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
    pub(crate) fn append_events(&self, events: &[Event], precondition: Option<Precondition>) -> Result<EventNumber, AppendEventsError<Never>> {
        let mut stream = self.events.write().unwrap();

        let next_event_number = EventNumber::new(stream.len());

        match precondition {
            None => {}
            Some(Precondition::ExpectedVersion(i)) if !stream.is_empty() && next_event_number == EventNumber::from(i).incr() => {}
            Some(Precondition::New) if !stream.is_empty() => {}
            Some(Precondition::Exists) if stream.is_empty() => {}
            Some(precondition) => return Err(AppendEventsError::PreconditionFailed(precondition)),
        }

        stream.extend_from_slice(events);
        Ok(next_event_number)
    }

    pub(crate) fn read(&self, version: Since) -> Vec<SequencedEvent<Event>> {
        let events = self.events.read().unwrap();
        match version {
            Since::BeginningOfStream => {
                let mut sequence = EventNumber::default();
                let mut evts = Vec::new();
                for event in events.iter() {
                    evts.push(SequencedEvent{ sequence, event: event.to_owned() });
                    sequence = sequence.incr();
                }
                evts
            }
            Since::Event(o) => {
                let next_sequence = o.incr();
                if o.number() >= events.len() {
                    Vec::default()
                } else {
                    let mut sequence = next_sequence;
                    let mut evts = Vec::new();
                    for event in events[next_sequence.number()..].iter() {
                        evts.push(SequencedEvent { sequence, event: event.to_owned() });
                        sequence = sequence.incr();
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