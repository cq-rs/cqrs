use ::event::EventSource as EventSource;
use ::state::SnapshotSource as StateSource;

pub struct EntitySource<'event, 'state, ES, SS, A>
where
    A: cqrs::Aggregate,
    ES: EventSource<A> + 'event,
    SS: StateSource<A> + 'state,
{
    event_source: &'event ES,
    state_source: &'state SS,
    _phantom: ::std::marker::PhantomData<A>,
}