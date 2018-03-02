use Precondition;

pub trait Store<'a> {
    type AggregateId: 'a;
    type Event: 'a;
    type Result;

    fn append_events(&'a self, agg_id: Self::AggregateId, events: &[Self::Event], precondition: Option<Precondition>) -> Self::Result;

    fn append_events_from_iterator<I: IntoIterator<Item=Self::Event>>(&'a self, agg_id: Self::AggregateId, event_iter: I, precondition: Option<Precondition>) -> Self::Result
        where Self: Sized,
    {
        let events: Vec<Self::Event> = event_iter.into_iter().collect();
        self.append_events(agg_id, &events, precondition)
    }
}

#[cfg(test)]
assert_obj_safe!(evtsnk; Store<'static, AggregateId=&'static str, Event=(), Result=()>, Store<'static, AggregateId=usize, Event=(), Result=()>, Store<'static, AggregateId=&'static str, Event=&'static (), Result=()>);
