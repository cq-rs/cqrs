use std::borrow::{Borrow, BorrowMut};
use std::fmt;
use std::marker::PhantomData;
use trivial::NullStore;
use cqrs_core::{Aggregate, AggregateCommand, AggregateId, Events, EventSource, EventSink, ExecuteTarget, SnapshotSource, SnapshotSink, EventNumber, Since, Version, VersionedAggregate, VersionedAggregateView, Precondition, CqrsError};

/// An aggregate that has been loaded from a source, which keeps track of the version of its last snapshot and the current version of the aggregate.
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
    /// The current version of the aggregate.
    pub fn version(&self) -> Version {
        self.version
    }

    /// The version of the snapshot from which the aggregate was loaded.
    pub fn snapshot_version(&self) -> Version {
        self.snapshot_version
    }

    /// The actual aggregate.
    pub fn state(&self) -> &A {
        &self.state
    }

    /// Applies a sequence of events to the internal aggregate.
    pub fn apply_events<I: IntoIterator<Item=A::Event>>(&mut self, events: I) {
        for event in events {
            self.apply(event);
        }
    }

    /// Applies a single event to the aggregate, keeping track of the new aggregate version.
    pub fn apply(&mut self, event: A::Event) {
        self.state.apply(event);
        self.version.incr();
    }

    /// Converts `self` into an [Entity] with the associated identifier.
    pub fn into_entity_with_id<I: AggregateId<Aggregate=A>>(self, id: I) -> Entity<I, A> {
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

/// An identified, specific instance of a hydrated aggregate.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate=A>,
{
    id: I,
    aggregate: HydratedAggregate<A>,
}

impl<I, A> Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate=A>,
{
    /// Creates a new entity from an identifier and an associated hydrated aggregate.
    pub fn new(id: I, aggregate: HydratedAggregate<A>) -> Self {
        Entity {
            id,
            aggregate,
        }
    }

    /// The entity's identifier.
    pub fn id(&self) -> &str {
        self.id.as_ref()
    }

    /// An immutable reference to the underlying aggregate.
    pub fn aggregate(&self) -> &HydratedAggregate<A> {
        &self.aggregate
    }

    /// A mutable reference to the underlying aggregate.
    pub fn aggregate_mut(&mut self) -> &mut HydratedAggregate<A> {
        &mut self.aggregate
    }
}

impl<I, A> From<Entity<I, A>> for HydratedAggregate<A>
where
    A: Aggregate,
    I: AggregateId<Aggregate=A>,
{
    fn from(entity: Entity<I, A>) -> Self {
        entity.aggregate
    }
}

impl<I, A> AsRef<HydratedAggregate<A>> for Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate=A>,
{
    fn as_ref(&self) -> &HydratedAggregate<A> {
        &self.aggregate
    }
}

impl<I, A> AsMut<HydratedAggregate<A>> for Entity<I, A>
    where
        A: Aggregate,
        I: AggregateId<Aggregate=A>,
{
    fn as_mut(&mut self) -> &mut HydratedAggregate<A> {
        &mut self.aggregate
    }
}

impl<I, A> Borrow<HydratedAggregate<A>> for Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate=A>,
{
    fn borrow(&self) -> &HydratedAggregate<A> {
        &self.aggregate
    }
}

impl<I, A> Borrow<A> for Entity<I, A>
    where
        A: Aggregate,
        I: AggregateId<Aggregate=A>,
{
    fn borrow(&self) -> &A {
        self.aggregate.borrow()
    }
}

impl<I, A> BorrowMut<HydratedAggregate<A>> for Entity<I, A>
    where
        A: Aggregate,
        I: AggregateId<Aggregate=A>,
{
    fn borrow_mut(&mut self) -> &mut HydratedAggregate<A> {
        &mut self.aggregate
    }
}

/// A source for loading an [Entity].
pub trait EntitySource<A>: EventSource<A> + SnapshotSource<A>
where
    A: Aggregate,
{
    /// Loads an identified [Entity] from the latest known snapshot.
    ///
    /// If the `SnapshotSource` returns an error, it is passed along. If the source does not have a snapshot
    /// for the requested entity, returns `Ok(None)`.
    fn load_from_snapshot<I>(
        &self,
        id: &I,
    ) -> EntityLoadSnapshotResult<A, Self>
    where
        I: AggregateId<Aggregate=A>,
    {
        let entity =
            if let Some(snapshot) = self.get_snapshot(id)? {
                Some(HydratedAggregate {
                    version: snapshot.version,
                    snapshot_version: snapshot.version,
                    state: snapshot.payload,
                })
            } else {
                None
            };

        Ok(entity)
    }

    /// Refreshes an existing hydrated aggregate with the given id.
    ///
    /// Errors may occur while loading the events.
    fn refresh<I>(
        &self,
        id: &I,
        aggregate: &mut HydratedAggregate<A>,
    ) -> Result<(), <Self as EventSource<A>>::Error>
    where
        I: AggregateId<Aggregate=A>
    {
        let seq_events = self.read_events(id, aggregate.version.into(), None)?;

        if let Some(seq_events) = seq_events {
            for seq_event in seq_events {
                let seq_event = seq_event?;

                aggregate.apply(seq_event.event);

                debug_assert_eq!(Version::Number(seq_event.sequence), aggregate.version);
            }
        }

        Ok(())
    }

    /// Loads an entity from the most recent snapshot of its aggregate, then applies any newer events that have not yet been
    /// applied.
    ///
    /// Errors may occur while loading the snapshot or the events. If no snapshot or events can be found
    /// for the entity, returns `Ok(None)`
    fn rehydrate<I>(
        &self,
        id: &I,
    ) -> EntityRefreshResult<A, Self>
    where
        I: AggregateId<Aggregate=A>
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

/// The result of loading an [Entity] from a snapshot.
pub type EntityLoadSnapshotResult<A, L> = Result<Option<HydratedAggregate<A>>, <L as SnapshotSource<A>>::Error>;

/// The result of refreshing an [Entity].
pub type EntityRefreshResult<A, L> = Result<Option<HydratedAggregate<A>>, EntityLoadError<<L as EventSource<A>>::Error, <L as SnapshotSource<A>>::Error>>;

/// The result of persisting an [Entity].
pub type EntityPersistResult<A, M, L> = Result<(), EntityPersistError<<L as EventSink<A, M>>::Error, <L as SnapshotSink<A>>::Error>>;

/// The result of executing a command against an [Entity], after attempting to persist any
/// new events and possibly updating the snapshot.
pub type EntityExecAndPersistResult<C, M, L> =
    Result<
        HydratedAggregate<ExecuteTarget<C>>,
        EntityExecAndPersistError<
            C,
            <L as EventSink<ExecuteTarget<C>, M>>::Error,
            <L as SnapshotSink<ExecuteTarget<C>>>::Error,
        >
    >;

/// The result of loading an [Entity], then executing a command and attempting to persist
/// any new events and possibly updating the snapshot.
pub type EntityResult<C, M, L> =
    Result<
        HydratedAggregate<ExecuteTarget<C>>,
        EntityError<
            <L as EventSource<ExecuteTarget<C>>>::Error,
            <L as SnapshotSource<ExecuteTarget<C>>>::Error,
            C,
            <L as EventSink<ExecuteTarget<C>, M>>::Error,
            <L as SnapshotSink<ExecuteTarget<C>>>::Error,
        >,
    >;

/// The result of trying to load an [Entity], which may not exists, then executing a command and
/// attempting to persist any new events and possibly updating the snapshot
pub type EntityOptionResult<C, M, L> =
    Result<
        Option<HydratedAggregate<ExecuteTarget<C>>>,
        EntityError<
            <L as EventSource<ExecuteTarget<C>>>::Error,
            <L as SnapshotSource<ExecuteTarget<C>>>::Error,
            C,
            <L as EventSink<ExecuteTarget<C>, M>>::Error,
            <L as SnapshotSink<ExecuteTarget<C>>>::Error,
        >,
    >;

impl<A, T> EntitySource<A> for T
where
    A: Aggregate,
    T: EventSource<A> + SnapshotSource<A>
{}

/// A sink for persisting an [Entity].
pub trait EntitySink<A, M>: EventSink<A, M> + SnapshotSink<A>
where
    A: Aggregate,
{
    /// Attempts persist a sequence of events to an identified aggregate and then apply those
    /// events to the mutable aggregate. Then attempts to persist a snapshot of the aggregate
    /// if there are at least `max_events_before_snapshot` events that have not been incorporated
    /// into the latest snapshot. Returns the resulting aggregate if persistence was successful.
    ///
    /// Errors may occur while persisting the events or the snapshot or the events. If there result indicates
    /// an error while persisting the snapshot, then any events have already been safely persisted.
    fn apply_events_and_persist<I, E>(
        &self,
        id: &I,
        aggregate: &mut HydratedAggregate<A>,
        events: E,
        expected_version: Version,
        metadata: M,
        max_events_before_snapshot: u64,
    ) -> EntityPersistResult<A, M, Self>
    where
        I: AggregateId<Aggregate=A>,
        E: Events<A::Event>,
    {
        self.append_events(id, events.as_ref(), Some(Precondition::ExpectedVersion(expected_version)), metadata).map_err(EntityPersistError::EventSink)?;

        for event in events {
            aggregate.apply(event);
        }

        if aggregate.version - aggregate.snapshot_version >= max_events_before_snapshot as i64 {
            let view = VersionedAggregateView {
                payload: &aggregate.state,
                version: aggregate.version,
            };
            self.persist_snapshot(id, view).map_err(EntityPersistError::SnapshotSink)?
        }

        Ok(())
    }

    /// Executes a command against an aggregate, using the default if `None`. If successful, then persists any resulting
    /// events (and possibly updating the snapshot, see [EventSink.apply_events_and_persist]). Returns the resulting
    /// aggregate if all persistence operations were successful.
    fn exec_and_persist<I, C>(
        &self,
        id: &I,
        aggregate: Option<HydratedAggregate<A>>,
        command: C,
        precondition: Option<Precondition>,
        metadata: M,
        max_events_before_snapshot: u64,
    ) -> EntityExecAndPersistResult<C, M, Self>
    where
        I: AggregateId<Aggregate=A>,
        C: AggregateCommand<Aggregate=A>,
        C::Events: Events<A::Event>,
    {
        if let Some(precondition) = precondition {
            let initial_version = aggregate.as_ref().map(|agg| agg.version);
            precondition.verify(initial_version)?;
        }

        let mut aggregate = aggregate.unwrap_or_default();

        let expected_version = aggregate.version;

        match aggregate.state.execute(command) {
            Ok(events) => {
                self.apply_events_and_persist(
                    id,
                    &mut aggregate,
                    events,
                    expected_version,
                    metadata,
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

impl<A, M, T> EntitySink<A, M> for T
where
    A: Aggregate,
    T: EventSink<A, M> + SnapshotSink<A>
{}

/// A generalized entity store that can perform operations on its entities.
pub trait EntityStore<A, M>: EntitySource<A> + EntitySink<A, M>
where
    A: Aggregate,
{
    /// Attempts to load an aggregate, using the default instance if the aggregate does not yet exist, executes a
    /// command and persists any new events, possibly peristing a new snapshot if necessary.
    fn load_or_default_exec_and_persist<I, C>(
        &self,
        id: &I,
        command: C,
        precondition: Option<Precondition>,
        metadata: M,
        max_events_before_snapshot: u64
    ) -> EntityResult<C, M, Self>
    where
        I: AggregateId<Aggregate=A>,
        C: AggregateCommand<Aggregate=A>,
        C::Events: Events<A::Event>,
    {
        let aggregate = self.rehydrate(id).map_err(EntityError::Load)?;
        let aggregate = self.exec_and_persist(
            id,
            aggregate,
            command,
            precondition,
            metadata,
            max_events_before_snapshot,
        )?;

        Ok(aggregate)
    }

    /// Loads an aggregate, executes a command and persists any new events, possibly persisting
    /// a new snapshot if necessary.
    ///
    /// If the aggregate does not exist, returns `Ok(None)`.
    fn load_exec_and_persist<I, C>(
        &self,
        id: &I,
        command: C,
        precondition: Option<Precondition>,
        metadata: M,
        max_events_before_snapshot: u64
    ) -> EntityOptionResult<C, M, Self>
    where
        I: AggregateId<Aggregate=A>,
        C: AggregateCommand<Aggregate=A>,
        C::Events: Events<A::Event>,
    {
        if let Some(aggregate) = self.rehydrate(id).map_err(EntityError::Load)? {
            let aggregate = self.exec_and_persist(
                id,
                Some(aggregate),
                command,
                precondition,
                metadata,
                max_events_before_snapshot,
            )?;

            Ok(Some(aggregate))
        } else {
            Ok(None)
        }
    }
}

impl<A, M, T> EntityStore<A, M> for T
where
    A: Aggregate,
    T: EntitySource<A> + EntitySink<A, M>
{}

/// Combines an `EventSource` and a `SnapshotSource` of different types by reference
/// so that they can be used jointly as an [EntitySource].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeEntitySource<'e, 's, A, ES, SS>
where
    A: Aggregate,
    ES: EventSource<A> + 'e,
    SS: SnapshotSource<A> + 's,
{
    event_source: &'e ES,
    snapshot_source: &'s SS,
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

impl<'e, 's, A, ES, SS> CompositeEntitySource<'e, 's, A, ES, SS>
where
    A: Aggregate,
    ES: EventSource<A> + 'e,
    SS: SnapshotSource<A> + 's,
{
    /// Attaches a specific event source.
    pub fn with_event_source<'new_e, NewES>(self, event_source: &'new_e NewES) -> CompositeEntitySource<'new_e, 's, A, NewES, SS>
    where
        NewES: EventSource<A> + 'new_e,
    {
        CompositeEntitySource {
            event_source,
            snapshot_source: self.snapshot_source,
            _phantom: PhantomData,
        }
    }

    /// Attaches a specific snapshot source.
    pub fn with_snapshot_source<'new_s, NewSS>(self, snapshot_source: &'new_s NewSS) -> CompositeEntitySource<'e, 'new_s, A, ES, NewSS>
    where
        NewSS: SnapshotSource<A> + 'new_s,
    {
        CompositeEntitySource {
            event_source: self.event_source,
            snapshot_source,
            _phantom: PhantomData,
        }
    }
}

impl<'e, 's, A, ES, SS> EventSource<A> for CompositeEntitySource<'e, 's, A, ES, SS>
where
    A: Aggregate,
    ES: EventSource<A> + 'e,
    SS: SnapshotSource<A> + 's,
{
    type Events = ES::Events;
    type Error = ES::Error;

    fn read_events<I>(&self, id: &I, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        self.event_source.read_events(id, since, max_count)
    }
}

impl<'e, 's, A, ES, SS> SnapshotSource<A> for CompositeEntitySource<'e, 's, A, ES, SS>
where
    A: Aggregate,
    ES: EventSource<A> + 'e,
    SS: SnapshotSource<A> + 's,
{
    type Error = SS::Error;

    fn get_snapshot<I>(&self, id: &I) -> Result<Option<VersionedAggregate<A>>, <Self as SnapshotSource<A>>::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        self.snapshot_source.get_snapshot(id)
    }
}

/// Combines an `EventSink` and a `SnapshotSink` of different types by reference
/// so that they can be used jointly as an [EntitySink].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeEntitySink<'e, 's, A, M, ES, SS>
where
    A: Aggregate,
    ES: EventSink<A, M> + 'e,
    SS: SnapshotSink<A> + 's,
{
    event_sink: &'e ES,
    snapshot_sink: &'s SS,
    _phantom: PhantomData<(A, M)>,
}

impl<A, M> Default for CompositeEntitySink<'static, 'static, A, M, NullStore, NullStore>
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

impl<'e, 's, A, M, ES, SS> CompositeEntitySink<'e, 's, A, M, ES, SS>
where
    A: Aggregate,
    ES: EventSink<A, M> + 'e,
    SS: SnapshotSink<A> + 's,
{
    /// Attaches a specific event sink.
    pub fn with_event_sink<'new_e, NewES>(self, event_sink: &'new_e NewES) -> CompositeEntitySink<'new_e, 's, A, M, NewES, SS>
    where
        NewES: EventSink<A, M> + 'new_e,
    {
        CompositeEntitySink {
            event_sink,
            snapshot_sink: self.snapshot_sink,
            _phantom: PhantomData,
        }
    }

    /// Attaches a specific snapshot sink.
    pub fn with_snapshot_sink<'new_s, NewSS>(self, snapshot_sink: &'new_s NewSS) -> CompositeEntitySink<'e, 'new_s, A, M, ES, NewSS>
    where
        NewSS: SnapshotSink<A> + 'new_s,
    {
        CompositeEntitySink {
            event_sink: self.event_sink,
            snapshot_sink,
            _phantom: PhantomData,
        }
    }
}

impl<'e, 's, A, M, ES, SS> EventSink<A, M> for CompositeEntitySink<'e, 's, A, M, ES, SS>
where
    A: Aggregate,
    ES: EventSink<A, M> + 'e,
    SS: SnapshotSink<A> + 's,
{
    type Error = ES::Error;

    fn append_events<I>(&self, id: &I, events: &[A::Event], precondition: Option<Precondition>, metadata: M) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        self.event_sink.append_events(id, events, precondition, metadata)
    }
}

impl<'e, 's, A, M, ES, SS> SnapshotSink<A> for CompositeEntitySink<'e, 's, A, M, ES, SS>
where
    A: Aggregate,
    ES: EventSink<A, M> + 'e,
    SS: SnapshotSink<A> + 's,
{
    type Error = SS::Error;

    fn persist_snapshot<I>(&self, id: &I, aggregate: VersionedAggregateView<A>) -> Result<(), Self::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        self.snapshot_sink.persist_snapshot(id, aggregate)
    }
}

/// Combines an [EntitySource] and an [EntitySink] into a single type so that they
/// can be jointly used as an [EntityStore].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeEntityStore<A, M, ES, SS>
where
    A: Aggregate,
    ES: EntitySource<A>,
    SS: EntitySink<A, M>,
{
    entity_source: ES,
    entity_sink: SS,
    _phantom: PhantomData<(A, M)>,
}

impl<A, M> Default for CompositeEntityStore<A, M, NullStore, NullStore>
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

impl<A, M, ES, SS> CompositeEntityStore<A, M, ES, SS>
where
    A: Aggregate,
    ES: EntitySource<A>,
    SS: EntitySink<A, M>,
{
    /// Attaches a specific entity source.
    pub fn with_entity_source<NewES>(self, entity_source: NewES) -> CompositeEntityStore<A, M, NewES, SS>
    where
        NewES: EntitySource<A>,
    {
        CompositeEntityStore {
            entity_source,
            entity_sink: self.entity_sink,
            _phantom: PhantomData,
        }
    }

    /// Attaches a specific entity sink.
    pub fn with_entity_sink<NewSS>(self, entity_sink: NewSS) -> CompositeEntityStore<A, M, ES, NewSS>
    where
        NewSS: EntitySink<A, M>,
    {
        CompositeEntityStore {
            entity_source: self.entity_source,
            entity_sink,
            _phantom: PhantomData,
        }
    }
}

impl<A, M, ES, SS> EventSource<A> for CompositeEntityStore<A, M, ES, SS>
where
    A: Aggregate,
    ES: EntitySource<A>,
    SS: EntitySink<A, M>,
{
    type Events = <ES as EventSource<A>>::Events;
    type Error = <ES as EventSource<A>>::Error;

    fn read_events<I>(&self, id: &I, since: Since, max_count: Option<u64>) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        self.entity_source.read_events(id, since, max_count)
    }
}

impl<A, M, ES, SS> SnapshotSource<A> for CompositeEntityStore<A, M, ES, SS>
where
    A: Aggregate,
    ES: EntitySource<A>,
    SS: EntitySink<A, M>,
{
    type Error = <ES as SnapshotSource<A>>::Error;

    fn get_snapshot<I>(&self, id: &I) -> Result<Option<VersionedAggregate<A>>, <Self as SnapshotSource<A>>::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        self.entity_source.get_snapshot(id)
    }
}

impl<A, M, ES, SS> EventSink<A, M> for CompositeEntityStore<A, M, ES, SS>
where
    A: Aggregate,
    ES: EntitySource<A>,
    SS: EntitySink<A, M>,
{
    type Error = <SS as EventSink<A, M>>::Error;

    fn append_events<I>(&self, id: &I, events: &[A::Event], precondition: Option<Precondition>, metadata: M) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        self.entity_sink.append_events(id, events, precondition, metadata)
    }
}

impl<A, M, ES, SS> SnapshotSink<A> for CompositeEntityStore<A, M, ES, SS>
where
    A: Aggregate,
    ES: EntitySource<A>,
    SS: EntitySink<A, M>,
{
    type Error = <SS as SnapshotSink<A>>::Error;

    fn persist_snapshot<I>(&self, id: &I, aggregate: VersionedAggregateView<A>) -> Result<(), Self::Error>
    where
        I: AggregateId<Aggregate=A>,
    {
        self.entity_sink.persist_snapshot(id, aggregate)
    }
}

/// An error produced when there is an error while loading an [Entity].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityLoadError<EErr, SErr>
where
    EErr: CqrsError,
    SErr: CqrsError,
{
    /// An error occurred while attempting to load events from the event source.
    EventSource(EErr),

    /// An error occurred while attempting to load the snapshot from the snapshot source.
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

/// An error produced when there is an error while persisting an [Entity].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityPersistError<EErr, SErr>
where
    EErr: CqrsError,
    SErr: CqrsError,
{
    /// An error occurred while persisting the events to the event sink.
    EventSink(EErr),

    /// An error occurred while persisting the snapshot to the snapshot sink.
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

/// An error produced when there is an error while attempting to execute a command against an aggregate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityExecAndPersistError<C, PEErr, PSErr>
where
    C: AggregateCommand,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    /// The command could not be applied because the aggregate was not in the expected state.
    PreconditionFailed(Precondition),

    /// The command reported an error while executing against the aggregate.
    Exec(HydratedAggregate<C::Aggregate>, C::Error),

    /// An error occurred while persisting the entity.
    Persist(EntityPersistError<PEErr, PSErr>),
}

impl<C, PEErr, PSErr> fmt::Display for EntityExecAndPersistError<C, PEErr, PSErr>
where
    C: AggregateCommand,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityExecAndPersistError::PreconditionFailed(p) =>
                write!(f, "entity exec error, precondition failed: {}", p),
            EntityExecAndPersistError::Exec(_, e) =>
                write!(f, "entity exec error, command was rejected: {}", e),
            EntityExecAndPersistError::Persist(e) => fmt::Display::fmt(&e, f),
        }
    }
}

impl<C, PEErr, PSErr> From<Precondition> for EntityExecAndPersistError<C, PEErr, PSErr>
where
    C: AggregateCommand,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn from(p: Precondition) -> Self {
        EntityExecAndPersistError::PreconditionFailed(p)
    }
}

/// An error produced when there is an error attempting to load an aggregate, execute a command, and perist the results.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityError<LEErr, LSErr, C, PEErr, PSErr>
where
    C: AggregateCommand,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    /// An error occurred while loading the entity.
    Load(EntityLoadError<LEErr, LSErr>),

    /// The command could not be applied because the aggregate was not in the expected state.
    PreconditionFailed(Precondition),

    /// The command reported an error while executing against the aggregate.
    Exec(HydratedAggregate<C::Aggregate>, C::Error),

    /// An error occurred while persisting the entity.
    Persist(EntityPersistError<PEErr, PSErr>),
}

impl<LEErr, LSErr, C, PEErr, PSErr> fmt::Display for EntityError<LEErr, LSErr, C, PEErr, PSErr>
where
    C: AggregateCommand,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityError::Load(e) => fmt::Display::fmt(&e, f),
            EntityError::PreconditionFailed(p) =>
                write!(f, "entity error, precondition failed: {}", p),
            EntityError::Exec(_, e) =>
                write!(f, "entity error, command was rejected: {}", e),
            EntityError::Persist(e) => fmt::Display::fmt(&e, f),
        }
    }
}

impl<LEErr, LSErr, C, PEErr, PSErr> From<Precondition> for EntityError<LEErr, LSErr, C, PEErr, PSErr>
where
    C: AggregateCommand,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn from(p: Precondition) -> Self {
        EntityError::PreconditionFailed(p)
    }
}

impl<LEErr, LSErr, C, PEErr, PSErr> From<EntityExecAndPersistError<C, PEErr, PSErr>> for EntityError<LEErr, LSErr, C, PEErr, PSErr>
where
    C: AggregateCommand,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn from(p: EntityExecAndPersistError<C, PEErr, PSErr>) -> Self {
        match p {
            EntityExecAndPersistError::PreconditionFailed(p) => EntityError::PreconditionFailed(p),
            EntityExecAndPersistError::Exec(agg, err) => EntityError::Exec(agg, err),
            EntityExecAndPersistError::Persist(e) => EntityError::Persist(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory::StateStore;
    use testing::*;

    #[test]
    fn can_construct_composite_entity_source() {
        let null = NullStore;
        let memory = StateStore::<TestAggregate>::default();
        let _source =
            CompositeEntitySource::default()
                .with_event_source(&null)
                .with_snapshot_source(&memory);
    }

    #[test]
    fn can_construct_composite_entity_sink() {
        let null = NullStore;
        let memory = StateStore::<TestAggregate>::default();
        let _sink: CompositeEntitySink<TestAggregate, TestMetadata, NullStore, StateStore<TestAggregate>> =
            CompositeEntitySink::default()
                .with_event_sink(&null)
                .with_snapshot_sink(&memory);
    }

    #[test]
    fn can_construct_composite_entity_store() {
        let null = NullStore;
        let memory = StateStore::<TestAggregate>::default();
        let source =
            CompositeEntitySource::default()
                .with_event_source(&null)
                .with_snapshot_source(&memory);
        let sink: CompositeEntitySink<TestAggregate, TestMetadata, NullStore, StateStore<TestAggregate>>  =
            CompositeEntitySink::default()
                .with_event_sink(&null)
                .with_snapshot_sink(&memory);
        let _store =
            CompositeEntityStore::default()
                .with_entity_source(source)
                .with_entity_sink(sink);
    }
}
