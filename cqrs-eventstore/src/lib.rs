extern crate cqrs;
extern crate cqrs_data;
extern crate hyper;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate uuid;
#[macro_use] extern crate lazy_static;
extern crate rayon;

#[cfg(test)] #[macro_use] extern crate static_assertions;

pub mod http;

use serde::de::DeserializeOwned;
use serde::Serialize;
use rayon::prelude::*;

pub struct EventStore<'a, D, M> {
    conn: &'a http::EventStoreConnection,
    _phantom: ::std::marker::PhantomData<(D, M)>
}

impl<'a, D, M> EventStore<'a, D, M> {
    pub fn new(conn: &'a http::EventStoreConnection) -> Self {
        EventStore {
            conn,
            _phantom: ::std::marker::PhantomData,
        }
    }
}

pub struct EventIterator<'a, D, M>
    where
        D: DeserializeOwned,
        M: DeserializeOwned,
{
    conn: &'a http::EventStoreConnection,
    stream_id: String,
    initial_event: cqrs::EventNumber,
    next_page: Option<String>,
    first_read: bool,
    buffer: Vec<cqrs::SequencedEvent<EventEnvelope<D, M>>>,
}

impl<'a, D, M> EventIterator<'a, D, M>
    where
        D: DeserializeOwned + Send + Sync,
        M: DeserializeOwned + Send + Sync,
{
    fn process_page(&mut self, page: http::dto::StreamPage) {
        if !page.head_of_stream {
            self.next_page = page.links.into_iter().filter_map(|l| {
                if l.relation == http::dto::Relation::Previous {
                    Some(l.uri)
                } else {
                    None
                }
            }).next();
        }
        self.buffer = page.entries.into_par_iter()
            .map(|entry| {
                let event_url = entry.links.into_iter().filter_map(|l| {
                    if l.relation == http::dto::Relation::Alternate {
                        Some(l.uri)
                    } else {
                        None
                    }
                }).next().expect("Event entries should always have an alternate relation");
                let event: http::dto::EventEnvelope<D, M> =
                    self.conn.get_event(&event_url)
                        .expect("Event should always be accessible at URL, otherwise fail");
                cqrs::SequencedEvent {
                    sequence_number: cqrs::EventNumber::new(event.event_number),
                    event: EventEnvelope {
                        event_id: event.event_id,
                        event_type: event.event_type,
                        data: event.data,
                        metadata: event.metadata,
                    }
                }
            })
            .collect();
    }
}

const PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventEnvelope<D, M> {
    pub event_id: uuid::Uuid,
    pub event_type: String,
    pub data: D,
    pub metadata: M,
}

impl<'a, D, M> Iterator for EventIterator<'a, D, M>
    where
        D: DeserializeOwned + Send + Sync,
        M: DeserializeOwned + Send + Sync,
{
    type Item = cqrs::SequencedEvent<EventEnvelope<D, M>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(event) = self.buffer.pop() {
           return Some(event);
        } else if self.next_page.is_some() {
            let mut url = None;
            ::std::mem::swap(&mut self.next_page, &mut url);
            let page = self.conn.get_stream_page_with_url(&url.unwrap()).unwrap();
            self.process_page(page);
            return self.next();
        } else if self.next_page.is_none() && self.first_read {
            let page = self.conn.get_stream_page(&self.stream_id, self.initial_event, PAGE_SIZE).unwrap();
            self.process_page(page);
            self.first_read = false;
            return self.next();
        } else {
            return None;
        }
    }
}

impl<'a, D, M> cqrs_data::events::Source for EventStore<'a, D, M>
    where
        D: DeserializeOwned + Send + Sync,
        M: DeserializeOwned + Send + Sync,
{
    type AggregateId = str;
    type Result = EventIterator<'a, D, M>;

    fn read_events(&self, agg_id: &Self::AggregateId, since: cqrs_data::Since) -> Self::Result {
        EventIterator {
            conn: self.conn,
            stream_id: agg_id.to_string(),
            initial_event:
                match since {
                    cqrs_data::Since::BeginningOfStream => cqrs::EventNumber::new(0),
                    cqrs_data::Since::Event(event_num) => event_num.incr(),
                },
            next_page: None,
            first_read: true,
            buffer: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
