use ::event::Source as EventSource;
use ::state::Source as StateSource;

pub struct EntitySource<'event, 'state, ES, SS, E, S>
where
    ES: EventSource<E> + 'event,
    SS: StateSource<S> + 'state,
{
    event_source: &'event ES,
    state_source: &'state SS,
    _phantom: ::std::marker::PhantomData<(E, S)>,
}