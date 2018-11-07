use std::fmt::{Debug, Display};
use cqrs::SequencedEvent;
use types::Since;

pub trait Source<Event>: Sized {
    type Events: IntoIterator<Item=Result<SequencedEvent<Event>, Self::Error>>;
    type Error: Debug + Display;

    fn read_events<Id: AsRef<str> + Into<String>>(&self, id: Id, since: Since) -> Result<Option<Self::Events>, Self::Error>;
}

