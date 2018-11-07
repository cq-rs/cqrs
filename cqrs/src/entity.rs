use std::borrow::Cow;
use std::fmt::{self, Debug, Display};
use super::*;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Entity<'id, A>
where
    A: Aggregate
{
    version: Version,
    snapshot_version: Version,
    aggregate: A,
    id: Cow<'id, str>,
}

impl<'id, A: Aggregate> Entity<'id, A> {
    pub fn from_snapshot(id: impl Into<Cow<'id, str>>, version: Version, snapshot: A) -> Self {
        Entity {
            version,
            snapshot_version: version,
            aggregate: snapshot,
            id: id.into(),
        }
    }

    pub fn with_initial_state(id: impl Into<Cow<'id, str>>, aggregate: A) -> Self {
        Entity {
            version: Version::default(),
            snapshot_version: Version::default(),
            aggregate,
            id: id.into(),
        }
    }

    pub fn refresh<Es, Err>(&mut self, event_source: &impl EventSource<A, Events=Es, Error=Err>) -> Result<(), Err>
    where
        Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, Err>>,
        Err: Debug + Display + Send + Sync + 'static,
    {
        let events = event_source.read_events(self.id.as_ref(), self.version.into())?;

        if let Some(events) = events {
            for event in events {
                let event = event?;

                self.aggregate.apply(event.event);
                self.version = self.version.incr();

                debug_assert_eq!(Version::Number(event.sequence), self.version);
            }
        }

        Ok(())
    }

    pub fn id(&self) -> &Cow<'id, str> {
        &self.id
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn snapshot_version(&self) -> Version {
        self.snapshot_version
    }

    pub fn aggregate(&self) -> &A {
        &self.aggregate
    }

    pub fn apply_events(&mut self, events: A::Events) {
        for e in events {
            self.aggregate.apply(e);
            self.version = self.version.incr();
        }
    }

    pub fn load_from_snapshot<Err>(
        id: impl Into<Cow<'id, str>>,
        snapshot_source: &impl SnapshotSource<A, Error=Err>
    ) -> Result<Option<Self>, Err>
    where
        Err: Debug + Display + Send + Sync + 'static,
    {
        let id = id.into();
        let entity =
            snapshot_source.get_snapshot(id.as_ref())?.map(|state| {
                Entity {
                    version: state.version,
                    snapshot_version: state.version,
                    aggregate: state.snapshot,
                    id,
                }
            });

        Ok(entity)
    }

    pub fn rehydrate_from_snapshot<Es, EErr, SErr>(
        id: impl Into<Cow<'id, str>>,
        event_source: &impl EventSource<A, Events=Es, Error=EErr>,
        snapshot_source: &impl SnapshotSource<A, Error=SErr>
    ) -> Result<Option<Self>, EntityLoadError<EErr, SErr>>
    where
        Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>>,
        EErr: Debug + Display + Send + Sync + 'static,
        SErr: Debug + Display + Send + Sync + 'static,
    {
        let id = id.into();
        let entity = Self::load_from_snapshot(id, snapshot_source).map_err(EntityLoadError::SnapshotSource)?;

        if let Some(mut e) = entity {
            e.refresh(event_source).map_err(EntityLoadError::EventSource)?;

            Ok(Some(e))
        } else {
            Ok(None)
        }
    }

    pub fn apply_events_and_persist<EErr, SErr>(
        &mut self,
        events: A::Events,
        precondition: Precondition,
        event_sink: &impl EventSink<A, Error=EErr>,
        snapshot_sink: &impl SnapshotSink<A, Error=SErr>,
        max_events_before_snapshot: u64
    ) -> Result<(), EntityPersistError<EErr, SErr>>
    where
        A: Clone,
        EErr: Debug + Display + Send + Sync + 'static,
        SErr: Debug + Display + Send + Sync + 'static,
    {
        let events: Vec<_> = events.into_iter().collect();
        event_sink.append_events(self.id.as_ref(), &events, Some(precondition)).map_err(EntityPersistError::EventSink)?;

        for e in events {
            self.aggregate.apply(e);
            self.version = self.version.incr();
        }

        if self.version - self.snapshot_version >= max_events_before_snapshot as i64 {
            let snapshot = StateSnapshot {
                snapshot: self.aggregate.clone(),
                version: self.version,
            };
            snapshot_sink.persist_snapshot(self.id.as_ref(), snapshot).map_err(EntityPersistError::SnapshotSink)?
        }

        Ok(())
    }

    pub fn from_default(id: impl Into<Cow<'id, str>>) -> Self {
        let id = id.into();

        Entity {
            version: Version::default(),
            snapshot_version: Version::default(),
            aggregate: A::default(),
            id,
        }
    }


    pub fn load_from_snapshot_or_default<Err>(
        id: impl Into<Cow<'id, str>>,
        snapshot_source: &impl SnapshotSource<A, Error=Err>
    ) -> Result<(Self, SnapshotStatus), Err>
    where
        Err: Debug + Display + Send + Sync + 'static,
    {
        let id = id.into();

        let entity =
            if let Some(state) = snapshot_source.get_snapshot(id.as_ref())? {
                (Entity {
                    version: state.version,
                    snapshot_version: state.version,
                    aggregate: state.snapshot,
                    id,
                }, SnapshotStatus::Found)
            } else {
                (Entity::from_default(id), SnapshotStatus::Missing)
            };

        Ok(entity)
    }

    pub fn rehydrate<Es, EErr, SErr>(
        id: impl Into<Cow<'id, str>>,
        event_source: &impl EventSource<A, Events=Es, Error=EErr>,
        snapshot_source: &impl SnapshotSource<A, Error=SErr>
    ) -> Result<Option<Self>, EntityLoadError<EErr, SErr>>
    where
        Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>>,
        EErr: Debug + Display + Send + Sync + 'static,
        SErr: Debug + Display + Send + Sync + 'static,
    {
        let (mut entity, snapshot_status) = Self::load_from_snapshot_or_default(id, snapshot_source).map_err(EntityLoadError::SnapshotSource)?;

        entity.refresh(event_source).map_err(EntityLoadError::EventSource)?;

        if snapshot_status == SnapshotStatus::Missing && entity.version == Version::Initial {
            Ok(None)
        } else {
            Ok(Some(entity))
        }
    }

    pub fn rehydrate_or_default<Es, EErr, SErr>(
        id: impl Into<Cow<'id, str>>,
        event_source: &impl EventSource<A, Events=Es, Error=EErr>,
        snapshot_source: &impl SnapshotSource<A, Error=SErr>
    ) -> Result<Self, EntityLoadError<EErr, SErr>>
    where
        Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>>,
        EErr: Debug + Display + Send + Sync + 'static,
        SErr: Debug + Display + Send + Sync + 'static,
    {
        let (mut entity, _) = Self::load_from_snapshot_or_default(id, snapshot_source).map_err(EntityLoadError::SnapshotSource)?;

        entity.refresh(event_source).map_err(EntityLoadError::EventSource)?;

        Ok(entity)
    }


    pub fn load_exec_and_persist<LEErr, LSErr, PEErr, PSErr>(
        id: impl Into<Cow<'id, str>>,
        command: A::Command,
        precondition: Option<Precondition>,
        event_source: &impl EventSource<A, Error=LEErr>,
        snapshot_source: &impl SnapshotSource<A, Error=LSErr>,
        event_sink: &impl EventSink<A, Error=PEErr>,
        snapshot_sink: &impl SnapshotSink<A, Error=PSErr>,
        max_events_before_snapshot: u64
    ) -> Result<Option<Entity<'id, A>>, EntityExecError<LEErr, LSErr, A, PEErr, PSErr>>
    where
        A: Clone,
        LEErr: Debug + Display + Send + Sync + 'static,
        LSErr: Debug + Display + Send + Sync + 'static,
        PEErr: Debug + Display + Send + Sync + 'static,
        PSErr: Debug + Display + Send + Sync + 'static,
    {
        if let Some(mut entity) = Self::rehydrate(id.into(), event_source, snapshot_source).map_err(EntityExecError::Load)? {
            if let Some(precondition) = precondition {
                precondition.verify(Some(entity.version()))?;
            }

            let precondition = Precondition::ExpectedVersion(entity.version());

            let events = entity.aggregate.execute(command).map_err(|e| EntityExecError::Exec(entity.clone_with_static_lifetime(), e))?;

            entity.apply_events_and_persist(
                events,
                precondition,
                event_sink,
                snapshot_sink,
                max_events_before_snapshot,
            ).map_err(EntityExecError::Persist)?;

            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }

    pub fn clone_with_static_lifetime(&self) -> Entity<'static, A>
    where
        A: Clone,
    {
        let entity = self.clone();
        Entity {
            version: entity.version,
            snapshot_version: entity.snapshot_version,
            aggregate: entity.aggregate,
            id: entity.id.to_string().into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityLoadError<EErr, SErr>
where
    EErr: Debug + Display + Send + Sync + 'static,
    SErr: Debug + Display + Send + Sync + 'static,
{
    EventSource(EErr),
    SnapshotSource(SErr),
}

impl<EErr, SErr> fmt::Display for EntityLoadError<EErr, SErr>
where
    EErr: Debug + Display + Send + Sync + 'static,
    SErr: Debug + Display + Send + Sync + 'static,
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
    EErr: Debug + Display + Send + Sync + 'static,
    SErr: Debug + Display + Send + Sync + 'static,
{
    EventSink(EErr),
    SnapshotSink(SErr),
}

impl<EErr, SErr> fmt::Display for EntityPersistError<EErr, SErr>
where
    EErr: Debug + Display + Send + Sync + 'static,
    SErr: Debug + Display + Send + Sync + 'static,
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
pub enum EntityExecError<LEErr, LSErr, A, PEErr, PSErr>
where
    A: Aggregate,
    LEErr: Debug + Display + Send + Sync + 'static,
    LSErr: Debug + Display + Send + Sync + 'static,
    PEErr: Debug + Display + Send + Sync + 'static,
    PSErr: Debug + Display + Send + Sync + 'static,
{
    Load(EntityLoadError<LEErr, LSErr>),
    PreconditionFailed(Precondition),
    Exec(Entity<'static, A>, A::Error),
    Persist(EntityPersistError<PEErr, PSErr>),
}

impl<LEErr, LSErr, A, PEErr, PSErr> fmt::Display for EntityExecError<LEErr, LSErr, A, PEErr, PSErr>
where
    A: Aggregate,
    LEErr: Debug + Display + Send + Sync + 'static,
    LSErr: Debug + Display + Send + Sync + 'static,
    PEErr: Debug + Display + Send + Sync + 'static,
    PSErr: Debug + Display + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityExecError::Load(e) => Display::fmt(&e, f),
            EntityExecError::PreconditionFailed(p) =>
                write!(f, "entity exec error, precondition failed: {}", p),
            EntityExecError::Exec(_, e) =>
                write!(f, "entity exec error, command was rejected: {}", e),
            EntityExecError::Persist(e) => Display::fmt(&e, f),
        }
    }
}

impl<LEErr, LSErr, A, PEErr, PSErr> From<Precondition> for EntityExecError<LEErr, LSErr, A, PEErr, PSErr>
where
    A: Aggregate,
    LEErr: Debug + Display + Send + Sync + 'static,
    LSErr: Debug + Display + Send + Sync + 'static,
    PEErr: Debug + Display + Send + Sync + 'static,
    PSErr: Debug + Display + Send + Sync + 'static,
{
    fn from(p: Precondition) -> Self {
        EntityExecError::PreconditionFailed(p)
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotStatus {
    Missing,
    Found,
}
