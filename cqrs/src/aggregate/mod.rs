use std::fmt::Debug;

//pub mod hydrated;

pub trait Aggregate {
    type Event;
    type Command;
    type Events: IntoIterator<Item=Self::Event>;
    type Error: Debug;

    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error>;
    #[inline(always)]
    fn entity_type() -> &'static str where Self: Sized;
}

impl<E, C, Es: IntoIterator<Item=E>, Err: Debug> Projection for Aggregate<Event=E, Command=C, Events=Es, Error=Err> {
    type Event = E;

    fn apply(&mut self, event: Self::Event) {
        Aggregate::apply(self, event);
    }
}

impl<E, C, Es: IntoIterator<Item=E>, Err: Debug> CommandHandler<C> for Aggregate<Event=E, Command=C, Events=Es, Error=Err> {
    type Event = E;
    type Events = Es;
    type Error = Err;

    fn execute(&self, command: C) -> Result<Self::Events, Self::Error> {
        Aggregate::execute(self, command)
    }
}

#[cfg(test)]
assert_obj_safe!(agg; Aggregate<Events=(), Event=(), Command=(), Error=::std::io::Error>);

pub trait CommandHandler<Command> {
    type Event;
    type Events: IntoIterator<Item=Self::Event>;
    type Error;

    fn execute(&self, command: Command) -> Result<Self::Events, Self::Error>;
}

#[cfg(test)]
assert_obj_safe!(cmd; CommandHandler<(), Event=(), Events=Vec<()>, Error=()>);

pub trait Projection {
    type Event;

    fn apply(&mut self, event: Self::Event);
}

#[cfg(test)]
assert_obj_safe!(proj; Projection<Event=()>);