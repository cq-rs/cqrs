use crate::trivial::{NullEventStore, NullSnapshotStore};
use cqrs_core::{
    Aggregate, AggregateCommand, AggregateEvent, AggregateId, CqrsError, EventNumber, EventSink,
    EventSource, Events, Precondition, ProducedEvent, Since, SnapshotSink, SnapshotSource, Version,
    VersionedAggregate,
};
use std::{
    borrow::{Borrow, BorrowMut},
    fmt,
    marker::PhantomData,
};

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

    /// Updates the snapshot version. Generally used to indicate that a snapshot was taken.
    pub fn set_snapshot_version(&mut self, new_snapshot_version: Version) {
        self.snapshot_version = new_snapshot_version;
    }

    /// The actual aggregate.
    pub fn state(&self) -> &A {
        &self.state
    }

    /// Applies a sequence of events to the internal aggregate.
    pub fn apply_events<E: AggregateEvent<A>, I: IntoIterator<Item = E>>(&mut self, events: I) {
        for event in events {
            self.apply(event);
        }
    }

    /// Applies a single event to the aggregate, keeping track of the new aggregate version.
    pub fn apply<E: AggregateEvent<A>>(&mut self, event: E) {
        self.state.apply(event);
        self.version.incr();
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
    I: AggregateId<Aggregate = A>,
{
    id: I,
    aggregate: HydratedAggregate<A>,
}

impl<I, A> Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate = A>,
{
    /// Creates a new entity from an identifier and an associated hydrated aggregate.
    pub fn new(id: I, aggregate: HydratedAggregate<A>) -> Self {
        Entity { id, aggregate }
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
    I: AggregateId<Aggregate = A>,
{
    fn from(entity: Entity<I, A>) -> Self {
        entity.aggregate
    }
}

impl<I, A> AsRef<HydratedAggregate<A>> for Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate = A>,
{
    fn as_ref(&self) -> &HydratedAggregate<A> {
        &self.aggregate
    }
}

impl<I, A> AsMut<HydratedAggregate<A>> for Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate = A>,
{
    fn as_mut(&mut self) -> &mut HydratedAggregate<A> {
        &mut self.aggregate
    }
}

impl<I, A> Borrow<HydratedAggregate<A>> for Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate = A>,
{
    fn borrow(&self) -> &HydratedAggregate<A> {
        &self.aggregate
    }
}

impl<I, A> Borrow<A> for Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate = A>,
{
    fn borrow(&self) -> &A {
        self.aggregate.borrow()
    }
}

impl<I, A> BorrowMut<HydratedAggregate<A>> for Entity<I, A>
where
    A: Aggregate,
    I: AggregateId<Aggregate = A>,
{
    fn borrow_mut(&mut self) -> &mut HydratedAggregate<A> {
        &mut self.aggregate
    }
}

/// A source for loading an [Entity].
pub trait EntitySource<A, E>: EventSource<A, E> + SnapshotSource<A>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    /// Loads an identified [Entity] from the latest known snapshot.
    ///
    /// If the `SnapshotSource` returns an error, it is passed along. If the source does not have a snapshot
    /// for the requested entity, returns `Ok(None)`.
    fn load_from_snapshot<I>(&self, id: &I) -> EntityLoadSnapshotResult<A, Self>
    where
        I: AggregateId<Aggregate = A>,
    {
        let entity = if let Some(snapshot) = self.get_snapshot(id)? {
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
    ) -> Result<(), <Self as EventSource<A, E>>::Error>
    where
        I: AggregateId<Aggregate = A>,
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
    fn rehydrate<I>(&self, id: &I) -> EntityRefreshResult<A, E, Self>
    where
        I: AggregateId<Aggregate = A>,
    {
        let aggregate = self
            .load_from_snapshot(id)
            .map_err(EntityLoadError::SnapshotSource)?;

        let missing = aggregate.is_none();

        let mut aggregate = aggregate.unwrap_or_default();

        self.refresh(id, &mut aggregate)
            .map_err(EntityLoadError::EventSource)?;

        if missing && aggregate.version == Version::Initial {
            Ok(None)
        } else {
            Ok(Some(aggregate))
        }
    }
}

/// The result of loading an [Entity] from a snapshot.
pub type EntityLoadSnapshotResult<A, L> =
    Result<Option<HydratedAggregate<A>>, <L as SnapshotSource<A>>::Error>;

/// The result of refreshing an [Entity].
pub type EntityRefreshResult<A, E, L> = Result<
    Option<HydratedAggregate<A>>,
    EntityLoadError<<L as EventSource<A, E>>::Error, <L as SnapshotSource<A>>::Error>,
>;

/// The result of persisting an [Entity].
pub type EntityPersistResult<A, E, M, L> =
    Result<(), EntityPersistError<<L as EventSink<A, E, M>>::Error, <L as SnapshotSink<A>>::Error>>;

/// The result of executing a command against an [Entity], after attempting to persist any
/// new events and possibly updating the snapshot.
pub type EntityExecAndPersistResult<A, C, M, L> = Result<
    HydratedAggregate<A>,
    EntityExecAndPersistError<
        A,
        C,
        <L as EventSink<A, ProducedEvent<A, C>, M>>::Error,
        <L as SnapshotSink<A>>::Error,
    >,
>;

/// The result of loading an [Entity], then executing a command and attempting to persist
/// any new events and possibly updating the snapshot.
pub type EntityResult<A, C, M, L> = Result<
    HydratedAggregate<A>,
    EntityError<
        <L as EventSource<A, ProducedEvent<A, C>>>::Error,
        <L as SnapshotSource<A>>::Error,
        A,
        C,
        <L as EventSink<A, ProducedEvent<A, C>, M>>::Error,
        <L as SnapshotSink<A>>::Error,
    >,
>;

/// The result of trying to load an [Entity], which may not exists, then executing a command and
/// attempting to persist any new events and possibly updating the snapshot
pub type EntityOptionResult<A, C, M, L> = Result<
    Option<HydratedAggregate<A>>,
    EntityError<
        <L as EventSource<A, ProducedEvent<A, C>>>::Error,
        <L as SnapshotSource<A>>::Error,
        A,
        C,
        <L as EventSink<A, ProducedEvent<A, C>, M>>::Error,
        <L as SnapshotSink<A>>::Error,
    >,
>;

impl<A, E, T> EntitySource<A, E> for T
where
    A: Aggregate,
    E: AggregateEvent<A>,
    T: EventSource<A, E> + SnapshotSource<A>,
{
}

/// A sink for persisting an [Entity].
pub trait EntitySink<A, E, M>: EventSink<A, E, M> + SnapshotSink<A>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    /// Attempts persist a sequence of events to an identified aggregate and then apply those
    /// events to the mutable aggregate. Then attempts to persist a snapshot of the aggregate
    /// if there are at least `max_events_before_snapshot` events that have not been incorporated
    /// into the latest snapshot. Returns the resulting aggregate if persistence was successful.
    ///
    /// Errors may occur while persisting the events or the snapshot or the events. If there result indicates
    /// an error while persisting the snapshot, then any events have already been safely persisted.
    fn apply_events_and_persist<I, Es>(
        &self,
        id: &I,
        aggregate: &mut HydratedAggregate<A>,
        events: Es,
        expected_version: Version,
        metadata: M,
    ) -> EntityPersistResult<A, E, M, Self>
    where
        I: AggregateId<Aggregate = A>,
        Es: Events<E>,
    {
        self.append_events(
            id,
            events.as_ref(),
            Some(Precondition::ExpectedVersion(expected_version)),
            metadata,
        )
        .map_err(EntityPersistError::EventSink)?;

        for event in events {
            aggregate.apply(event);
        }

        let new_snapshot_version = self
            .persist_snapshot(
                id,
                aggregate.state(),
                aggregate.version(),
                aggregate.snapshot_version(),
            )
            .map_err(EntityPersistError::SnapshotSink)?;
        aggregate.set_snapshot_version(new_snapshot_version);

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
    ) -> EntityExecAndPersistResult<A, C, M, Self>
    where
        I: AggregateId<Aggregate = A>,
        C: AggregateCommand<A, Event = E>,
        C::Events: Events<E>,
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
                )
                .map_err(EntityExecAndPersistError::Persist)?;
            },
            Err(e) => {
                return Err(EntityExecAndPersistError::Exec(aggregate, e));
            },
        }

        Ok(aggregate)
    }
}

impl<A, E, M, T> EntitySink<A, E, M> for T
where
    A: Aggregate,
    E: AggregateEvent<A>,
    T: EventSink<A, E, M> + SnapshotSink<A>,
{
}

/// A generalized entity store that can perform operations on its entities.
pub trait EntityStore<A, E, M>: EntitySource<A, E> + EntitySink<A, E, M>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    /// Attempts to load an aggregate, using the default instance if the aggregate does not yet exist, executes a
    /// command and persists any new events, possibly peristing a new snapshot if necessary.
    fn load_or_default_exec_and_persist<I, C>(
        &self,
        id: &I,
        command: C,
        precondition: Option<Precondition>,
        metadata: M,
    ) -> EntityResult<A, C, M, Self>
    where
        I: AggregateId<Aggregate = A>,
        C: AggregateCommand<A, Event = E>,
        C::Events: Events<E>,
    {
        let aggregate = self.rehydrate(id).map_err(EntityError::Load)?;
        let aggregate = self.exec_and_persist(id, aggregate, command, precondition, metadata)?;

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
    ) -> EntityOptionResult<A, C, M, Self>
    where
        I: AggregateId<Aggregate = A>,
        C: AggregateCommand<A, Event = E>,
        C::Events: Events<E>,
    {
        if let Some(aggregate) = self.rehydrate(id).map_err(EntityError::Load)? {
            let aggregate =
                self.exec_and_persist(id, Some(aggregate), command, precondition, metadata)?;

            Ok(Some(aggregate))
        } else {
            Ok(None)
        }
    }
}

impl<A, E, M, T> EntityStore<A, E, M> for T
where
    A: Aggregate,
    E: AggregateEvent<A>,
    T: EntitySource<A, E> + EntitySink<A, E, M>,
{
}

/// Combines an `EventSource` and a `SnapshotSource` of different types by reference
/// so that they can be used jointly as an [EntitySource].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeEntitySource<'e, 's, A, E, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EventSource<A, E> + 'e,
    SS: SnapshotSource<A> + 's,
{
    event_source: &'e ES,
    snapshot_source: &'s SS,
    _phantom: PhantomData<&'e (A, E)>,
}

impl<A, E> Default
    for CompositeEntitySource<'static, 'static, A, E, NullEventStore<A, E>, NullSnapshotStore<A>>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    fn default() -> Self {
        CompositeEntitySource {
            event_source: &NullEventStore::DEFAULT,
            snapshot_source: &NullSnapshotStore::DEFAULT,
            _phantom: PhantomData,
        }
    }
}

impl<'e, 's, A, E, ES, SS> CompositeEntitySource<'e, 's, A, E, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EventSource<A, E> + 'e,
    SS: SnapshotSource<A> + 's,
{
    /// Attaches a specific event source.
    pub fn with_event_source<'new_e, NewES>(
        self,
        event_source: &'new_e NewES,
    ) -> CompositeEntitySource<'new_e, 's, A, E, NewES, SS>
    where
        NewES: EventSource<A, E> + 'new_e,
    {
        CompositeEntitySource {
            event_source,
            snapshot_source: self.snapshot_source,
            _phantom: PhantomData,
        }
    }

    /// Attaches a specific snapshot source.
    pub fn with_snapshot_source<'new_s, NewSS>(
        self,
        snapshot_source: &'new_s NewSS,
    ) -> CompositeEntitySource<'e, 'new_s, A, E, ES, NewSS>
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

impl<'e, 's, A, E, ES, SS> EventSource<A, E> for CompositeEntitySource<'e, 's, A, E, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EventSource<A, E> + 'e,
    SS: SnapshotSource<A> + 's,
{
    type Error = ES::Error;
    type Events = ES::Events;

    fn read_events<I>(
        &self,
        id: &I,
        since: Since,
        max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        self.event_source.read_events(id, since, max_count)
    }
}

impl<'e, 's, A, E, ES, SS> SnapshotSource<A> for CompositeEntitySource<'e, 's, A, E, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EventSource<A, E> + 'e,
    SS: SnapshotSource<A> + 's,
{
    type Error = SS::Error;

    fn get_snapshot<I>(
        &self,
        id: &I,
    ) -> Result<Option<VersionedAggregate<A>>, <Self as SnapshotSource<A>>::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        self.snapshot_source.get_snapshot(id)
    }
}

/// Combines an `EventSink` and a `SnapshotSink` of different types by reference
/// so that they can be used jointly as an [EntitySink].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeEntitySink<'e, 's, A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EventSink<A, E, M> + 'e,
    SS: SnapshotSink<A> + 's,
{
    event_sink: &'e ES,
    snapshot_sink: &'s SS,
    _phantom: PhantomData<&'e (A, E, M)>,
}

impl<A, E, M> Default
    for CompositeEntitySink<'static, 'static, A, E, M, NullEventStore<A, E>, NullSnapshotStore<A>>
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    fn default() -> Self {
        CompositeEntitySink {
            event_sink: &NullEventStore::DEFAULT,
            snapshot_sink: &NullSnapshotStore::DEFAULT,
            _phantom: PhantomData,
        }
    }
}

impl<'e, 's, A, E, M, ES, SS> CompositeEntitySink<'e, 's, A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EventSink<A, E, M> + 'e,
    SS: SnapshotSink<A> + 's,
{
    /// Attaches a specific event sink.
    pub fn with_event_sink<'new_e, NewES>(
        self,
        event_sink: &'new_e NewES,
    ) -> CompositeEntitySink<'new_e, 's, A, E, M, NewES, SS>
    where
        NewES: EventSink<A, E, M> + 'new_e,
    {
        CompositeEntitySink {
            event_sink,
            snapshot_sink: self.snapshot_sink,
            _phantom: PhantomData,
        }
    }

    /// Attaches a specific snapshot sink.
    pub fn with_snapshot_sink<'new_s, NewSS>(
        self,
        snapshot_sink: &'new_s NewSS,
    ) -> CompositeEntitySink<'e, 'new_s, A, E, M, ES, NewSS>
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

impl<'e, 's, A, E, M, ES, SS> EventSink<A, E, M> for CompositeEntitySink<'e, 's, A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EventSink<A, E, M> + 'e,
    SS: SnapshotSink<A> + 's,
{
    type Error = ES::Error;

    fn append_events<I>(
        &self,
        id: &I,
        events: &[E],
        precondition: Option<Precondition>,
        metadata: M,
    ) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        self.event_sink
            .append_events(id, events, precondition, metadata)
    }
}

impl<'e, 's, A, E, M, ES, SS> SnapshotSink<A> for CompositeEntitySink<'e, 's, A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EventSink<A, E, M> + 'e,
    SS: SnapshotSink<A> + 's,
{
    type Error = SS::Error;

    fn persist_snapshot<I>(
        &self,
        id: &I,
        aggregate: &A,
        version: Version,
        last_snapshot_version: Version,
    ) -> Result<Version, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        self.snapshot_sink
            .persist_snapshot(id, aggregate, version, last_snapshot_version)
    }
}

/// Combines an [EntitySource] and an [EntitySink] into a single type so that they
/// can be jointly used as an [EntityStore].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeEntityStore<A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EntitySource<A, E>,
    SS: EntitySink<A, E, M>,
{
    entity_source: ES,
    entity_sink: SS,
    _phantom: PhantomData<*const (A, E, M)>,
}

impl<A, E, M> Default
    for CompositeEntityStore<
        A,
        E,
        M,
        CompositeEntitySource<'static, 'static, A, E, NullEventStore<A, E>, NullSnapshotStore<A>>,
        CompositeEntitySink<'static, 'static, A, E, M, NullEventStore<A, E>, NullSnapshotStore<A>>,
    >
where
    A: Aggregate,
    E: AggregateEvent<A>,
{
    fn default() -> Self {
        CompositeEntityStore {
            entity_source: CompositeEntitySource::default(),
            entity_sink: CompositeEntitySink::default(),
            _phantom: PhantomData,
        }
    }
}

impl<A, E, M, ES, SS> CompositeEntityStore<A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EntitySource<A, E>,
    SS: EntitySink<A, E, M>,
{
    /// Attaches a specific entity source.
    pub fn with_entity_source<NewES>(
        self,
        entity_source: NewES,
    ) -> CompositeEntityStore<A, E, M, NewES, SS>
    where
        NewES: EntitySource<A, E>,
    {
        CompositeEntityStore {
            entity_source,
            entity_sink: self.entity_sink,
            _phantom: PhantomData,
        }
    }

    /// Attaches a specific entity sink.
    pub fn with_entity_sink<NewSS>(
        self,
        entity_sink: NewSS,
    ) -> CompositeEntityStore<A, E, M, ES, NewSS>
    where
        NewSS: EntitySink<A, E, M>,
    {
        CompositeEntityStore {
            entity_source: self.entity_source,
            entity_sink,
            _phantom: PhantomData,
        }
    }
}

impl<A, E, M, ES, SS> EventSource<A, E> for CompositeEntityStore<A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EntitySource<A, E>,
    SS: EntitySink<A, E, M>,
{
    type Error = <ES as EventSource<A, E>>::Error;
    type Events = <ES as EventSource<A, E>>::Events;

    fn read_events<I>(
        &self,
        id: &I,
        since: Since,
        max_count: Option<u64>,
    ) -> Result<Option<Self::Events>, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        self.entity_source.read_events(id, since, max_count)
    }
}

impl<A, E, M, ES, SS> SnapshotSource<A> for CompositeEntityStore<A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EntitySource<A, E>,
    SS: EntitySink<A, E, M>,
{
    type Error = <ES as SnapshotSource<A>>::Error;

    fn get_snapshot<I>(
        &self,
        id: &I,
    ) -> Result<Option<VersionedAggregate<A>>, <Self as SnapshotSource<A>>::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        self.entity_source.get_snapshot(id)
    }
}

impl<A, E, M, ES, SS> EventSink<A, E, M> for CompositeEntityStore<A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EntitySource<A, E>,
    SS: EntitySink<A, E, M>,
{
    type Error = <SS as EventSink<A, E, M>>::Error;

    fn append_events<I>(
        &self,
        id: &I,
        events: &[E],
        precondition: Option<Precondition>,
        metadata: M,
    ) -> Result<EventNumber, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        self.entity_sink
            .append_events(id, events, precondition, metadata)
    }
}

impl<A, E, M, ES, SS> SnapshotSink<A> for CompositeEntityStore<A, E, M, ES, SS>
where
    A: Aggregate,
    E: AggregateEvent<A>,
    ES: EntitySource<A, E>,
    SS: EntitySink<A, E, M>,
{
    type Error = <SS as SnapshotSink<A>>::Error;

    fn persist_snapshot<I>(
        &self,
        id: &I,
        aggregate: &A,
        version: Version,
        last_snapshot_version: Version,
    ) -> Result<Version, Self::Error>
    where
        I: AggregateId<Aggregate = A>,
    {
        self.entity_sink
            .persist_snapshot(id, aggregate, version, last_snapshot_version)
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
            EntityLoadError::EventSource(e) => {
                write!(f, "entity load error, problem loading events: {}", e)
            },
            EntityLoadError::SnapshotSource(e) => {
                write!(f, "entity load error, problem loading snapshot: {}", e)
            },
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
            EntityPersistError::EventSink(e) => {
                write!(f, "entity persist error, problem persisting events: {}", e)
            },
            EntityPersistError::SnapshotSink(e) => write!(
                f,
                "entity persist error, problem persisting snapshot (events successfully \
                 persisted): {}",
                e
            ),
        }
    }
}

/// An error produced when there is an error while attempting to execute a command against an aggregate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityExecAndPersistError<A, C, PEErr, PSErr>
where
    A: Aggregate,
    C: AggregateCommand<A>,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    /// The command could not be applied because the aggregate was not in the expected state.
    PreconditionFailed(Precondition),

    /// The command reported an error while executing against the aggregate.
    Exec(HydratedAggregate<A>, C::Error),

    /// An error occurred while persisting the entity.
    Persist(EntityPersistError<PEErr, PSErr>),
}

impl<A, C, PEErr, PSErr> fmt::Display for EntityExecAndPersistError<A, C, PEErr, PSErr>
where
    A: Aggregate,
    C: AggregateCommand<A>,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityExecAndPersistError::PreconditionFailed(p) => {
                write!(f, "entity exec error, precondition failed: {}", p)
            },
            EntityExecAndPersistError::Exec(_, e) => {
                write!(f, "entity exec error, command was rejected: {}", e)
            },
            EntityExecAndPersistError::Persist(e) => fmt::Display::fmt(&e, f),
        }
    }
}

impl<A, C, PEErr, PSErr> From<Precondition> for EntityExecAndPersistError<A, C, PEErr, PSErr>
where
    A: Aggregate,
    C: AggregateCommand<A>,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn from(p: Precondition) -> Self {
        EntityExecAndPersistError::PreconditionFailed(p)
    }
}

/// An error produced when there is an error attempting to load an aggregate, execute a command, and perist the results.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityError<LEErr, LSErr, A, C, PEErr, PSErr>
where
    A: Aggregate,
    C: AggregateCommand<A>,
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
    Exec(HydratedAggregate<A>, C::Error),

    /// An error occurred while persisting the entity.
    Persist(EntityPersistError<PEErr, PSErr>),
}

impl<LEErr, LSErr, A, C, PEErr, PSErr> fmt::Display
    for EntityError<LEErr, LSErr, A, C, PEErr, PSErr>
where
    A: Aggregate,
    C: AggregateCommand<A>,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityError::Load(e) => fmt::Display::fmt(&e, f),
            EntityError::PreconditionFailed(p) => {
                write!(f, "entity error, precondition failed: {}", p)
            },
            EntityError::Exec(_, e) => write!(f, "entity error, command was rejected: {}", e),
            EntityError::Persist(e) => fmt::Display::fmt(&e, f),
        }
    }
}

impl<LEErr, LSErr, A, C, PEErr, PSErr> From<Precondition>
    for EntityError<LEErr, LSErr, A, C, PEErr, PSErr>
where
    A: Aggregate,
    C: AggregateCommand<A>,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn from(p: Precondition) -> Self {
        EntityError::PreconditionFailed(p)
    }
}

impl<LEErr, LSErr, A, C, PEErr, PSErr> From<EntityExecAndPersistError<A, C, PEErr, PSErr>>
    for EntityError<LEErr, LSErr, A, C, PEErr, PSErr>
where
    A: Aggregate,
    C: AggregateCommand<A>,
    LEErr: CqrsError,
    LSErr: CqrsError,
    PEErr: CqrsError,
    PSErr: CqrsError,
{
    fn from(p: EntityExecAndPersistError<A, C, PEErr, PSErr>) -> Self {
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
    use crate::{memory::StateStore, testing::*};

    #[test]
    fn can_construct_composite_entity_source() {
        let null = NullEventStore::<TestAggregate, TestEvent>::default();
        let memory = StateStore::<TestAggregate>::default();
        let _source = CompositeEntitySource::default()
            .with_event_source(&null)
            .with_snapshot_source(&memory);
    }

    #[test]
    fn can_construct_composite_entity_sink() {
        let null = NullEventStore::<TestAggregate, TestEvent>::default();
        let memory = StateStore::<TestAggregate>::default();
        let _sink: CompositeEntitySink<
            TestAggregate,
            TestEvent,
            TestMetadata,
            NullEventStore<TestAggregate, TestEvent>,
            StateStore<TestAggregate>,
        > = CompositeEntitySink::default()
            .with_event_sink(&null)
            .with_snapshot_sink(&memory);
    }

    #[test]
    fn can_construct_composite_entity_store() {
        let null = NullEventStore::<TestAggregate, TestEvent>::default();
        let memory = StateStore::<TestAggregate>::default();
        let source = CompositeEntitySource::default()
            .with_event_source(&null)
            .with_snapshot_source(&memory);
        let sink: CompositeEntitySink<
            TestAggregate,
            TestEvent,
            TestMetadata,
            NullEventStore<TestAggregate, TestEvent>,
            StateStore<TestAggregate>,
        > = CompositeEntitySink::default()
            .with_event_sink(&null)
            .with_snapshot_sink(&memory);
        let _store = CompositeEntityStore::default()
            .with_entity_source(source)
            .with_entity_sink(sink);
    }
}
