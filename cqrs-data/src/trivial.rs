use types::Since;
use cqrs::{EventNumber, Precondition, SequencedEvent, StateSnapshot};
use event;
use state;
use std::iter::Empty;
use void::Void;

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct NullStore;

impl<Event> event::Source<Event> for NullStore {
    type Events = Empty<Result<SequencedEvent<Event>, Void>>;
    type Error = Void;

    #[inline]
    fn read_events<Id: AsRef<str> + Into<String>>(&self, _id: Id, _version: Since) -> Result<Option<Self::Events>, Self::Error> {
        Ok(None)
    }
}

impl<Event> event::Store<Event> for NullStore {
    type Error = Void;

    #[inline]
    fn append_events<Id: AsRef<str> + Into<String>>(&self, _id: Id, _events: &[Event], _expect: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        Ok(EventNumber::MIN_VALUE)
    }
}

impl<State> state::Source<State> for NullStore {
    type Error = Void;

    #[inline]
    fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, _id: Id) -> Result<Option<StateSnapshot<State>>, Self::Error>
        where Self: Sized
    {
        Ok(None)
    }
}

impl<State> state::Store<State> for NullStore {
    type Error = Void;

    #[inline]
    fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, _id: Id, _snapshot: StateSnapshot<State>) -> Result<(), Self::Error>
        where Self: Sized
    {
        Ok(())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
