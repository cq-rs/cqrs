pub use super::*;
use trivial::{NullEventStore, NullSnapshotStore, NopEventDecorator};
use domain::query::QueryableSnapshotAggregate;
use domain::execute::ViewExecutor;
use domain::persist::{PersistableSnapshotAggregate};
use error::Never;
use smallvec::SmallVec;

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
    type Events = SmallVec<[Self::Event;1]>;
    type Event = MyEvent;
    type Command = MyCommand;
    type CommandError = Never;
    fn apply(&mut self, evt: Self::Event) {
        println!("applying {:?}", evt);
    }
    fn execute(&self, cmd: Self::Command) -> Result<Self::Events, Self::CommandError> {
        println!("execute {:?}", cmd);
        let mut v = SmallVec::new();
        v.push(MyEvent::Wow);
        Ok(v)
    }
}

impl SnapshotAggregate for CoolAggregate {
    type Snapshot = Self;

    fn as_snapshot(&self) -> Self::Snapshot {
        self.clone()
    }
}

impl RestoreAggregate for CoolAggregate {
    type Snapshot = Self;

    fn restore(snapshot: Self::Snapshot) -> Self {
        snapshot
    }
}

#[test]
fn maybe_this_works() {
    let es: NullEventStore<MyEvent, usize> = Default::default();
    let ss: NullSnapshotStore<CoolAggregate, usize> = Default::default();

    let view = CoolAggregate::snapshot_with_events_view(&es, &ss);
    let command_view = CoolAggregate::snapshot_with_events_view(&es, &ss);

    let x = CoolAggregate::persist_events_and_snapshot(ViewExecutor::new(command_view), &es, &ss);

    x.execute_and_persist_with_decorator(&0, MyCommand::Much, Some(AggregatePrecondition::New), NopEventDecorator::default()).unwrap();

    let agg: Option<HydratedAggregate<CoolAggregate>> = view.rehydrate(&0).unwrap();

    println!("{:?}", agg);
}
