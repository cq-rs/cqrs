use types::CqrsError;

pub trait Aggregate: Default {
    type Event: Event;
    type Events: IntoIterator<Item=Self::Event>;

    type Command;
    type Error: CqrsError;

    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error>;
    fn entity_type() -> &'static str;
}

pub trait Event {
    fn event_type(&self) -> &'static str;
}