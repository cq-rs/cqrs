use types::CqrsError;

pub trait Aggregate: Default {
    type Event;
    type Events: IntoIterator<Item=Self::Event>;

    type Command;
    type Error: CqrsError;

    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error>;
    fn entity_type() -> &'static str;
}

#[cfg(test)]
pub(crate) mod testing {
    use std::iter::Empty;
    use void::Void;
    use super::Aggregate;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct TestAggregate;

    impl Aggregate for TestAggregate {
        type Event = ();
        type Events = Empty<Self::Event>;

        type Command = ();
        type Error = Void;

        fn apply(&mut self, _event: Self::Event) {}

        fn execute(&self, _command: Self::Command) -> Result<Self::Events, Self::Error> {
            Ok(Empty::default())
        }

        fn entity_type() -> &'static str {
            "test"
        }
    }
}
