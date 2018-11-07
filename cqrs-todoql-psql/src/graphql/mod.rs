use std::ops;
use std::sync::{Arc,RwLock};

use juniper;
use r2d2_postgres::PostgresConnectionManager;
use r2d2::Pool;

use super::{AggregateId};

mod schema;
pub mod endpoint;

pub struct InnerContext {
    pub stream_index: RwLock<Vec<AggregateId>>,
    pub backend: Pool<PostgresConnectionManager>,
    pub id_provider: super::IdProvider,
}

impl InnerContext {
    pub fn new(stream_index: Vec<AggregateId>, backend: Pool<PostgresConnectionManager>, id_provider: super::IdProvider) -> Self {
        InnerContext {
            stream_index: RwLock::new(stream_index),
            backend,
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
