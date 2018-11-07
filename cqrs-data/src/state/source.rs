use std::fmt::{Debug, Display};
use cqrs::StateSnapshot;

pub trait Source<State>: Sized {
    type Error: Debug + Display;

    fn get_snapshot<Id: AsRef<str> + Into<String>>(&self, id: Id) -> Result<Option<StateSnapshot<State>>, Self::Error>;
}
