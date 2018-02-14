use super::{Version, Since, Precondition};
use super::{EventStore, StateStore, EventDecorator};
use super::{PersistResult, ReadStreamResult, ReadStateResult, Never};
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct NullEventStore<Event, AggregateId> {
    _phantom: PhantomData<(Event, AggregateId)>,
}

impl<Event, AggregateId> Default for NullEventStore<Event, AggregateId> {
    fn default() -> Self {
        NullEventStore {
            _phantom: PhantomData,
        }
    }
}

impl<Event, AggregateId> EventStore for NullEventStore<Event, AggregateId>
{
    type AggregateId = AggregateId;
    type Event = Event;
    type AppendResult = PersistResult<Never>;
    type ReadResult = ReadStreamResult<Self::Event, Never>;

    #[inline]
    fn append_events(&self, _aggregate_id: &Self::AggregateId, _events: &[Self::Event], _condition: Precondition) -> Self::AppendResult {
        Ok(())
    }

    #[inline]
    fn read(&self, _aggregate_id: &Self::AggregateId, _version: Since) -> Self::ReadResult {
        Ok(Some(Vec::new()))
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct NullStateStore<State, AggregateId> {
    _phantom: PhantomData<(State, AggregateId)>,
}

impl<State, AggregateId> Default for NullStateStore<State, AggregateId> {
    fn default() -> Self {
        NullStateStore {
            _phantom: PhantomData,
        }
    }
}

impl<State, AggregateId> StateStore for NullStateStore<State, AggregateId>
{
    type AggregateId = AggregateId;
    type State = State;
    type StateResult = ReadStateResult<Self::State, Never>;
    type PersistResult = PersistResult<Never>;

    #[inline]
    fn get_state(&self, _agg_id: &Self::AggregateId) -> Self::StateResult {
        Ok(None)
    }

    #[inline]
    fn put_state(&self, _agg_id: &Self::AggregateId, _version: Version, _state: Self::State) -> Self::PersistResult {
        Ok(())
    }
}

#[derive(Debug, PartialEq, Hash, Clone, Copy)]
pub struct NopEventDecorator<Event> {
    _phantom: PhantomData<Event>,
}

impl<Event> Default for NopEventDecorator<Event> {
    fn default() -> Self {
        NopEventDecorator {
            _phantom: PhantomData,
        }
    }
}

impl<Event> EventDecorator for NopEventDecorator<Event>
{
    type Event = Event;
    type DecoratedEvent = Event;

    #[inline]
    fn decorate(&self, event: Self::Event) -> Self::DecoratedEvent {
        event
    }
}

