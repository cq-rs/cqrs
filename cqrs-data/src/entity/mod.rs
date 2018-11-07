use std::borrow::Cow;
use std::fmt::{self, Debug, Display};
use cqrs::{Aggregate, SequencedEvent, Version};
use ::event;
use ::state;

mod source;
mod store;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Entity<'id, A>
where
    A: Aggregate,
    A::Event: Debug,
{
    version: Version,
    snapshot_version: Version,
    aggregate: A,
    id: Cow<'id, str>,
}

impl<'id, A: Aggregate + Debug> Entity<'id, A> where A::Event: Debug, {
    pub fn from_snapshot(id: impl Into<Cow<'id, str>>, version: Version, aggregate: A) -> Self {
        Entity {
            version,
            snapshot_version: version,
            aggregate,
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

    pub fn refresh<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, Err>> + Debug, Err: Debug + Display>(&mut self, event_source: &impl event::Source<A::Event, Events=Es, Error=Err>) -> Result<(), Err> {
        let events = event_source.read_events(self.id.as_ref(), ::Since::from(self.version))?;

        println!("Events! {:?}", events);

        println!("! {:?}", self);
        if let Some(events) = events {
            for event in events {
                let event = event?;

                self.aggregate.apply(event.event);

                self.version = self.version.incr();

                println!("! {:?}", self);

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

    pub fn load_from_snapshot<Err: Debug + Display>(id: impl Into<Cow<'id, str>>, state_source: &impl state::Source<A, Error=Err>) -> Result<Option<Self>, Err> {
        let id = id.into();
        let entity =
            state_source.get_snapshot(id.as_ref())?.map(|state| {
                Entity {
                    version: state.version,
                    snapshot_version: state.version,
                    aggregate: state.snapshot,
                    id,
                }
            });

        Ok(entity)
    }

    pub fn rehydrate_from_snapshot<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>> + Debug, EErr: Debug + Display, SErr: Debug + Display>(id: impl Into<Cow<'id, str>>, event_source: &impl event::Source<A::Event, Events=Es, Error=EErr>, state_source: &impl state::Source<A, Error=SErr>) -> Result<Option<Self>, EntityLoadError<EErr, SErr>> {
        let id = id.into();
        let entity = Self::load_from_snapshot(id, state_source).map_err(EntityLoadError::StateSource)?;

        if let Some(mut e) = entity {
            e.refresh(event_source).map_err(EntityLoadError::EventSource)?;

            Ok(Some(e))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityLoadError<EErr, SErr> {
    EventSource(EErr),
    StateSource(SErr),
}

impl<EErr: Debug, SErr: Debug> fmt::Display for EntityLoadError<EErr, SErr> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntityLoadError::EventSource(e) =>
                write!(f, "Entity load error, problem loading events: {:?}", e),
            EntityLoadError::StateSource(e) =>
                write!(f, "Entity load error, problem loading snapshot: {:?}", e),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotStatus {
    Missing,
    Found,
}

impl<'id, A> Entity<'id, A>
where
    A: Aggregate + Default + Debug,
    A::Event: Debug
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


    pub fn load_from_snapshot_or_default<Err: Debug + Display>(id: impl Into<Cow<'id, str>>, state_source: &impl state::Source<A, Error=Err>) -> Result<(Self, SnapshotStatus), Err> {
        let id = id.into();

        let entity =
            if let Some(state) = state_source.get_snapshot(id.as_ref())? {
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

    pub fn rehydrate<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>> + Debug, EErr: Debug + Display, SErr: Debug + Display>(id: impl Into<Cow<'id, str>>, event_source: &impl event::Source<A::Event, Events=Es, Error=EErr>, state_source: &impl state::Source<A, Error=SErr>) -> Result<Option<Self>, EntityLoadError<EErr, SErr>> {
        let (mut entity, snapshot_status) = Self::load_from_snapshot_or_default(id, state_source).map_err(EntityLoadError::StateSource)?;

        entity.refresh(event_source).map_err(EntityLoadError::EventSource)?;

        if snapshot_status == SnapshotStatus::Missing && entity.version == Version::Initial {
            Ok(None)
        } else {
            Ok(Some(entity))
        }
    }

    pub fn rehydrate_or_default<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>> + Debug, EErr: Debug + Display, SErr: Debug + Display>(id: impl Into<Cow<'id, str>>, event_source: &impl event::Source<A::Event, Events=Es, Error=EErr>, state_source: &impl state::Source<A, Error=SErr>) -> Result<Self, EntityLoadError<EErr, SErr>> {
        let (mut entity, _) = Self::load_from_snapshot_or_default(id, state_source).map_err(EntityLoadError::StateSource)?;

        entity.refresh(event_source).map_err(EntityLoadError::EventSource)?;

        Ok(entity)
    }
}