use std::sync::atomic::{AtomicUsize, Ordering};

pub trait AggregateIdProvider {
    type AggregateId;

    fn new_id(&self) -> Self::AggregateId;
}

#[derive(Debug, Default)]
pub struct UsizeIdProvider(AtomicUsize);

impl AggregateIdProvider for UsizeIdProvider {
    type AggregateId = usize;

    fn new_id(&self) -> Self::AggregateId {
        self.0.fetch_add(1, Ordering::SeqCst)
    }
}
