use std::fmt::{Debug, Display};
use aggregate::Aggregate;
use types::StateSnapshot;

pub trait SnapshotSource<A: Aggregate>: Sized {
    type Error: Debug + Display;

    fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id) -> Result<Option<StateSnapshot<A>>, Self::Error>;
}

pub trait SnapshotSink<A: Aggregate>: Sized {
    type Error: Debug + Display;

    fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id, snapshot: StateSnapshot<A>) -> Result<(), Self::Error>;
}
