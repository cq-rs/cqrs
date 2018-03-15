extern crate cqrs;
extern crate cqrs_data;
extern crate hyper;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;
#[macro_use]
extern crate lazy_static;
extern crate rayon;
extern crate failure;
#[macro_use]
extern crate failure_derive;

#[cfg(test)]
#[macro_use]
extern crate static_assertions;

pub mod http;

use serde::de::DeserializeOwned;
use serde::Serialize;
use rayon::prelude::*;
use failure::{Fail, ResultExt};

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug)]
pub struct EventIterator<'a, D, M>
    where
        D: DeserializeOwned,
        M: DeserializeOwned,
{
    conn: &'a http::EventStoreConnection,
    next_page: Option<String>,
    buffer: Vec<cqrs::SequencedEvent<EventEnvelope<D, M>>>,
    embed: http::Embedding,
}

impl<'a, D, M> EventIterator<'a, D, M>
    where
        D: DeserializeOwned + Send + Sync,
        M: DeserializeOwned + Send + Sync,
{
    fn process_event_entries(&mut self, page: http::dto::StreamPage) {
        self.buffer = page.entries.into_par_iter()
            .map(|entry| {
                match entry {
                    http::dto::EventEntry::WithEmbeddedEvent(header) => {
                        let event = EventEnvelope {
                            event_id: header.event_id,
                            event_type: header.event_type,
                            data: serde_json::from_str(&header.data).unwrap(),
                            metadata: header.metadata.map(|m| serde_json::from_str(&m).unwrap()),
                        };
                        cqrs::SequencedEvent {
                            sequence_number: cqrs::EventNumber::new(header.event_number),
                            event,
                        }
                    },
                    http::dto::EventEntry::Header(header) => {
                        let event_url =
                            header.links.into_iter()
                                .find(|l| l.relation == http::dto::Relation::Alternate)
                                .map(|l| l.uri)
                                .expect("Event entries should always have an alternate relation");
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
                    }
                }
            })
            .collect();
    }

    fn process_page(&mut self, page: http::dto::StreamPage) {
        if !page.head_of_stream {
            self.next_page =
                page.links.iter()
                    .find(|l| l.relation == http::dto::Relation::Previous)
                    .map(|l| l.uri.to_owned());
        }
        self.process_event_entries(page);
    }
}

const PAGE_SIZE: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventEnvelope<D, M> {
    pub event_id: uuid::Uuid,
    pub event_type: String,
    pub data: D,
    pub metadata: Option<M>,
}

impl<'a, D, M> Iterator for EventIterator<'a, D, M>
    where
        D: DeserializeOwned + Send + Sync,
        M: DeserializeOwned + Send + Sync,
{
    type Item = Result<cqrs::SequencedEvent<EventEnvelope<D, M>>, failure::Compat<http::Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(event) = self.buffer.pop() {
           Some(Ok(event))
        } else if self.next_page.is_some() {
            let mut url = None;
            ::std::mem::swap(&mut self.next_page, &mut url);
            match self.conn.get_stream_page_with_url(&url.unwrap(), self.embed) {
                Ok(Some(page)) => {
                    self.process_page(page);
                    self.next()
                },
                Ok(None) => {
                    self.next_page = None;
                    None
                },
                Err(err) => {
                    self.next_page = None;
                    self.buffer.clear();
                    Some(Err(err.compat()))
                },
            }
        } else {
            None
        }
    }
}

impl<'a, 'id, D, M> cqrs_data::event::Source<'id, EventEnvelope<D, M>> for EventStore<'a, D, M>
    where
        D: DeserializeOwned + Send + Sync,
        M: DeserializeOwned + Send + Sync,
{
    type AggregateId = &'id str;
    type Events = EventIterator<'a, D, M>;
    type Error = failure::Compat<http::Error>;

    fn read_events(&self, agg_id: Self::AggregateId, since: cqrs_data::Since) -> Result<Option<Self::Events>, Self::Error> {
        let initial_event =
            match since {
                cqrs_data::Since::BeginningOfStream => cqrs::EventNumber::new(0),
                cqrs_data::Since::Event(event_num) => event_num.incr(),
            };
        let page = self.conn.get_stream_page(agg_id, initial_event, PAGE_SIZE, http::Embedding::EmbedEvents)
            .map_err(|e| e.compat())?;
        Ok(page.map(|p| {
            let mut iter = EventIterator {
                conn: self.conn,
                next_page: None,
                buffer: Vec::new(),
                embed: http::Embedding::EmbedEvents,
            };
            iter.process_page(p);
            iter
        }))
    }
}

impl<'a, 'id, D, M> cqrs_data::event::Store<'id, EventEnvelope<D, M>> for EventStore<'a, D, M>
    where
        D: Serialize,
        M: Serialize,
{
    type AggregateId = &'id str;
    type Error = failure::Compat<http::Error>;

    fn append_events(&self, agg_id: Self::AggregateId, events: &[EventEnvelope<D, M>], expect: cqrs_data::Expectation) -> Result<cqrs::EventNumber, Self::Error> {
        let events: Vec<_> = events.iter().map(|e| {
                http::dto::AppendEvent {
                    event_id: e.event_id,
                    event_type: &e.event_type,
                    data: &e.data,
                    metadata: e.metadata.as_ref(),
                }
            })
            .collect();
        self.conn.append_events(agg_id, &events, expect)
            .map_err(|e| e.compat())
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
