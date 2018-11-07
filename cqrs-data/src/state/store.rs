use std::fmt::{Debug, Display};
use cqrs::StateSnapshot;

pub trait Store<State>: Sized {
    type Error: Debug + Display;

    fn persist_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id, snapshot: StateSnapshot<State>) -> Result<(), Self::Error>;
}
