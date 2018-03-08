use projection::Projection;
use std::error;

pub mod hydrated;

pub trait Aggregate {
    type Event;
    type Command;
    type Events;
    type Error: error::Error;

    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error>;
}

impl<E, C, Es, Err: error::Error> Projection for Aggregate<Event=E, Command=C, Events=Es, Error=Err> {
    type Event = <Self as Aggregate>::Event;

    fn apply(&mut self, event: Self::Event) {
        Aggregate::apply(self, event);
    }
}

#[cfg(test)]
assert_obj_safe!(agg; Aggregate<Events=(), Event=(), Command=(), Error=::std::io::Error>);
