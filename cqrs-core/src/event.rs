use aggregate::Aggregate;
use types::{CqrsError, EventNumber, Precondition, SequencedEvent, Since};

pub trait EventSource<A: Aggregate> {
    type Events: IntoIterator<Item=Result<SequencedEvent<A::Event>, Self::Error>>;
    type Error: CqrsError;

    fn read_events(&self, id: &str, since: Since) -> Result<Option<Self::Events>, Self::Error>;
}

pub trait EventSink<A: Aggregate> {
    type Error: CqrsError;

    fn append_events(&self, id: &str, events: &[A::Event], precondition: Option<Precondition>) -> Result<EventNumber, Self::Error>;

    fn append_events_from_iterator<I>(&self, id: &str, event_iter: I, precondition: Option<Precondition>) -> Result<EventNumber, Self::Error>
    where
        I: IntoIterator<Item=A::Event>,
        Self: Sized,
    {
        let events: Vec<A::Event> = event_iter.into_iter().collect();
        self.append_events(id, &events, precondition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aggregate::testing::TestAggregate;
    use std::iter::Empty;
    use void::Void;

    assert_obj_safe!(event_source_object_safety; EventSource<TestAggregate, Events=Empty<()>, Error=Void>);
    assert_obj_safe!(event_sink_object_safety; EventSink<TestAggregate, Error=Void>);
}