use std::ops;
use std::sync::Arc;

use juniper;
use r2d2_postgres::PostgresConnectionManager;
use r2d2::Pool;

mod schema;
pub mod endpoint;

pub struct InnerContext {
    pub backend: Pool<PostgresConnectionManager>,
    pub id_provider: super::IdProvider,
}

impl InnerContext {
    pub fn new(backend: Pool<PostgresConnectionManager>, id_provider: super::IdProvider) -> Self {
        InnerContext {
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
