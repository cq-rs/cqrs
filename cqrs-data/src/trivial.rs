use types::Since;
use cqrs::{EventNumber, Precondition, SequencedEvent, StateSnapshot};
use event;
use state;
use std::marker::PhantomData;
use void::Void;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct NullEventStore<AggregateId: ?Sized> {
    _phantom: PhantomData<AggregateId>,
}

impl<AggregateId: ?Sized> Default for NullEventStore<AggregateId> {
    fn default() -> Self {
        NullEventStore {
            _phantom: PhantomData,
        }
    }
}

impl<Event, AggregateId: ?Sized> event::Source<Event> for NullEventStore<AggregateId> {
    type AggregateId = AggregateId;
    type Events = Vec<Result<SequencedEvent<Event>, Void>>;
    type Error = Void;

    #[inline]
    fn read_events(&self, _aggregate_id: &Self::AggregateId, _version: Since) -> Result<Option<Self::Events>, Self::Error> {
        Ok(None)
    }
}

impl<Event, AggregateId: ?Sized> event::Store<Event> for NullEventStore<AggregateId> {
    type AggregateId = AggregateId;
    type Error = Void;

    #[inline]
    fn append_events(&self, _aggregate_id: &Self::AggregateId, _events: &[Event], _expect: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        Ok(EventNumber::MIN_VALUE)
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct NullStateStore<AggregateId: ?Sized> {
    _phantom: PhantomData<AggregateId>,
}

impl<AggregateId: ?Sized> Default for NullStateStore<AggregateId> {
    fn default() -> Self {
        NullStateStore {
            _phantom: PhantomData,
        }
    }
}

impl<State, AggregateId: ?Sized> state::Source<State> for NullStateStore<AggregateId> {
    type AggregateId = AggregateId;
    type Error = Void;

    #[inline]
    fn get_snapshot(&self, _agg_id: &Self::AggregateId) -> Result<Option<StateSnapshot<State>>, Self::Error> {
        Ok(None)
    }
}

impl<State, AggregateId: ?Sized> state::Store<State> for NullStateStore<AggregateId> {
    type AggregateId = AggregateId;
    type Error = Void;

    #[inline]
    fn persist_snapshot(&self, _agg_id: &Self::AggregateId, _snapshot: StateSnapshot<State>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
