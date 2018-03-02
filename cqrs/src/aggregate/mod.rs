use projection::Projection;
use std::error;

pub mod hydrated;

pub trait Aggregate: Projection {
    type Command;
    type Events;
    type Error: error::Error;

    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error>;
}

pub trait PersistableAggregate: Aggregate {
    type Snapshot;

    fn restore(snapshot: Self::Snapshot) -> Self
        where Self: Sized;
    fn into_snapshot(self) -> Self::Snapshot
        where Self: Sized;
}


#[cfg(test)]
assert_obj_safe!(agg; Aggregate<Events=(), Event=(), Command=(), Error=::std::io::Error>);
#[cfg(test)]
assert_obj_safe!(pstagg; PersistableAggregate<Snapshot=(), Events=(), Event=(), Command=(), Error=::std::io::Error>);
