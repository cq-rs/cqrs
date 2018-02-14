pub mod trivial;

#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(usize);

impl Version {
    pub fn new(version: usize) -> Self {
        Version(version)
    }

    pub fn number(&self) -> usize {
        self.0
    }
}

impl PartialEq<usize> for Version {
    fn eq(&self, rhs: &usize) -> bool {
        *rhs == self.0
    }
}

impl PartialEq<Version> for usize {
    fn eq(&self, rhs: &Version) -> bool {
        rhs.0 == *self
    }
}

impl ::std::ops::Add<usize> for Version {
    type Output = Version;
    fn add(self, rhs: usize) -> Self::Output {
        Version(self.0 + rhs)
    }
}

impl ::std::ops::AddAssign<usize> for Version {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl ::std::convert::AsRef<usize> for Version {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Since {
    BeginningOfStream,
    Version(Version),
}

impl From<Version> for Since {
    fn from(v: Version) -> Self {
        Since::Version(v)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Precondition {
    Always,
    LastVersion(Version),
    NewStream,
    EmptyStream,
}

impl Default for Precondition {
    fn default() -> Self {
        Precondition::Always
    }
}

impl From<Version> for Precondition {
    fn from(v: Version) -> Self {
        Precondition::LastVersion(v)
    }
}

impl From<Option<Version>> for Precondition {
    fn from(v_o: Option<Version>) -> Self {
        if let Some(v) = v_o {
            v.into()
        } else {
            Precondition::EmptyStream
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum AppendError<Err> {
    PreconditionFailed(Precondition),
    WriteError(Err),
}

impl<Err> From<Precondition> for AppendError<Err> {
    fn from(p: Precondition) -> Self {
        AppendError::PreconditionFailed(p)
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedEvent<Event>
{
    pub version: Version,
    pub event: Event,
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct PersistedSnapshot<State> {
    pub version: Version,
    pub data: State,
}

pub trait EventStore {
    type AggregateId;
    type Event;
    type AppendResult;
    type ReadResult;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], condition: Precondition) -> Self::AppendResult;
    fn read(&self, agg_id: &Self::AggregateId, since: Since) -> Self::ReadResult;
}

pub trait StateStore {
    type AggregateId;
    type State;
    type StateResult;
    type PersistResult;

    fn get_state(&self, agg_id: &Self::AggregateId) -> Self::StateResult;
    fn put_state(&self, agg_id: &Self::AggregateId, version: Version, state: Self::State) -> Self::PersistResult;
}

pub trait EventDecorator {
    type Event;
    type DecoratedEvent;

    fn decorate(&self, event: Self::Event) -> Self::DecoratedEvent;
    fn decorate_events(&self, events: Vec<Self::Event>) -> Vec<Self::DecoratedEvent> {
        events.into_iter()
            .map(|e| self.decorate(e))
            .collect()
    }
}

pub type CommandResult<Event, Error> = Result<Vec<Event>, Error>;
pub type ReadResult<Data, Error> = Result<Option<Data>, Error>;
pub type ReadStreamResult<Event, Error> = ReadResult<Vec<PersistedEvent<Event>>, Error>;
pub type ReadStateResult<State, Error> = ReadResult<PersistedSnapshot<State>, Error>;
pub type PersistResult<Error> = Result<(), Error>;

pub trait Aggregate: Default {
    type Event;
    type Command;
    type CommandError;
    fn apply(&mut self, evt: Self::Event);
    fn execute(&self, cmd: Self::Command) -> CommandResult<Self::Event, Self::CommandError>;
    fn should_snapshot(&self) -> SnapshotDecision {
        SnapshotDecision::Skip
    }
}

#[cfg(not(feature = "never_type"))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Never {}

#[cfg(feature = "never_type")]
type Never = !;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SnapshotDecision {
    Persist,
    Skip,
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
struct AlwaysSnapshotAggregate<A: Aggregate>(A);

impl<A: Aggregate> Aggregate for AlwaysSnapshotAggregate<A> {
    type Event = A::Event;
    type Command = A::Command;
    type CommandError = A::CommandError;

    #[inline]
    fn apply(&mut self, evt: Self::Event) {
        self.0.apply(evt);
    }

    #[inline]
    fn execute(&self, cmd: Self::Command) -> CommandResult<Self::Event, Self::CommandError> {
        self.0.execute(cmd)
    }

    #[inline]
    fn should_snapshot(&self) -> SnapshotDecision {
        SnapshotDecision::Persist
    }
}

#[derive(Debug, Hash, PartialEq, Clone)]
pub struct AggregateStore<EventStore, StateStore>
    where
        EventStore: self::EventStore,
        StateStore: self::StateStore<AggregateId=EventStore::AggregateId>,
{
    event_store: EventStore,
    state_store: StateStore,
}

#[derive(Debug, Hash, PartialEq, Clone)]
pub enum AggregateError<CmdErr, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr> {
    BadCommand(CmdErr),
    ReadStream(ReadStreamErr),
    ReadState(ReadStateErr),
    WriteStream(WriteStreamErr),
    WriteState(WriteStateErr),
}

impl<Aggregate, AggregateId, EventStore, StateStore, Event, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>
AggregateStore<EventStore, StateStore>
    where
        EventStore: self::EventStore<
            AggregateId=AggregateId,
            Event=Event,
            ReadResult=ReadStreamResult<Event, ReadStreamErr>,
            AppendResult=PersistResult<WriteStreamErr>>,
        StateStore: self::StateStore<
            State=Aggregate,
            AggregateId=AggregateId,
            StateResult=ReadStateResult<Aggregate, ReadStateErr>,
            PersistResult=PersistResult<WriteStateErr>>,
        Aggregate: self::Aggregate<Event=Event>,
        Event: Clone,
{
    pub fn new(event_store: EventStore, state_store: StateStore) -> Self {
        AggregateStore {
            event_store,
            state_store,
        }
    }

    pub fn execute_and_persist<D>(&self, agg_id: &AggregateId, cmd: Aggregate::Command, decorator: D) -> Result<usize, AggregateError<Aggregate::CommandError, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>>
        where
            D: EventDecorator<Event=Event, DecoratedEvent=Event>,
    {
        let saved_snapshot = self.get_last_snapshot(&agg_id)?;

        // Instantiate aggregate with snapshot or default data
        let (snapshot_version, mut state) =
            if let Some(snapshot) = saved_snapshot {
                (Some(snapshot.version), snapshot.data)
            } else {
                (None, Aggregate::default())
            };

        let (read_since, mut version) =
            if let Some(v) = snapshot_version {
                (Since::Version(v), Some(v))
            } else {
                (Since::BeginningOfStream, None)
            };

        if let Some(event_version) = self.rehydrate(&agg_id, &mut state, read_since)? {
            version = Some(event_version);
        }

        // Apply command to aggregate
        let events =
            state.execute(cmd)
                .map_err(|e| AggregateError::BadCommand(e))?;

        let event_count = events.len();

        // Skip if no new events
        if event_count > 0 {
            let decorated_events = decorator.decorate_events(events);

            let precondition =
                if let Some(v) = version {
                    Precondition::LastVersion(v)
                } else {
                    Precondition::EmptyStream
                };

            // Append new events to event store if underlying stream
            // has not changed
            self.event_store.append_events(&agg_id, &decorated_events, precondition)
                .map_err(|e| AggregateError::WriteStream(e))?;

            for e in decorated_events {
                state.apply(e)
            }

            if state.should_snapshot() == SnapshotDecision::Persist {
                let new_snapshot_version =
                    if let Some(v) = version {
                        v + event_count
                    } else {
                        Version::new(event_count - 1)
                    };

                self.state_store.put_state(&agg_id, new_snapshot_version, state)
                    .map_err(|e| AggregateError::WriteState(e))?;
            }
        }

        Ok(event_count)
    }

    fn rehydrate(&self, agg_id: &AggregateId, agg: &mut Aggregate, since: Since) -> Result<Option<Version>, AggregateError<Aggregate::CommandError, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>> {
        let read_events =
            self.event_store.read(agg_id, since)
                .map_err(|e| AggregateError::ReadStream(e))?;

        let mut latest_version = None;
        if let Some(events) = read_events {
            // Re-hydrate aggregate
            for e in events {
                agg.apply(e.event);
                latest_version = Some(e.version);
            }
        }

        Ok(latest_version)
    }

    fn get_last_snapshot(&self, agg_id: &AggregateId) -> ReadStateResult<Aggregate, AggregateError<Aggregate::CommandError, ReadStreamErr, ReadStateErr, WriteStreamErr, WriteStateErr>> {
        self.state_store.get_state(&agg_id)
            .map_err(|e| AggregateError::ReadState(e))
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;

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
        type Command = MyCommand;
        type CommandError = Never;
        fn apply(&mut self, _evt: Self::Event) {}
        fn execute(&self, _cmd: Self::Command) -> CommandResult<Self::Event, Self::CommandError> {
            Ok(vec![MyEvent::Wow])
        }
    }

    #[test]
    fn maybe_this_works_() {
        let es: trivial::NullEventStore<MyEvent, usize> =
            NullEventStore { _phantom: ::std::marker::PhantomData };
        let ss: trivial::NullStateStore<CoolAggregate, usize> =
            NullStateStore { _phantom: ::std::marker::PhantomData };

        let agg = AggregateStore::new(es, ss);

        let result =
            agg.execute_and_persist(
                &0,
                MyCommand::Much,
                trivial::NopEventDecorator::default());
        assert_eq!(result, Ok(1usize));
    }
}