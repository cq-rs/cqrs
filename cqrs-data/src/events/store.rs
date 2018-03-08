use cqrs::Precondition;

pub trait Store {
    type AggregateId: ?Sized;
    type Event;
    type Result;

    fn append_events(&self, agg_id: &Self::AggregateId, events: &[Self::Event], precondition: Option<Precondition>) -> Self::Result;

    fn append_events_from_iterator<I: IntoIterator<Item=Self::Event>>(&self, agg_id: &Self::AggregateId, event_iter: I, precondition: Option<Precondition>) -> Self::Result
        where Self: Sized,
    {
        let events: Vec<Self::Event> = event_iter.into_iter().collect();
        self.append_events(agg_id, &events, precondition)
    }
}

#[cfg(test)]
assert_obj_safe!(evtsnk; Store<AggregateId=str, Event=(), Result=()>, Store<AggregateId=usize, Event=(), Result=()>);
