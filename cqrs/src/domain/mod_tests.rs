pub use super::*;
use trivial::{NullEventStore, NullSnapshotStore, NopEventDecorator};
use domain::Aggregate;
use store::AggregateStore;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum MyEvent {
    Wow
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
enum MyCommand {
    Much
}

#[derive(Debug, Default, Copy, Clone, Hash, PartialEq, Eq)]
struct CoolAggregate;

impl Aggregate for CoolAggregate {
    type Event = MyEvent;
    type Snapshot = Self;
    type Command = MyCommand;
    type CommandError = ::Never;
    fn from_snapshot(x: Self) -> Self {
        x
    }
    fn apply(&mut self, _evt: Self::Event) {}
    fn execute(&self, _cmd: Self::Command) -> CommandResult<Self::Event, Self::CommandError> {
        Ok(vec![MyEvent::Wow])
    }
    fn snapshot(self) -> Self::Snapshot {
        self
    }
}

#[test]
fn maybe_this_works_() {
    let es: NullEventStore<MyEvent, usize> = Default::default();
    let ss: NullSnapshotStore<CoolAggregate, usize> = Default::default();

    let agg = AggregateStore::<CoolAggregate, _, _, _, _>::new(es, ss);

    let result =
        agg.execute_and_persist(
            &0,
            MyCommand::Much,
            NopEventDecorator::default());
    assert_eq!(result, Ok(1usize));
}
