use std::fmt::{Debug, Display};
use cqrs::StateSnapshot;

pub trait SnapshotSource<A: cqrs::Aggregate>: Sized {
    type Error: Debug + Display;

    fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id) -> Result<Option<StateSnapshot<A>>, Self::Error>;
}
