use std::borrow::Cow;
use std::fmt::{self, Debug, Display};
use cqrs::{Aggregate, SequencedEvent, Version};

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

    pub fn refresh<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, Err>>, Err: Debug + Display>(&mut self, event_source: &impl event::EventSource<A, Events=Es, Error=Err>) -> Result<(), Err> {
        let events = event_source.read_events(self.id.as_ref(), ::Since::from(self.version))?;

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

    pub fn load_from_snapshot<Err: Debug + Display>(id: impl Into<Cow<'id, str>>, snapshot_source: &impl SnapshotSource<A, Error=Err>) -> Result<Option<Self>, Err> {
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

    pub fn rehydrate_from_snapshot<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>>, EErr: Debug + Display, SErr: Debug + Display>(id: impl Into<Cow<'id, str>>, event_source: &impl event::EventSource<A, Events=Es, Error=EErr>, snapshot_source: &impl SnapshotSource<A, Error=SErr>) -> Result<Option<Self>, EntityLoadError<EErr, SErr>> {
        let id = id.into();
        let entity = Self::load_from_snapshot(id, snapshot_source).map_err(EntityLoadError::SnapshotSource)?;

        if let Some(mut e) = entity {
            e.refresh(event_source).map_err(EntityLoadError::EventSource)?;

            Ok(Some(e))
        } else {
            Ok(None)
        }
    }

    pub fn apply_events_and_persist<EErr: Debug + Display, SErr: Debug + Display>(&mut self, events: A::Events, precondition: cqrs::Precondition, event_sink: &impl EventSink<A, Error=EErr>, snapshot_sink: &impl SnapshotSink<A, Error=SErr>, max_events_before_snapshot: u64) -> Result<(), EntityPersistError<EErr, SErr>>
    where A: Clone,
    {
        let events: Vec<_> = events.into_iter().collect();
        event_sink.append_events(self.id.as_ref(), &events, Some(precondition)).map_err(EntityPersistError::EventSink)?;

        for e in events {
            self.aggregate.apply(e);
            self.version = self.version.incr();
        }

        if self.version - self.snapshot_version >= max_events_before_snapshot as i64 {
            let snapshot = cqrs::StateSnapshot {
                snapshot: self.aggregate.clone(),
                version: self.version,
            };
            snapshot_sink.persist_snapshot(self.id.as_ref(), snapshot).map_err(EntityPersistError::SnapshotSink)?
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityLoadError<EErr: Debug + Display, SErr: Debug + Display> {
    EventSource(EErr),
    SnapshotSource(SErr),
}

impl<EErr: Debug + Display, SErr: Debug + Display> fmt::Display for EntityLoadError<EErr, SErr> {
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
pub enum EntityPersistError<EErr, SErr> {
    EventSink(EErr),
    SnapshotSink(SErr),
}

impl<EErr: Debug + Display, SErr: Debug + Display> fmt::Display for EntityPersistError<EErr, SErr> {
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
pub enum EntityExecError<'id, LEErr: Debug + Display, LSErr: Debug + Display, A, PEErr: Debug + Display, PSErr: Debug + Display>
where A: Aggregate,
{
    Load(EntityLoadError<LEErr, LSErr>),
    Precondition(cqrs::Precondition),
    Exec(Entity<'id, A>, A::Error),
    Persist(EntityPersistError<PEErr, PSErr>),
}

impl<'id, LEErr: Debug + Display, LSErr: Debug + Display, A: Aggregate, PEErr: Debug + Display, PSErr: Debug + Display> fmt::Display for EntityExecError<'id, LEErr, LSErr, A, PEErr, PSErr> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityExecError::Load(e) => Display::fmt(&e, f),
            EntityExecError::Precondition(p) =>
                write!(f, "entity exec error, precondition failed: {}", p),
            EntityExecError::Exec(_, e) =>
                write!(f, "entity exec error, command was rejected: {}", e),
            EntityExecError::Persist(e) => Display::fmt(&e, f),
        }
    }
}

impl<'id, LEErr: Debug + Display, LSErr: Debug + Display, A: Aggregate, PEErr: Debug + Display, PSErr: Debug + Display> From<cqrs::Precondition> for EntityExecError<'id, LEErr, LSErr, A, PEErr, PSErr> {
    fn from(p: cqrs::Precondition) -> Self {
        EntityExecError::Precondition(p)
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotStatus {
    Missing,
    Found,
}

impl<'id, A> Entity<'id, A>
where
    A: Aggregate + Default,
{
    pub fn from_default(id: impl Into<Cow<'id, str>>) -> Self {
        let id = id.into();

        Entity {
            version: Version::default(),
            snapshot_version: Version::default(),
            aggregate: A::default(),
            id,
        }
    }


    pub fn load_from_snapshot_or_default<Err: Debug + Display>(id: impl Into<Cow<'id, str>>, snapshot_source: &impl state::SnapshotSource<A, Error=Err>) -> Result<(Self, SnapshotStatus), Err> {
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

    pub fn rehydrate<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>>, EErr: Debug + Display, SErr: Debug + Display>(id: impl Into<Cow<'id, str>>, event_source: &impl event::EventSource<A, Events=Es, Error=EErr>, snapshot_source: &impl state::SnapshotSource<A, Error=SErr>) -> Result<Option<Self>, EntityLoadError<EErr, SErr>> {
        let (mut entity, snapshot_status) = Self::load_from_snapshot_or_default(id, snapshot_source).map_err(EntityLoadError::SnapshotSource)?;

        entity.refresh(event_source).map_err(EntityLoadError::EventSource)?;

        if snapshot_status == SnapshotStatus::Missing && entity.version == Version::Initial {
            Ok(None)
        } else {
            Ok(Some(entity))
        }
    }

    pub fn rehydrate_or_default<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>>, EErr: Debug + Display, SErr: Debug + Display>(id: impl Into<Cow<'id, str>>, event_source: &impl event::EventSource<A, Events=Es, Error=EErr>, snapshot_source: &impl state::SnapshotSource<A, Error=SErr>) -> Result<Self, EntityLoadError<EErr, SErr>> {
        let (mut entity, _) = Self::load_from_snapshot_or_default(id, snapshot_source).map_err(EntityLoadError::SnapshotSource)?;

        entity.refresh(event_source).map_err(EntityLoadError::EventSource)?;

        Ok(entity)
    }


    pub fn load_exec_and_persist<Id: Into<Cow<'id, str>>, LEErr: Debug + Display, LSErr: Debug + Display, PEErr: Debug + Display, PSErr: Debug + Display>(
        id: Id,
        command: A::Command,
        precondition: Option<cqrs::Precondition>,
        event_source: &impl EventSource<A, Error=LEErr>,
        snapshot_source: &impl SnapshotSource<A, Error=LSErr>,
        event_sink: &impl EventSink<A, Error=PEErr>,
        snapshot_sink: &impl SnapshotSink<A, Error=PSErr>,
        max_events_before_snapshot: u64)
        -> Result<Option<Entity<'id, A>>, EntityExecError<'id, LEErr, LSErr, A, PEErr, PSErr>>
        where A: Clone,
    {
        if let Some(mut entity) = Self::rehydrate(id.into(), event_source, snapshot_source).map_err(EntityExecError::Load)? {
            if let Some(precondition) = precondition {
                precondition.verify(Some(entity.version()))?;
            }

            let precondition = precondition.unwrap_or(cqrs::Precondition::New);

            let events = entity.aggregate().execute(command).map_err(|e| EntityExecError::Exec(entity.clone(), e))?;

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
}