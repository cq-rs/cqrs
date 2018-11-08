use std::borrow::{Borrow, BorrowMut};
use std::fmt::{self, Display};
use std::marker::PhantomData;
use trivial::NullStore;
use super::*;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct HydratedAggregate<A>
where
    A: Aggregate,
{
    version: Version,
    snapshot_version: Version,
    state: A,
}

impl<A> HydratedAggregate<A>
where
    A: Aggregate,
{
    pub fn version(&self) -> Version {
        self.version
    }

    pub fn snapshot_version(&self) -> Version {
        self.snapshot_version
    }

    pub fn state(&self) -> &A {
        &self.state
    }

    pub fn apply_events(&mut self, events: A::Events) {
        for event in events {
            self.apply(event);
        }
    }

    pub fn apply(&mut self, event: A::Event) {
        self.state.apply(event);
        self.version = self.version.incr();
    }

    pub fn to_entity_with_id<I: AsRef<str>>(self, id: I) -> Entity<I, A> {
        Entity::new(id, self)
    }
}

impl<A> AsRef<A> for HydratedAggregate<A>
where
    A: Aggregate,
{
    fn as_ref(&self) -> &A {
        &self.state
    }
}

impl<A> Borrow<A> for HydratedAggregate<A>
where
    A: Aggregate,
{
    fn borrow(&self) -> &A {
        &self.state
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Entity<I, A>
where
    A: Aggregate,
    I: AsRef<str>,
{
    id: I,
    aggregate: HydratedAggregate<A>,
}

impl<I, A> Entity<I, A>
where
    A: Aggregate,
    I: AsRef<str>,
{
    pub fn new(id: I, aggregate: HydratedAggregate<A>) -> Self {
        Entity {
            id,
            aggregate,
        }
    }

    pub fn id(&self) -> &str {
        self.id.as_ref()
    }

    pub fn aggregate(&self) -> &HydratedAggregate<A> {
        &self.aggregate
    }

    pub fn aggregate_mut(&mut self) -> &mut HydratedAggregate<A> {
        &mut self.aggregate
    }
}

impl<I, A> From<Entity<I, A>> for HydratedAggregate<A>
where
    A: Aggregate,
    I: AsRef<str>,
{
    fn from(entity: Entity<I, A>) -> Self {
        entity.aggregate
    }
}

impl<I, A> AsRef<HydratedAggregate<A>> for Entity<I, A>
where
    A: Aggregate,
    I: AsRef<str>,
{
    fn as_ref(&self) -> &HydratedAggregate<A> {
        &self.aggregate
    }
}

impl<I, A> AsMut<HydratedAggregate<A>> for Entity<I, A>
    where
        A: Aggregate,
        I: AsRef<str>,
{
    fn as_mut(&mut self) -> &mut HydratedAggregate<A> {
        &mut self.aggregate
    }
}

impl<I, A> Borrow<HydratedAggregate<A>> for Entity<I, A>
where
    A: Aggregate,
    I: AsRef<str>,
{
    fn borrow(&self) -> &HydratedAggregate<A> {
        &self.aggregate
    }
}

impl<I, A> Borrow<A> for Entity<I, A>
    where
        A: Aggregate,
        I: AsRef<str>,
{
    fn borrow(&self) -> &A {
        self.aggregate.borrow()
    }
}

impl<I, A> BorrowMut<HydratedAggregate<A>> for Entity<I, A>
    where
        A: Aggregate,
        I: AsRef<str>,
{
    fn borrow_mut(&mut self) -> &mut HydratedAggregate<A> {
        &mut self.aggregate
    }
}

pub trait EntitySource<A>: EventSource<A> + SnapshotSource<A>
where A: Aggregate
{
    fn load_from_snapshot(
        &self,
        id: &str,
    ) -> Result<Option<HydratedAggregate<A>>, <Self as SnapshotSource<A>>::Error>
    {
        let entity =
            self.get_snapshot(id)?.map(|state|
                HydratedAggregate {
                    version: state.version,
                    snapshot_version: state.version,
                    state: state.snapshot,
                }
            );

        Ok(entity)
    }

    fn refresh(
        &self,
        id: &str,
        aggregate: &mut HydratedAggregate<A>,
    ) -> Result<(), <Self as EventSource<A>>::Error>
    {
        let seq_events = self.read_events(id, aggregate.version.into())?;

        if let Some(seq_events) = seq_events {
            for seq_event in seq_events {
                let seq_event = seq_event?;

                aggregate.apply(seq_event.event);

                debug_assert_eq!(Version::Number(seq_event.sequence), aggregate.version);
            }
        }

        Ok(())
    }

    fn rehydrate(
        &self,
        id: &str,
    ) -> EntityRefreshResult<A, Self>
    {
        let aggregate = self.load_from_snapshot(id).map_err(EntityLoadError::SnapshotSource)?;

        let missing = aggregate.is_none();

        let mut aggregate = aggregate.unwrap_or_default();

        self.refresh(id, &mut aggregate).map_err(EntityLoadError::EventSource)?;

        if missing && aggregate.version == Version::Initial {
            Ok(None)
        } else {
            Ok(Some(aggregate))
        }
    }
}

pub type EntityRefreshResult<A, S> = Result<Option<HydratedAggregate<A>>, EntityLoadError<<S as EventSource<A>>::Error, <S as SnapshotSource<A>>::Error>>;
pub type EntityPersistResult<A, S> = Result<(), EntityPersistError<<S as EventSink<A>>::Error, <S as SnapshotSink<A>>::Error>>;
pub type EntityExecAndPersistResult<A, S> =
    Result<
        HydratedAggregate<A>,
        EntityExecAndPersistError<
            A,
            <S as EventSink<A>>::Error,
            <S as SnapshotSink<A>>::Error,
        >
    >;
pub type EntityResult<A, S> =
    Result<
        HydratedAggregate<A>,
        EntityError<
            <S as EventSource<A>>::Error,
            <S as SnapshotSource<A>>::Error,
            A,
            <S as EventSink<A>>::Error,
            <S as SnapshotSink<A>>::Error,
        >,
    >;
pub type EntityOptionResult<A, S> =
    Result<
        Option<HydratedAggregate<A>>,
        EntityError<
            <S as EventSource<A>>::Error,
            <S as SnapshotSource<A>>::Error,
            A,
            <S as EventSink<A>>::Error,
            <S as SnapshotSink<A>>::Error,
        >,
    >;

impl<A, T> EntitySource<A> for T
where
    A: Aggregate,
    T: EventSource<A> + SnapshotSource<A>
{}

pub trait EntitySink<A>: EventSink<A> + SnapshotSink<A>
    where A: Aggregate
{
    fn apply_events_and_persist(
        &self,
        id: &str,
        aggregate: &mut HydratedAggregate<A>,
        events: A::Events,
        expected_version: Version,
        max_events_before_snapshot: u64
    ) -> EntityPersistResult<A, Self>
    {
        let events: Vec<A::Event> = events.into_iter().collect();
        self.append_events(id, &events, Some(Precondition::ExpectedVersion(expected_version))).map_err(EntityPersistError::EventSink)?;

        for event in events {
            aggregate.apply(event);
        }

        if aggregate.version - aggregate.snapshot_version >= max_events_before_snapshot as i64 {
            let snapshot = StateSnapshotView {
                snapshot: &aggregate.state,
                version: aggregate.version,
            };
            self.persist_snapshot(id, snapshot).map_err(EntityPersistError::SnapshotSink)?
        }

        Ok(())
    }

    fn exec_and_persist(
        &self,
        id: &str,
        aggregate: HydratedAggregate<A>,
        command: A::Command,
        precondition: Option<Precondition>,
        max_events_before_snapshot: u64,
    ) -> EntityExecAndPersistResult<A, Self>
    {
        let mut aggregate = aggregate;

        if let Some(precondition) = precondition {
            precondition.verify(Some(aggregate.version))?;
        }

        let expected_version = aggregate.version;

        match aggregate.state.execute(command) {
            Ok(events) => {
                self.apply_events_and_persist(
                    id,
                    &mut aggregate,
                    events,
                    expected_version,
                    max_events_before_snapshot,
                ).map_err(EntityExecAndPersistError::Persist)?;
            },
            Err(e) => {
                return Err(EntityExecAndPersistError::Exec(aggregate, e));
            }
        }

        Ok(aggregate)
    }
}

impl<A, T> EntitySink<A> for T
    where
        A: Aggregate,
        T: EventSink<A> + SnapshotSink<A>
{}

pub trait EntityStore<A>: EntitySource<A> + EntitySink<A>
    where A: Aggregate
{
    fn load_or_default_exec_and_persist(
        &self,
        id: &str,
        command: A::Command,
        precondition: Option<Precondition>,
        max_events_before_snapshot: u64
    ) -> EntityResult<A, Self>
    {
        let aggregate = self.rehydrate(id).map_err(EntityError::Load)?.unwrap_or_default();
        let aggregate = self.exec_and_persist(
            id,
            aggregate,
            command,
            precondition,
            max_events_before_snapshot,
        )?;

        Ok(aggregate)
    }

    fn load_exec_and_persist(
        &self,
        id: &str,
        command: A::Command,
        precondition: Option<Precondition>,
        max_events_before_snapshot: u64
    ) -> EntityOptionResult<A, Self>
    {
        if let Some(aggregate) = self.rehydrate(id).map_err(EntityError::Load)? {
            let aggregate = self.exec_and_persist(
                id,
                aggregate,
                command,
                precondition,
                max_events_before_snapshot,
            )?;

            Ok(Some(aggregate))
        } else {
            Ok(None)
        }
    }
}

impl<A, T> EntityStore<A> for T
    where
        A: Aggregate,
        T: EntitySource<A> + EntitySink<A>
{}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeEntitySource<'e, 's, A, E, S>
where
    A: Aggregate,
    E: EventSource<A> + 'e,
    S: SnapshotSource<A> + 's,
{
    event_source: &'e E,
    snapshot_source: &'s S,
    _phantom: PhantomData<A>,
}

impl<A> Default for CompositeEntitySource<'static, 'static, A, NullStore, NullStore>
where
    A: Aggregate,
{
    fn default() -> Self {
        CompositeEntitySource {
            event_source: &NullStore,
            snapshot_source: &NullStore,
            _phantom: PhantomData,
        }
    }
}

impl<'e, 's, A, E, S> CompositeEntitySource<'e, 's, A, E, S>
where
    A: Aggregate,
    E: EventSource<A> + 'e,
    S: SnapshotSource<A> + 's,
{
    pub fn with_event_source<'new_e, NewE>(self, event_source: &'new_e NewE) -> CompositeEntitySource<'new_e, 's, A, NewE, S>
    where
        NewE: EventSource<A> + 'new_e,
    {
        CompositeEntitySource {
            event_source,
            snapshot_source: self.snapshot_source,
            _phantom: PhantomData,
        }
    }

    pub fn with_snapshot_source<'new_s, NewS>(self, snapshot_source: &'new_s NewS) -> CompositeEntitySource<'e, 'new_s, A, E, NewS>
        where
            NewS: SnapshotSource<A> + 'new_s,
    {
        CompositeEntitySource {
            event_source: self.event_source,
            snapshot_source,
            _phantom: PhantomData,
        }
    }
}

impl<'e, 's, A, E, S> EventSource<A> for CompositeEntitySource<'e, 's, A, E, S>
where
    A: Aggregate,
    E: EventSource<A> + 'e,
    S: SnapshotSource<A> + 's,
{
    type Events = E::Events;
    type Error = E::Error;

    fn read_events(&self, id: &str, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        self.event_source.read_events(id, since)
    }
}

impl<'e, 's, A, E, S> SnapshotSource<A> for CompositeEntitySource<'e, 's, A, E, S>
where
    A: Aggregate,
    E: EventSource<A> + 'e,
    S: SnapshotSource<A> + 's,
{
    type Error = S::Error;

    fn get_snapshot(&self, id: &str) -> Result<Option<StateSnapshot<A>>, <Self as SnapshotSource<A>>::Error> {
        self.snapshot_source.get_snapshot(id)
    }
}

#[cfg(test)]
#[test]
fn can_construct_composite_entity_source() {
    let null = NullStore;
    let memory = ::memory::StateStore::<::memory::tests::TestAggregate>::default();
    let _source =
        CompositeEntitySource::default()
            .with_event_source(&null)
            .with_snapshot_source(&memory);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeEntitySink<'e, 's, A, E, S>
    where
        A: Aggregate,
        E: EventSink<A> + 'e,
        S: SnapshotSink<A> + 's,
{
    event_sink: &'e E,
    snapshot_sink: &'s S,
    _phantom: PhantomData<A>,
}

impl<A> Default for CompositeEntitySink<'static, 'static, A, NullStore, NullStore>
    where
        A: Aggregate,
{
    fn default() -> Self {
        CompositeEntitySink {
            event_sink: &NullStore,
            snapshot_sink: &NullStore,
            _phantom: PhantomData,
        }
    }
}

impl<'e, 's, A, E, S> CompositeEntitySink<'e, 's, A, E, S>
    where
        A: Aggregate,
        E: EventSink<A> + 'e,
        S: SnapshotSink<A> + 's,
{
    pub fn with_event_sink<'new_e, NewE>(self, event_sink: &'new_e NewE) -> CompositeEntitySink<'new_e, 's, A, NewE, S>
        where
            NewE: EventSink<A> + 'new_e,
    {
        CompositeEntitySink {
            event_sink,
            snapshot_sink: self.snapshot_sink,
            _phantom: PhantomData,
        }
    }

    pub fn with_snapshot_sink<'new_s, NewS>(self, snapshot_sink: &'new_s NewS) -> CompositeEntitySink<'e, 'new_s, A, E, NewS>
        where
            NewS: SnapshotSink<A> + 'new_s,
    {
        CompositeEntitySink {
            event_sink: self.event_sink,
            snapshot_sink,
            _phantom: PhantomData,
        }
    }
}

impl<'e, 's, A, E, S> EventSink<A> for CompositeEntitySink<'e, 's, A, E, S>
    where
        A: Aggregate,
        E: EventSink<A> + 'e,
        S: SnapshotSink<A> + 's,
{
    type Error = E::Error;

    fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        self.event_sink.append_events(id, events, precondition)
    }
}

impl<'e, 's, A, E, S> SnapshotSink<A> for CompositeEntitySink<'e, 's, A, E, S>
    where
        A: Aggregate,
        E: EventSink<A> + 'e,
        S: SnapshotSink<A> + 's,
{
    type Error = S::Error;

    fn persist_snapshot(&self, id: &str, snapshot: StateSnapshotView<A>) -> Result<(), Self::Error> {
        self.snapshot_sink.persist_snapshot(id, snapshot)
    }
}

#[cfg(test)]
#[test]
fn can_construct_composite_entity_sink() {
    let null = NullStore;
    let memory = ::memory::StateStore::<::memory::tests::TestAggregate>::default();
    let _sink =
        CompositeEntitySink::default()
            .with_event_sink(&null)
            .with_snapshot_sink(&memory);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeEntityStore<A, E, S>
where
    A: Aggregate,
    E: EntitySource<A>,
    S: EntitySink<A>,
{
    entity_source: E,
    entity_sink: S,
    _phantom: PhantomData<A>,
}

impl<A> Default for CompositeEntityStore<A, NullStore, NullStore>
where
    A: Aggregate,
{
    fn default() -> Self {
        CompositeEntityStore {
            entity_source: NullStore,
            entity_sink: NullStore,
            _phantom: PhantomData,
        }
    }
}

impl<A, E, S> CompositeEntityStore<A, E, S>
where
    A: Aggregate,
    E: EntitySource<A>,
    S: EntitySink<A>,
{
    pub fn with_entity_source<NewE>(self, entity_source: NewE) -> CompositeEntityStore<A, NewE, S>
        where
            NewE: EntitySource<A>,
    {
        CompositeEntityStore {
            entity_source,
            entity_sink: self.entity_sink,
            _phantom: PhantomData,
        }
    }

    pub fn with_entity_sink<NewS>(self, entity_sink: NewS) -> CompositeEntityStore<A, E, NewS>
        where
            NewS: EntitySink<A>,
    {
        CompositeEntityStore {
            entity_source: self.entity_source,
            entity_sink,
            _phantom: PhantomData,
        }
    }
}

impl<A, E, S> EventSource<A> for CompositeEntityStore<A, E, S>
where
    A: Aggregate,
    E: EntitySource<A>,
    S: EntitySink<A>,
{
    type Events = <E as EventSource<A>>::Events;
    type Error = <E as EventSource<A>>::Error;

    fn read_events(&self, id: &str, since: Since) -> Result<Option<Self::Events>, Self::Error> {
        self.entity_source.read_events(id, since)
    }
}

impl<A, E, S> SnapshotSource<A> for CompositeEntityStore<A, E, S>
where
    A: Aggregate,
    E: EntitySource<A>,
    S: EntitySink<A>,
{
    type Error = <E as SnapshotSource<A>>::Error;

    fn get_snapshot(&self, id: &str) -> Result<Option<StateSnapshot<A>>, <Self as SnapshotSource<A>>::Error> {
        self.entity_source.get_snapshot(id)
    }
}

impl<A, E, S> EventSink<A> for CompositeEntityStore<A, E, S>
where
    A: Aggregate,
    E: EntitySource<A>,
    S: EntitySink<A>,
{
    type Error = <S as EventSink<A>>::Error;

    fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error> {
        self.entity_sink.append_events(id, events, precondition)
    }
}

impl<A, E, S> SnapshotSink<A> for CompositeEntityStore<A, E, S>
where
    A: Aggregate,
    E: EntitySource<A>,
    S: EntitySink<A>,
{
    type Error = <S as SnapshotSink<A>>::Error;

    fn persist_snapshot(&self, id: &str, snapshot: StateSnapshotView<A>) -> Result<(), Self::Error> {
        self.entity_sink.persist_snapshot(id, snapshot)
    }
}

#[cfg(test)]
#[test]
fn can_construct_composite_entity_store() {
    let null = NullStore;
    let memory = ::memory::StateStore::<::memory::tests::TestAggregate>::default();
    let source =
        CompositeEntitySource::default()
            .with_event_source(&null)
            .with_snapshot_source(&memory);
    let sink =
        CompositeEntitySink::default()
            .with_event_sink(&null)
            .with_snapshot_sink(&memory);
    let _store =
        CompositeEntityStore::default()
            .with_entity_source(source)
            .with_entity_sink(sink);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityLoadError<EErr, SErr>
where
    EErr: CqrsError,
    SErr: CqrsError,
{
    EventSource(EErr),
    SnapshotSource(SErr),
}

impl<EErr, SErr> fmt::Display for EntityLoadError<EErr, SErr>
where
    EErr: CqrsError,
    SErr: CqrsError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityLoadError::EventSource(e) =>
                write!(f, "entity load error, problem loading events: {}", e),
            EntityLoadError::SnapshotSource(e) =>
                write!(f, "entity load error, problem loading snapshot: {}", e),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityPersistError<EErr, SErr>
where
    EErr: CqrsError,
    SErr: CqrsError,
{
    EventSink(EErr),
    SnapshotSink(SErr),
}

impl<EErr, SErr> fmt::Display for EntityPersistError<EErr, SErr>
where
    EErr: CqrsError,
    SErr: CqrsError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityPersistError::EventSink(e) =>
                write!(f, "entity persist error, problem persisting events: {}", e),
            EntityPersistError::SnapshotSink(e) =>
                write!(f, "entity persist error, problem persisting snapshot (events successfully persisted): {}", e),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityExecAndPersistError<A, PEErr, PSErr>
    where
        A: Aggregate,
        PEErr: CqrsError,
        PSErr: CqrsError,
{
    PreconditionFailed(Precondition),
    Exec(HydratedAggregate<A>, A::Error),
    Persist(EntityPersistError<PEErr, PSErr>),
}

impl<A, PEErr, PSErr> fmt::Display for EntityExecAndPersistError<A, PEErr, PSErr>
    where
        A: Aggregate,
        PEErr: CqrsError,
        PSErr: CqrsError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityExecAndPersistError::PreconditionFailed(p) =>
                write!(f, "entity exec error, precondition failed: {}", p),
            EntityExecAndPersistError::Exec(_, e) =>
                write!(f, "entity exec error, command was rejected: {}", e),
            EntityExecAndPersistError::Persist(e) => Display::fmt(&e, f),
        }
    }
}

impl<A, PEErr, PSErr> From<Precondition> for EntityExecAndPersistError<A, PEErr, PSErr>
    where
        A: Aggregate,
        PEErr: CqrsError,
        PSErr: CqrsError,
{
    fn from(p: Precondition) -> Self {
        EntityExecAndPersistError::PreconditionFailed(p)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityError<LEErr, LSErr, A, PEErr, PSErr>
where
    A: Aggregate,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    Load(EntityLoadError<LEErr, LSErr>),
    PreconditionFailed(Precondition),
    Exec(HydratedAggregate<A>, A::Error),
    Persist(EntityPersistError<PEErr, PSErr>),
}

impl<LEErr, LSErr, A, PEErr, PSErr> fmt::Display for EntityError<LEErr, LSErr, A, PEErr, PSErr>
where
    A: Aggregate,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityError::Load(e) => Display::fmt(&e, f),
            EntityError::PreconditionFailed(p) =>
                write!(f, "entity error, precondition failed: {}", p),
            EntityError::Exec(_, e) =>
                write!(f, "entity error, command was rejected: {}", e),
            EntityError::Persist(e) => Display::fmt(&e, f),
        }
    }
}

impl<LEErr, LSErr, A, PEErr, PSErr> From<Precondition> for EntityError<LEErr, LSErr, A, PEErr, PSErr>
where
    A: Aggregate,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn from(p: Precondition) -> Self {
        EntityError::PreconditionFailed(p)
    }
}

impl<LEErr, LSErr, A, PEErr, PSErr> From<EntityExecAndPersistError<A, PEErr, PSErr>> for EntityError<LEErr, LSErr, A, PEErr, PSErr>
    where
        A: Aggregate,
        LEErr: CqrsError,
        LSErr: CqrsError,
        PEErr: CqrsError,
        PSErr: CqrsError,
{
    fn from(p: EntityExecAndPersistError<A, PEErr, PSErr>) -> Self {
        match p {
            EntityExecAndPersistError::PreconditionFailed(p) => EntityError::PreconditionFailed(p),
            EntityExecAndPersistError::Exec(agg, err) => EntityError::Exec(agg, err),
            EntityExecAndPersistError::Persist(e) => EntityError::Persist(e),
        }
    }
}
