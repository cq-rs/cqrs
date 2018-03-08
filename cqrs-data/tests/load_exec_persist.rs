extern crate cqrs;
extern crate cqrs_data;
extern crate cqrs_todo_core;

use std::collections::HashMap;

use cqrs::{Aggregate};
use cqrs_data::events;
use cqrs_data::snapshots;
use cqrs_data::Since;

struct EventMap(HashMap<String, Vec<cqrs::SequencedEvent<cqrs_todo_core::Event>>>);

impl events::Source for EventMap {
    type AggregateId = str;
    type Result = Result<Option<Vec<cqrs::SequencedEvent<cqrs_todo_core::Event>>>, String>;

    fn read_events(&self, agg_id: &Self::AggregateId, since: Since) -> Self::Result {
        Ok(None)
    }
}

#[test]
fn main_test() {

}