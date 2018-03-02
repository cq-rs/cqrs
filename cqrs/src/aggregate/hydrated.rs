use types::{EventNumber, Precondition, Version, SequencedEvent, VersionedSnapshot};
use error::ExecuteError;
use projection::Projection;
use super::{Aggregate, PersistableAggregate};

#[derive(Debug, Default, Clone, PartialEq, Hash)]
pub struct HydratedAggregate<Agg> {
    latest_applied_event_number: Option<EventNumber>,
    applied_event_count: usize,
    aggregate: Agg,
    rehydrated_version: Version,
}

impl<Agg> HydratedAggregate<Agg> {
    #[inline]
    pub fn is_initial(&self) -> bool {
        self.rehydrated_version == Version::Initial && self.latest_applied_event_number.is_none()
    }

    #[inline]
    pub fn latest_applied_event_number(&self) -> Option<EventNumber> {
        self.latest_applied_event_number
    }

    #[inline]
    pub fn aggregate(&self) -> &Agg {
        &self.aggregate
    }

    #[inline]
    pub fn version_at_rehydration(&self) -> Version {
        self.rehydrated_version
    }

    #[inline]
    pub fn applied_event_count(&self) -> usize {
        self.applied_event_count
    }

    #[inline]
    pub fn current_version(&self) -> Version {
        self.latest_applied_event_number
            .map(|n| n.into())
            .unwrap_or(self.rehydrated_version)
    }
}

impl<Agg: Aggregate + Default> HydratedAggregate<Agg> {
    fn verify_precondition(state_opt: Option<Self>, precondition: Precondition) -> Result<Self, ExecuteError<Agg::Error>> {
        if let Some(state) = state_opt {
            match precondition {
                Precondition::Exists => Ok(state),
                Precondition::ExpectedVersion(v) if v == state.current_version() => Ok(state),
                Precondition::ExpectedVersion(_) | Precondition::New =>
                    Err(ExecuteError::PreconditionFailed(precondition)),
            }
        } else if precondition == Precondition::New {
            Ok(Default::default())
        } else {
            Err(ExecuteError::PreconditionFailed(precondition))
        }
    }

    pub fn execute_if(state_opt: Option<Self>, command: Agg::Command, precondition: Precondition) -> Result<Agg::Events, ExecuteError<Agg::Error>> {
        let state = Self::verify_precondition(state_opt, precondition)?;

        state.aggregate.execute(command)
            .map_err(|e| ExecuteError::Command(e))
    }
}

impl<Agg: Projection> Projection for HydratedAggregate<Agg> {
    type Event = SequencedEvent<Agg::Event>;

    #[inline]
    fn apply(&mut self, seq_event: Self::Event) {
        self.aggregate.apply(seq_event.event);
        self.latest_applied_event_number = Some(seq_event.sequence_number);
        self.applied_event_count += 1;
    }
}

impl<Agg: Aggregate> Aggregate for HydratedAggregate<Agg> {
    type Command = Agg::Command;
    type Events = Agg::Events;
    type Error = Agg::Error;

    #[inline]
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error> {
        self.aggregate.execute(command)
    }
}

impl<Agg: PersistableAggregate> PersistableAggregate for HydratedAggregate<Agg> {
    type Snapshot = VersionedSnapshot<Agg::Snapshot>;

    #[inline]
    fn restore(snapshot: Self::Snapshot) -> Self
        where Self: Sized,
    {
        HydratedAggregate {
            latest_applied_event_number: None,
            applied_event_count: 0,
            rehydrated_version: snapshot.version,
            aggregate: PersistableAggregate::restore(snapshot.snapshot)
        }
    }

    #[inline]
    fn into_snapshot(self) -> Self::Snapshot
        where Self: Sized,
    {
        VersionedSnapshot {
            version: self.current_version(),
            snapshot: self.aggregate.into_snapshot(),
        }
    }
}

