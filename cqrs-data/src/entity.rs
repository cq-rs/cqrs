use std::fmt::{self, Debug};
use cqrs::{Aggregate, SequencedEvent, Version};
use ::event;
use ::state;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Entity<A, I>
where
    A: Aggregate,
    A::Event: Debug,
{
    version: Version,
    snapshot_version: Version,
    aggregate: A,
    id: I,
}

impl<A: Aggregate + Debug, I: Debug> Entity<A, I> where     A::Event: Debug, {
    pub fn from_snapshot(id: I, version: Version, aggregate: A) -> Self {
        Entity {
            version,
            snapshot_version: version,
            aggregate,
            id,
        }
    }

    pub fn with_initial_state(id: I, aggregate: A) -> Self {
        Entity {
            version: Version::default(),
            snapshot_version: Version::default(),
            aggregate,
            id,
        }
    }

    pub fn refresh<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, Err>> + Debug, Err: Debug>(&mut self, event_source: &impl event::Source<A::Event, AggregateId=I, Events=Es, Error=Err>) -> Result<(), Err> {
        let events = event_source.read_events(&self.id, ::Since::from(self.version))?;

        println!("Events! {:?}", events);

        println!("! {:?}", self);
        if let Some(events) = events {
            for event in events {
                let event = event?;

                self.aggregate.apply(event.event);

                self.version = self.version.incr();

                println!("! {:?}", self);

                debug_assert_eq!(Version::Number(event.sequence_number), self.version);
            }
        }

        Ok(())
    }

    pub fn id(&self) -> &I {
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
}

impl<A: Aggregate + Debug, I: Clone + Debug> Entity<A, I> where    A::Event: Debug, {
    pub fn load_from_snapshot<Err: Debug>(id: I, state_source: &impl state::Source<A, AggregateId=I, Error=Err>) -> Result<Option<Self>, Err> {
        let entity =
            state_source.get_snapshot(&id)?.map(|state| {
                Entity {
                    version: state.version,
                    snapshot_version: state.version,
                    aggregate: state.snapshot,
                    id,
                }
            });

        Ok(entity)
    }


    pub fn rehydrate_from_snapshot<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>> + Debug, EErr: Debug, SErr: Debug>(id: I, event_source: &impl event::Source<A::Event, AggregateId=I, Events=Es, Error=EErr>, state_source: &impl state::Source<A, AggregateId=I, Error=SErr>) -> Result<Option<Self>, EntityLoadError<EErr, SErr>> {
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

impl<A, I> Entity<A, I>
where
    A: Aggregate + Default + Debug,
    A::Event: Debug,
    I: Debug
{
    pub fn from_default(id: I) -> Self {
        Entity {
            version: Version::default(),
            snapshot_version: Version::default(),
            aggregate: A::default(),
            id,
        }
    }


    pub fn load_from_snapshot_or_default<Err: Debug>(id: I, state_source: &impl state::Source<A, AggregateId=I, Error=Err>) -> Result<(Self, SnapshotStatus), Err> {
        let entity =
            if let Some(state) = state_source.get_snapshot(&id)? {
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

    pub fn rehydrate<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>> + Debug, EErr: Debug, SErr: Debug>(id: I, event_source: &impl event::Source<A::Event, AggregateId=I, Events=Es, Error=EErr>, state_source: &impl state::Source<A, AggregateId=I, Error=SErr>) -> Result<Option<Self>, EntityLoadError<EErr, SErr>> {
        let (mut entity, snapshot_status) = Self::load_from_snapshot_or_default(id, state_source).map_err(EntityLoadError::StateSource)?;

        entity.refresh(event_source).map_err(EntityLoadError::EventSource)?;

        if snapshot_status == SnapshotStatus::Missing && entity.version == Version::Initial {
            Ok(None)
        } else {
            Ok(Some(entity))
        }
    }

    pub fn rehydrate_or_default<Es: IntoIterator<Item=Result<SequencedEvent<A::Event>, EErr>> + Debug, EErr: Debug, SErr: Debug>(id: I, event_source: &impl event::Source<A::Event, AggregateId=I, Events=Es, Error=EErr>, state_source: &impl state::Source<A, AggregateId=I, Error=SErr>) -> Result<Self, EntityLoadError<EErr, SErr>> {
        let (mut entity, _) = Self::load_from_snapshot_or_default(id, state_source).map_err(EntityLoadError::StateSource)?;

        entity.refresh(event_source).map_err(EntityLoadError::EventSource)?;

        Ok(entity)
    }
}