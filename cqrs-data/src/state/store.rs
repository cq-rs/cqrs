use std::fmt::{Debug, Display};
use cqrs::StateSnapshot;

pub trait SnapshotSink<A: cqrs::Aggregate>: Sized {
    type Error: Debug + Display;

    fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id, snapshot: StateSnapshot<A>) -> Result<(), Self::Error>;
}
