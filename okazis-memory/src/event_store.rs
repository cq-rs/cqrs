use okazis::EventStore;
use event_stream::MemoryEventStream;
use fnv::FnvHashMap;
use std::sync::RwLock;

pub struct MemoryEventStore<Event> {
    data: RwLock<FnvHashMap<usize, MemoryEventStream<Event>>>,
}

impl<Event> Default for MemoryEventStore<Event> {
    fn default() -> Self {
        MemoryEventStore {
            data: RwLock::default(),
        }
    }
}

impl<Event> EventStore for MemoryEventStore<Event>
    where
        Event: Clone,
{
    type StreamId = usize;
    type EventStream = MemoryEventStream<Event>;
    fn open_stream(&self, stream_id: Self::StreamId) -> Self::EventStream {
        {
            let lock = self.data.read().unwrap();
            match lock.get(&stream_id) {
                Some(es) => return es.clone(),
                None => {}
            }
        }
        let mut w_lock = self.data.write().unwrap();
        {
            match w_lock.get(&stream_id) {
                Some(es) => return es.clone(),
                None => {}
            }
        }
        let new_es = MemoryEventStream::new();
        let ret_es = new_es.clone();
        w_lock.insert(stream_id, new_es);
        ret_es
    }
}

#[cfg(test)]
#[path = "./event_store_tests.rs"]
mod tests;
