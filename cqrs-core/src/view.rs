use crate::{AggregateEvent, Event};

/// A downstream listener responding to committed events
pub trait View<E: Event> {
    /// Apply all events to the listener
    fn apply_events(&mut self, events: &Vec<E>);
}