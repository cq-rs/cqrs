pub mod sync;

pub type CommandResult<Event, Error> = Result<Vec<Event>, Error>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SnapshotChoice {
    Persist,
    Skip,
}

pub trait Aggregate: Default {
    type Event;
    type Snapshot;
    type Command;
    type CommandError;

    fn from_snapshot(snapshot: Self::Snapshot) -> Self;
    fn apply(&mut self, event: Self::Event);
    fn execute(&self, command: Self::Command) -> CommandResult<Self::Event, Self::CommandError>;
    fn should_snapshot(&self) -> SnapshotChoice {
        SnapshotChoice::Skip
    }
    fn snapshot(self) -> Self::Snapshot;
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
struct AlwaysSnapshotAggregate<A: Aggregate>(A);

impl<A: Aggregate> Aggregate for AlwaysSnapshotAggregate<A> {
    type Event = A::Event;
    type Snapshot = A::Snapshot;
    type Command = A::Command;
    type CommandError = A::CommandError;

    #[inline]
    fn from_snapshot(snapshot: Self::Snapshot) -> Self {
        AlwaysSnapshotAggregate(A::from_snapshot(snapshot))
    }

    #[inline]
    fn apply(&mut self, evt: Self::Event) {
        self.0.apply(evt);
    }

    #[inline]
    fn execute(&self, cmd: Self::Command) -> CommandResult<Self::Event, Self::CommandError> {
        self.0.execute(cmd)
    }

    #[inline]
    fn should_snapshot(&self) -> SnapshotChoice {
        SnapshotChoice::Persist
    }

    #[inline]
    fn snapshot(self) -> Self::Snapshot {
        self.0.snapshot()
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;