use std::fmt::{Debug, Display};
use cqrs::SequencedEvent;
use types::Since;

pub trait EventSource<A: cqrs::Aggregate>: Sized {
    type Events: IntoIterator<Item=Result<SequencedEvent<A::Event>, Self::Error>>;
    type Error: Debug + Display;

    fn read_events<Id: AsRef<str> + Into<String>>(&self, id: Id, since: Since) -> Result<Option<Self::Events>, Self::Error>;
}

