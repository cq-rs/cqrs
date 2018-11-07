use std::fmt::{Debug, Display};

//pub mod hydrated;

pub trait Aggregate: Default {
    type Event;
    type Events: IntoIterator<Item=Self::Event>;

    type Command;
    type Error: Debug + Display + Send + Sync + 'static;

    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error>;
    fn entity_type() -> &'static str;
}
