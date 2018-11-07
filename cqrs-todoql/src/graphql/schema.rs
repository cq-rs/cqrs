use base64;
use cqrs::{Aggregate, Precondition, StateSnapshot, Version};
use cqrs::Entity;
use cqrs::{EventSink, SnapshotSink};
use cqrs_todo_core::{domain, TodoAggregate, TodoStatus, Command};
use chrono::{DateTime, Utc};
use juniper::{ID, FieldResult, Value};

use super::Context;

pub struct Query;

graphql_object!(Query: Context |&self| {
    field apiVersion() -> &str {
        "1.0"
    }

    field allTodos(&executor, first: Option<ID>, after: Option<Cursor>) -> FieldResult<TodoPage> {
        let context = executor.context();

        let reader = context.stream_index.read().unwrap();
        let len = reader.len();

        let mut skip =
            if let Some(Cursor(cursor)) = after {
                cursor
            } else { 0 };

        let mut also_skipped = 0;
        let iterator =
            reader.iter().skip_while(move |id| {
                if let Some(ref first_id) = first {
                    also_skipped += 1;
                    **id != **first_id
                } else {
                    false
                }
            }).skip(skip);

        let mut items = Vec::default();
        let mut end_cursor = None;
        const MAX_PAGE_SIZE: usize = 10;
        for agg_id in iterator.take(MAX_PAGE_SIZE) {
            skip += 1;
            let cursor = Cursor(skip);
            items.push(TodoEdge {
                agg_id: ID::from(agg_id.clone()),
                cursor,
            });
            end_cursor = Some(cursor);
        }

        Ok(TodoPage {
            total_count: len,
            page_info: PageInfo {
                has_next_page: items.len() != 0 && skip + also_skipped < len,
                end_cursor,
            },
            edges: items,
        })
    }

    field todo(&executor, id: ID) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let entity = Entity::rehydrate(id.to_string(), &context.event_db, &context.state_db)?;

        Ok(entity.map(TodoQL))
    }
});

struct TodoQL(Entity<'static, TodoAggregate>);

graphql_object!(TodoQL: Context |&self| {
    field id() -> FieldResult<ID> {
        Ok(self.0.id().to_string().into())
    }

    field description() -> FieldResult<&str> {
        Ok(self.0.aggregate().get_data()?.description.as_str())
    }

    field reminder() -> FieldResult<Option<DateTime<Utc>>> {
        Ok(self.0.aggregate().get_data()?.reminder.map(|r| r.get_time()))
    }

    field completed() -> FieldResult<bool> {
        Ok(self.0.aggregate().get_data()?.status == TodoStatus::Completed)
    }

    field version() -> FieldResult<i32> {
        Ok(self.0.version().get() as i32)
    }
});

struct TodoPage {
    total_count: usize,
    edges: Vec<TodoEdge>,
    page_info: PageInfo,
}

graphql_object!(TodoPage: Context |&self| {
    field total_count() -> FieldResult<i32> {
        Ok(self.total_count as i32)
    }

    field edges() -> FieldResult<&[TodoEdge]> {
        Ok(&*self.edges)
    }

    field page_info() -> FieldResult<&PageInfo> {
        Ok(&self.page_info)
    }
});

struct TodoEdge {
    agg_id: ID,
    cursor: Cursor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Cursor(usize);

impl ToString for Cursor {
    fn to_string(&self) -> String {
        base64::encode(&self.0.to_string())
    }
}

graphql_scalar!(Cursor {
    description: "An opaque identifier, represented as a location in an enumeration"

    resolve(&self) -> Value {
        Value::string(self.to_string())
    }

    from_input_value(v: &InputValue) -> Option<Cursor> {
        v.as_string_value()
            .and_then(|v| base64::decode(v).ok())
            .and_then(|v| String::from_utf8_lossy(&v).parse::<usize>().ok())
            .map(Cursor)
    }
});

graphql_object!(TodoEdge: Context |&self| {
    field node(&executor) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let id = self.agg_id.to_string();

        let entity = Entity::rehydrate(id, &context.event_db, &context.state_db)?;

        Ok(entity.map(TodoQL))
    }

    field cursor() -> FieldResult<Cursor> {
        Ok(self.cursor)
    }
});

#[derive(GraphQLObject)]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<Cursor>,
}

pub struct Mutations;

graphql_object!(Mutations: Context |&self| {
    field todo(id: ID) -> FieldResult<TodoMutQL> {
        Ok(TodoMutQL(id))
    }

    field new_todo(&executor, text: String, reminder_time: Option<DateTime<Utc>>) -> FieldResult<TodoQL> {
        let context = executor.context();

        let description = domain::Description::new(text)?;
        let reminder =
            if let Some(time) = reminder_time {
                Some(domain::Reminder::new(time, Utc::now())?)
            } else { None };


        let command = Command::Create(description, reminder);

        let new_id = context.id_provider.new_id();

        let mut entity: Entity<TodoAggregate> = Entity::from_default(new_id.clone());

        let events = entity.aggregate().execute(command)?;

        context.event_db.append_events(new_id.as_ref(), &events, Some(Precondition::New))?;
        entity.apply_events(events);

        context.stream_index.write().unwrap().push(new_id.clone());

        Ok(TodoQL(entity))
    }

});

struct TodoMutQL(ID);

fn expect_exists_or(expected_version: Option<i32>) -> Precondition {
    expected_version
        .map(|i| Version::new(i as u64))
        .map(Precondition::ExpectedVersion)
        .unwrap_or(Precondition::Exists)
}

graphql_object!(TodoMutQL: Context |&self| {
    field set_description(&executor, text: String, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let description = domain::Description::new(text)?;

        let command = Command::UpdateText(description);

        let id = self.0.to_string();

        let mut entity = Entity::rehydrate(id, &context.event_db, &context.state_db)?.ok_or("Entity not found")?;

        let events = entity.aggregate().execute(command)?;

        context.event_db.append_events(entity.id().as_ref(), &events, Some(precondition))?;
        entity.apply_events(events);

        if entity.version() - entity.snapshot_version() > 10 {
            context.state_db.persist_snapshot(entity.id().as_ref(), StateSnapshot {snapshot: entity.aggregate().to_owned(), version: entity.version()})?;
        }

        Ok(Some(TodoQL(entity)))
    }

    field set_reminder(&executor, time: DateTime<Utc>, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let reminder = domain::Reminder::new(time, Utc::now())?;

        let command = Command::SetReminder(reminder);

        let id = self.0.to_string();

        let mut entity = Entity::rehydrate(id.to_owned(), &context.event_db, &context.state_db)?.ok_or("Entity not found")?;

        let events = entity.aggregate().execute(command)?;

        context.event_db.append_events(entity.id().as_ref(), &events, Some(precondition))?;
        entity.apply_events(events);

        if entity.version() - entity.snapshot_version() > 10 {
            context.state_db.persist_snapshot(entity.id().as_ref(), StateSnapshot {snapshot: entity.aggregate().to_owned(), version: entity.version()})?;
        }

        Ok(Some(TodoQL(entity)))
    }

    field cancel_reminder(&executor, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let command = Command::CancelReminder;

        let id = self.0.to_string();

        let mut entity = Entity::rehydrate(id.to_owned(), &context.event_db, &context.state_db)?.ok_or("Entity not found")?;

        let events = entity.aggregate().execute(command)?;

        context.event_db.append_events(entity.id().as_ref(), &events, Some(precondition))?;
        entity.apply_events(events);

        if entity.version() - entity.snapshot_version() > 10 {
            context.state_db.persist_snapshot(entity.id().as_ref(), StateSnapshot {snapshot: entity.aggregate().to_owned(), version: entity.version()})?;
        }

        Ok(Some(TodoQL(entity)))
    }

    field toggle(&executor, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let command = Command::ToggleCompletion;

        let id = self.0.to_string();

        let mut entity = Entity::rehydrate(id.to_owned(), &context.event_db, &context.state_db)?.ok_or("Entity not found")?;

        let events = entity.aggregate().execute(command)?;

        context.event_db.append_events(entity.id().as_ref(), &events, Some(precondition))?;
        entity.apply_events(events);

        if entity.version() - entity.snapshot_version() > 10 {
            context.state_db.persist_snapshot(entity.id().as_ref(), StateSnapshot {snapshot: entity.aggregate().to_owned(), version: entity.version()})?;
        }

        Ok(Some(TodoQL(entity)))
    }

    field reset(&executor, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let command = Command::ResetCompleted;

        let id = self.0.to_string();

        let mut entity = Entity::rehydrate(id.to_owned(), &context.event_db, &context.state_db)?.ok_or("Entity not found")?;

        let events = entity.aggregate().execute(command)?;

        context.event_db.append_events(entity.id().as_ref(), &events, Some(precondition))?;
        entity.apply_events(events);

        if entity.version() - entity.snapshot_version() > 10 {
            context.state_db.persist_snapshot(entity.id().as_ref(), StateSnapshot {snapshot: entity.aggregate().to_owned(), version: entity.version()})?;
        }

        Ok(Some(TodoQL(entity)))
    }

    field complete(&executor, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let command = Command::MarkCompleted;

        let id = self.0.to_string();

        let mut entity = Entity::rehydrate(id.to_owned(), &context.event_db, &context.state_db)?.ok_or("Entity not found")?;

        let events = entity.aggregate().execute(command)?;

        context.event_db.append_events(entity.id().as_ref(), &events, Some(precondition))?;
        entity.apply_events(events);

        if entity.version() - entity.snapshot_version() > 10 {
            context.state_db.persist_snapshot(entity.id().as_ref(), StateSnapshot {snapshot: entity.aggregate().to_owned(), version: entity.version()})?;
        }

        Ok(Some(TodoQL(entity)))
    }
});
