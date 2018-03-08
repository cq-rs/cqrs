use types::{EventNumber, Version, SequencedEvent, VersionedSnapshot};
use super::Aggregate;

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

impl<Agg: Aggregate> HydratedAggregate<Agg> {
    #[inline]
    pub fn apply_raw(&mut self, event: Agg::Event, event_number: Option<EventNumber>) {
        self.aggregate.apply(event);
        self.applied_event_count += 1;
        self.latest_applied_event_number =
            Some(
                event_number.unwrap_or_else(||
                    self.latest_applied_event_number
                        .map(|v| v.incr())
                        .unwrap_or_default()));
    }
}

impl<Agg: Aggregate> Aggregate for HydratedAggregate<Agg> {
    type Event = SequencedEvent<Agg::Event>;
    type Command = Agg::Command;
    type Events = Agg::Events;
    type Error = Agg::Error;

    #[inline]
    fn apply(&mut self, seq_event: Self::Event) {
        self.apply_raw(seq_event.event, Some(seq_event.sequence_number));
    }

    #[inline]
    fn execute(&self, command: Self::Command) -> Result<Self::Events, Self::Error> {
        self.aggregate.execute(command)
    }
}

impl<Agg: Aggregate> HydratedAggregate<Agg> {
    #[inline]
    pub fn restore(versioned_snapshot: VersionedSnapshot<Agg>) -> Self
        where Self: Sized,
    {
        HydratedAggregate {
            latest_applied_event_number: None,
            applied_event_count: 0,
            rehydrated_version: versioned_snapshot.version,
            aggregate: versioned_snapshot.snapshot,
        }
    }

    #[inline]
    pub fn snapshot(self) -> VersionedSnapshot<Agg>
        where Self: Sized,
    {
        VersionedSnapshot {
            version: self.current_version(),
            snapshot: self.aggregate,
        }
    }
}

