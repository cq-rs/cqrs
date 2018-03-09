pub(crate) mod dto;
use hyper;
use hyper::mime;
use serde_json;

use std::fmt::{self, Display};
use std::error;
use std::io::Read;
use std::borrow::Borrow;
use cqrs::{EventNumber, Version, Precondition};
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
    mime::Mime(mime::TopLevel::Application, mime::SubLevel::Ext(ext.to_string()), vec![(mime::Attr::Charset, mime::Value::Utf8)])
}

lazy_static! {
    static ref ES_ATOM_ACCEPT: hyper::header::Accept =
        hyper::header::Accept(vec![hyper::header::qitem(make_mime("vnd.eventstore.atom+json"))]);

    static ref ES_EVENT_ACCEPT: hyper::header::Accept =
        hyper::header::Accept(vec![hyper::header::qitem(make_mime("vnd.eventstore.event+json"))]);

    static ref ES_EVENTS_CONTENT: hyper::header::ContentType =
        hyper::header::ContentType(make_mime("vnd.eventstore.events+json"));
}

#[derive(Clone, Debug, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum ExpectedVersion {
    Any,
    CreateNew,
    Empty,
    LastEvent(EventNumber),
}

impl hyper::header::Header for ExpectedVersion {
    fn header_name() -> &'static str {
        "ES-ExpectedVersion"
    }

    fn parse_header(_raw: &[Vec<u8>]) -> hyper::Result<ExpectedVersion> {
        unimplemented!()
    }
}

impl hyper::header::HeaderFormat for ExpectedVersion {
    fn fmt_header(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExpectedVersion::Empty => f.write_str("0"),
            ExpectedVersion::CreateNew => f.write_str("-1"),
            ExpectedVersion::Any => f.write_str("-2"),
            ExpectedVersion::LastEvent(evt_nbr) => evt_nbr.fmt(f),
        }
    }
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
            U: hyper::client::IntoUrl + fmt::Debug,
    {
        println!("Requesting URL: {:?}", url);
        let result = self.client.get(url)
            .header(self.credentials.clone())
            .header(ES_EVENT_ACCEPT.clone())
            .send()?;

        Ok(serde_json::from_reader(result)?)
    }

    pub fn append_events<D, M>(&self, stream_id: &str, append_events: &[dto::AppendEvent<D, M>], expected_version: Option<Precondition>) -> Result<(), Error>
        where
            D: Serialize,
            M: Serialize,
    {
        let serialized_body = serde_json::to_vec(append_events)?;
        let borrowed_body: &[u8] = serialized_body.borrow();
        let mut req = self.client.post(build_event_append_url(&self.base_url, stream_id))
            .body(borrowed_body)
            .header(self.credentials.clone())
            .header(ES_EVENTS_CONTENT.clone());

        req =
            if let Some(expected_event) = expected_version {
                match expected_event {
                    Precondition::Exists => req,
                    Precondition::New => req.header(ExpectedVersion::CreateNew),
                    Precondition::ExpectedVersion(Version::Initial) => req.header(ExpectedVersion::Empty),
                    Precondition::ExpectedVersion(Version::Number(v)) => req.header(ExpectedVersion::LastEvent(v)),
                }
            } else {
                req.header(ExpectedVersion::Any)
            };

        let mut resp = req.send()?;

        if resp.status != hyper::status::StatusCode::Created {
            println!("Error: {}\n {:#?}", resp.status, resp.headers);
            let mut string = String::new();
            resp.read_to_string(&mut string).unwrap();
            println!("body: {}", string);
            Ok(())
//            Err(Error::Hyper(::hyper::Error::Status))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;