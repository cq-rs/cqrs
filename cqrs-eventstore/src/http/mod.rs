mod dto;
use hyper;
use hyper::mime;
use serde_json;

use std::fmt;
use std::error;
use cqrs::EventNumber;
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


impl EventStoreConnection {
    pub fn new(base_url: hyper::Url, username: String, password: String) -> Self {
        EventStoreConnection {
            client: hyper::Client::new(),
            credentials:
                hyper::header::Authorization(hyper::header::Basic {
                    username,
                    password: Some(password),
                }),
            base_url,
        }
    }

    pub fn get_stream_page(&self, stream_id: &str, offset: EventNumber, limit: usize) -> Result<dto::StreamPage, Error> {
        let result = self.client.get(build_stream_page_url(&self.base_url, stream_id, offset, limit))
            .header(self.credentials.clone())
            .header(hyper::header::Accept(vec![hyper::header::qitem(
                mime::Mime(mime::TopLevel::Application, mime::SubLevel::Ext("vnd.eventstore.atom+json".to_string()), vec![]))]))
            .send()?;

        Ok(serde_json::from_reader(result)?)
    }

    pub fn get_event<T: ::serde::de::DeserializeOwned>(&self, stream_id: &str, event_num: EventNumber) -> Result<dto::EventEnvelope<T>, Error>{
        let result = self.client.get(build_event_url(&self.base_url, stream_id, event_num))
            .header(self.credentials.clone())
            .header(hyper::header::Accept(vec![hyper::header::qitem(
                mime::Mime(mime::TopLevel::Application, mime::SubLevel::Ext("vnd.eventstore.event+json".to_string()), vec![]))]))
            .send()?;

        Ok(serde_json::from_reader(result)?)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;