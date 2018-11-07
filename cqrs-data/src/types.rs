use cqrs;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Since {
    BeginningOfStream,
    Event(cqrs::EventNumber),
}

impl From<cqrs::Version> for Since {
    fn from(v: cqrs::Version) -> Self {
        match v {
            cqrs::Version::Initial => Since::BeginningOfStream,
            cqrs::Version::Number(x) => Since::Event(x),
        }
    }
}