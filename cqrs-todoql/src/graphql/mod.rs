use std::ops;
use std::sync::{Arc,RwLock};

use cqrs::domain::ident::UsizeIdProvider;
use juniper;

use super::{AggregateId, View, Commander};

mod schema;
pub mod endpoint;

pub struct InnerContext {
    pub stream_index: RwLock<Vec<AggregateId>>,
    pub query: View,
    pub command: Commander,
    pub id_provider: super::IdProvider,
}

impl InnerContext {
    pub fn new(stream_index: Vec<AggregateId>, query: View, command: Commander, id_provider: super::IdProvider) -> Self {
        InnerContext {
            stream_index: RwLock::new(stream_index),
            query,
            command,
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
