use types::{Expectation, Since};
use cqrs::error::{Never};
use cqrs::{EventNumber, SequencedEvent, StateSnapshot};
use event;
use state;
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct NullEventStore<AggregateId> {
    _phantom: PhantomData<AggregateId>,
}

impl<AggregateId> Default for NullEventStore<AggregateId> {
    fn default() -> Self {
        NullEventStore {
            _phantom: PhantomData,
        }
    }
}

impl<'a, Event, AggregateId: 'a> event::Source<'a, Event> for NullEventStore<AggregateId> {
    type AggregateId = AggregateId;
    type Events = Vec<Result<SequencedEvent<Event>, Never>>;
    type Error = Never;

    #[inline]
    fn read_events(&self, _aggregate_id: Self::AggregateId, _version: Since) -> Result<Option<Self::Events>, Self::Error> {
        Ok(None)
    }
}

impl<'id, Event, AggregateId: 'id> event::Store<'id, Event> for NullEventStore<AggregateId> {
    type AggregateId = AggregateId;
    type Error = Never;

    #[inline]
    fn append_events(&self, _aggregate_id: Self::AggregateId, _events: &[Event], _expect: Expectation) -> Result<EventNumber, Never> {
        Ok(EventNumber::default())
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct NullStateStore<AggregateId> {
    _phantom: PhantomData<AggregateId>,
}

impl<AggregateId> Default for NullStateStore<AggregateId> {
    fn default() -> Self {
        NullStateStore {
            _phantom: PhantomData,
        }
    }
}

impl<'id, State, AggregateId: 'id> state::Source<'id, State> for NullStateStore<AggregateId> {
    type AggregateId = AggregateId;
    type Error = Never;

    #[inline]
    fn get_snapshot(&self, _agg_id: Self::AggregateId) -> Result<Option<StateSnapshot<State>>, Self::Error> {
        Ok(None)
    }
}

impl<'id, State, AggregateId: 'id> state::Store<'id, State> for NullStateStore<AggregateId> {
    type AggregateId = AggregateId;
    type Error = Never;

    #[inline]
    fn persist_snapshot(&self, _agg_id: Self::AggregateId, _snapshot: StateSnapshot<State>) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
