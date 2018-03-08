use cqrs;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Since {
    BeginningOfStream,
    Event(cqrs::EventNumber),
}

