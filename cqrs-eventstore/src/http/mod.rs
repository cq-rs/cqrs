pub(crate) mod dto;
use hyper;
use hyper::mime;
use serde_json;

use std::fmt;
use std::error;
use std::io;
use cqrs::EventNumber;
use serde::de::DeserializeOwned;
use serde::Serialize;

// getStreamPage
// getEvent
// appendEvents

#[derive(Debug)]
pub enum Error {
    Hyper(hyper::Error),
    Json(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = self as &error::Error;
        f.write_str(err.description())?;
        f.write_str(": ")?;
        fmt::Display::fmt(err.cause().unwrap(), f)
    }
}

impl error::Error for Error {
    fn description(&self) -> &'static str {
        match *self {
            Error::Hyper(_) => "hyper client error",
            Error::Json(_) => "json serialization/deserialization error",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Hyper(ref err) => Some(err),
            Error::Json(ref err) => Some(err),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::Hyper(err)
    }
}

pub struct EventStoreConnection {
    client: hyper::Client,
    credentials: hyper::header::Authorization<hyper::header::Basic>,
    base_url: hyper::Url,
}

fn build_stream_page_url(base_url: &hyper::Url, stream_id: &str, offset: EventNumber, limit: usize) -> hyper::Url {
    let path = format!("streams/{}/{}/forward/{}", stream_id, offset, limit);
    base_url.join(&path).unwrap()
}

fn build_event_url(base_url: &hyper::Url, stream_id: &str, event_num: EventNumber) -> hyper::Url {
    let path = format!("streams/{}/{}", stream_id, event_num);
    base_url.join(&path).unwrap()
}

fn build_event_append_url(base_url: &hyper::Url, stream_id: &str) -> hyper::Url {
    let path = format!("streams/{}", stream_id);
    base_url.join(&path).unwrap()
}

fn make_mime(ext: &str) -> mime::Mime {
    mime::Mime(mime::TopLevel::Application, mime::SubLevel::Ext(ext.to_string()), vec![])
}

lazy_static! {
    static ref ES_ATOM_ACCEPT: hyper::header::Accept =
        hyper::header::Accept(vec![hyper::header::qitem(make_mime("vnd.eventstore.atom+json"))]);

    static ref ES_EVENT_ACCEPT: hyper::header::Accept =
        hyper::header::Accept(vec![hyper::header::qitem(make_mime("vnd.eventstore.event+json"))]);

    static ref ES_EVENTS_CONTENT: hyper::header::ContentType =
        hyper::header::ContentType(make_mime("vnd.eventstore.events+json"));
}

impl EventStoreConnection {
    pub fn new(client: hyper::Client, base_url: hyper::Url, username: String, password: String) -> Self {
        EventStoreConnection {
            client,
            credentials:
                hyper::header::Authorization(hyper::header::Basic {
                    username,
                    password: Some(password),
                }),
            base_url,
        }
    }

    pub fn get_stream_page(&self, stream_id: &str, offset: EventNumber, limit: usize) -> Result<dto::StreamPage, Error> {
        let url = build_stream_page_url(&self.base_url, stream_id, offset, limit);
        self.get_stream_page_with_url(url)
    }

    pub fn get_stream_page_with_url<U: hyper::client::IntoUrl>(&self, url: U) -> Result<dto::StreamPage, Error> {
        let result = self.client.get(url)
            .header(self.credentials.clone())
            .header(ES_ATOM_ACCEPT.clone())
            .send()?;

        Ok(serde_json::from_reader(result)?)
    }

    pub fn get_event<D, M, U>(&self, url: U) -> Result<dto::EventEnvelope<D, M>, Error>
        where
            D: DeserializeOwned,
            M: DeserializeOwned,
            U: hyper::client::IntoUrl,
    {
        let result = self.client.get(url)
            .header(self.credentials.clone())
            .header(ES_EVENT_ACCEPT.clone())
            .send()?;

        Ok(serde_json::from_reader(result)?)
    }

    pub fn append_events<D, M>(&self, stream_id: &str, append_events: &[dto::AppendEvent<D, M>]) -> Result<(), Error>
        where
            D: Serialize,
            M: Serialize,
    {
        let serialized_body = serde_json::to_vec(append_events)?;
        let mut cursor = io::Cursor::new(serialized_body);
        let _result = self.client.post(build_event_append_url(&self.base_url, stream_id))
            .body(&mut cursor)
            .header(self.credentials.clone())
            .header(ES_EVENTS_CONTENT.clone())
            .send()?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;