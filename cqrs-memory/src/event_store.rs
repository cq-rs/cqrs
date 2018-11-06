use cqrs::{EventNumber, Precondition, SequencedEvent};
use cqrs::error::{AppendEventsError};
use cqrs_data::event;
use cqrs_data::Since;
use event_stream::MemoryEventStream;
use std::sync::RwLock;
use std::hash::{Hash, BuildHasher};
use std::collections::HashMap;
use std::collections::hash_map::RandomState;

#[derive(Debug)]
pub struct MemoryEventStore<Event, AggId, Hasher = RandomState>
    where
        AggId: Hash + Eq,
        Hasher: BuildHasher,
{
    data: RwLock<HashMap<AggId, MemoryEventStream<Event>, Hasher>>,
}

impl<Event, AggId, Hasher> MemoryEventStore<Event, AggId, Hasher>
    where
        AggId: Hash + Eq + Clone,
        Hasher: BuildHasher,
{
    fn try_get_stream(&self, agg_id: &AggId) -> Option<MemoryEventStream<Event>> {
        self.data.read().unwrap()
            .get(agg_id)
            .map(|es| es.clone())
    }

    fn create_stream(&self, agg_id: &AggId) -> MemoryEventStream<Event> {
        let mut lock = self.data.write().unwrap();
        match lock.get(&agg_id) {
            Some(es) => return es.clone(),
            None => {}
        }

        let new_stream = MemoryEventStream::default();
        lock.insert(agg_id.clone(), new_stream.clone());

        new_stream
    }
}

impl<Event, AggId, Hasher> Default for MemoryEventStore<Event, AggId, Hasher>
    where
        AggId: Hash + Eq,
        Hasher: BuildHasher + Default,
{
    fn default() -> Self {
        MemoryEventStore {
            data: RwLock::new(HashMap::<_, _, Hasher>::default()),
        }
    }
}

impl<Event, AggId, Hasher> event::Source<Event> for MemoryEventStore<Event, AggId, Hasher>
    where
        AggId: Hash + Eq + Clone + 'static,
        Event: Clone,
        Hasher: BuildHasher,
{
    type AggregateId = AggId;
    type Events = Vec<Result<SequencedEvent<Event>, Never>>;
    type Error = Never;

    fn read_events(&self, agg_id: Self::AggregateId, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        Ok(match self.try_get_stream(&agg_id) {
            Some(es) => Some(es.read(since).iter().map(|e| Ok(e.to_owned())).collect()),
            None => None,
        })
    }
}

impl<Event, AggId, Hasher> event::Store<Event> for MemoryEventStore<Event, AggId, Hasher>
    where
        AggId: Hash + Eq + Clone + 'static,
        Event: Clone,
        Hasher: BuildHasher,
{
    type AggregateId = AggId;
    type Error = AppendEventsError<Never>;

    fn append_events(&self, agg_id: Self::AggregateId, events: &[Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        if let Some(stream) = self.try_get_stream(&agg_id) {
            stream.append_events(events, precondition)
        } else {
            if let Some(precondition) = precondition {
                match precondition {
                    Precondition::New | Precondition::ExpectedVersion(_) => return Err(AppendEventsError::PreconditionFailed(precondition)),
                    _ => {}
                }
            }

            let stream = self.create_stream(&agg_id);
            stream.append_events(events, None)
        }
    }
}

#[cfg(test)]
#[path = "./event_store_tests.rs"]
mod tests;
