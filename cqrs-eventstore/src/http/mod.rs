pub(crate) mod dto;
use hyper;
use hyper::header;
use hyper::status::StatusCode;
use hyper::mime;
use serde_json;

use failure::{Backtrace, Context, Fail, ResultExt};

use std::fmt::{self, Display};
use std::borrow::Borrow;
use std::error;
use std::io::Read;
use cqrs::EventNumber;
use cqrs_data::Expectation;
use serde::de::DeserializeOwned;
use serde::Serialize;

// getStreamPage
// getEvent
// appendEvents

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Embedding {
    DoNotEmbed,
    EmbedEvents,
}

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Fail)]
pub enum ErrorKind {
    #[fail(display = "invalid url")]
    InvalidUrl,
    #[fail(display = "EventStore returned an error {}", _0)]
    AppendFailure(StatusCode),
    #[fail(display = "EventStore indicated that the precondition for appending events was not satisfied")]
    AppendPreconditionFailed,
    #[fail(display = "problem while appending events to the EventStore")]
    NetworkIoAppend,
    #[fail(display = "problem requesting an event from EventStore")]
    NetworkIoEvent,
    #[fail(display = "problem requesting a stream page from EventStore")]
    NetworkIoStreamPage,
    #[fail(display = "serializing data prior to sending to EventStore")]
    Serialization,
    #[fail(display = "deserializing data from EventStore")]
    Deserialization,
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Error {
    fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(ek: ErrorKind) -> Self {
        Error {
            inner: Context::new(ek),
        }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(c: Context<ErrorKind>) -> Self {
        Error {
            inner: c,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

#[derive(Debug)]
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

    pub fn get_stream_page(&self, stream_id: &str, offset: EventNumber, limit: usize, embed: Embedding) -> Result<Option<dto::StreamPage>, Error> {
        let url = build_stream_page_url(&self.base_url, stream_id, offset, limit);
        self.get_stream_page_with_url(url, embed)
    }

    pub fn get_stream_page_with_url<U: hyper::client::IntoUrl>(&self, url: U, embed: Embedding) -> Result<Option<dto::StreamPage>, Error> {
        let mut url =
            hyper::client::IntoUrl::into_url(url)
                .context(ErrorKind::InvalidUrl)?;
        match embed {
            Embedding::EmbedEvents => { url.query_pairs_mut().append_pair("embed", "body"); },
            Embedding::DoNotEmbed => {},
        }

        let mut result = self.client.get(url)
            .header(self.credentials.clone())
            .header(ES_ATOM_ACCEPT.clone())
            .send()
            .context(ErrorKind::NetworkIoStreamPage)?;

        let mut data = String::new();
        result.read_to_string(&mut data)
            .context(ErrorKind::NetworkIoStreamPage)?;

//        println!("{:#?}", serde_json::from_str::<serde_json::Value>(&data));

        if result.status == StatusCode::NotFound || result.status == StatusCode::Gone {
            Ok(None)
        } else {
            Ok(Some(serde_json::from_str(&data).context(ErrorKind::Deserialization)?))
        }
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
            .send()
            .context(ErrorKind::NetworkIoEvent)?;

        Ok(serde_json::from_reader(result).context(ErrorKind::Deserialization)?)
    }

    pub fn append_events<D, M>(&self, stream_id: &str, append_events: &[dto::AppendEvent<D, M>], expected_event: Expectation) -> Result<EventNumber, Error>
        where
            D: Serialize,
            M: Serialize,
    {
        let serialized_body = serde_json::to_vec(append_events).context(ErrorKind::Serialization)?;
        let borrowed_body: &[u8] = serialized_body.borrow();
        let mut req = self.client.post(build_event_append_url(&self.base_url, stream_id))
            .body(borrowed_body)
            .header(self.credentials.clone())
            .header(ES_EVENTS_CONTENT.clone());

        req =
            match expected_event {
                Expectation::None => req.header(ExpectedVersion::Any),
                Expectation::New => req.header(ExpectedVersion::CreateNew),
                Expectation::Empty => req.header(ExpectedVersion::Empty),
                Expectation::LastEvent(event_number) => req.header(ExpectedVersion::LastEvent(event_number)),
            };

        let resp = req.send().context(ErrorKind::NetworkIoAppend)?;

        if resp.status == StatusCode::Created {
            let &header::Location(ref loc) = resp.headers.get().unwrap();
            let first_event_number: usize = loc.split('/').last().unwrap().parse().unwrap();

            Ok(EventNumber::new(first_event_number))
        } else if resp.status == StatusCode::PreconditionFailed {
            Err(ErrorKind::AppendPreconditionFailed)?
        } else {
//            println!("Error: {}\n {:#?}", resp.status, resp.headers);
//            let mut string = String::new();
//            resp.read_to_string(&mut string).unwrap();
//            println!("body: {}", string);
//            Ok(())
            Err(ErrorKind::AppendFailure(resp.status))?
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;