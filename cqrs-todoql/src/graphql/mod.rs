use std::ops;
use std::sync::Arc;
use parking_lot::RwLock;

use juniper;

mod schema;
pub mod endpoint;

pub struct InnerContext {
    pub stream_index: RwLock<Vec<String>>,
    pub event_db: super::EventStore,
    pub state_db: super::SnapshotStore,
    pub id_provider: super::IdProvider,
}

impl InnerContext {
    pub fn new(stream_index: Vec<String>, event_db: super::EventStore, state_db: super::SnapshotStore, id_provider: super::IdProvider) -> Self {
        InnerContext {
            stream_index: RwLock::new(stream_index),
            event_db,
            state_db,
            id_provider,
        }
    }
}

pub struct Context {
    inner: Arc<InnerContext>
}

impl ops::Deref for Context {
    type Target = InnerContext;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl juniper::Context for Context {}
