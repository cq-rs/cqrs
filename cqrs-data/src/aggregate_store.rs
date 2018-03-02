use std::marker::PhantomData;
use events;
use snapshots;

pub struct AggregateStore<'a, E, S>
    where
        E: events::Store<'a>,
        S: snapshots::Store + 'a,
{
    event_store: E,
    snapshot_store: S,
    _lifetime: PhantomData<&'a ()>
}

impl<'a, E, S> AggregateStore<'a, E, S>
    where
        E: events::Store<'a>,
        S: snapshots::Store + 'a,
{
    pub fn new(event_store: E, snapshot_store: S) -> Self {
        AggregateStore {
            event_store,
            snapshot_store,
            _lifetime: PhantomData,
        }
    }
}