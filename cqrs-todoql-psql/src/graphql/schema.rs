use crate::TodoStore;
use base64;
use chrono::{DateTime, Utc};
use cqrs::{Entity, EntitySink, EntitySource, EntityStore, Precondition, Version};
use cqrs_todo_core::{commands, domain, TodoAggregate, TodoId, TodoMetadata, TodoStatus};
use juniper::{FieldResult, Value, ID};

use super::Context;

#[derive(Clone, Copy, Debug)]
pub struct Query;

graphql_object!(Query: Context |&self| {
    field apiVersion() -> &str {
        "1.0"
    }

    field allTodos(&executor, first: Option<i32>, after: Option<Cursor>) -> FieldResult<TodoPage> {
        let context = executor.context();

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let total_count = store.get_entity_count()?;

        const DEFAULT_LIMIT: u32 = 100;
        const MAX_LIMIT: u32 = 1000;

        let limit = {
            let limit_raw = first.map(|i| i.max(0) as u32).unwrap_or(DEFAULT_LIMIT);
            if limit_raw == 0 {
                DEFAULT_LIMIT
            } else {
                limit_raw.min(MAX_LIMIT)
            }
        };

        let offset = {
            if let Some(Cursor(cursor)) = after {
                cursor + 1
            } else {
                0
            }
        };

        let entity_ids: Vec<_> =
            store.get_entity_ids(offset, limit)?
                .into_iter()
                .enumerate()
                .map(|(i, id)| TodoEdge {
                    agg_id: ID::from(id),
                    cursor: Cursor(i as u32 + offset)
                })
                .collect();

        Ok(TodoPage {
            total_count,
            offset,
            limit,
            edges: entity_ids,
        })
    }

    field todo(&executor, id: ID) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let id = TodoId(id.to_string());

        let entity = store.rehydrate(&id)?
            .map(|agg| TodoQL(Entity::new(id, agg)));

        Ok(entity)
    }
});

#[derive(Clone, Debug)]
struct TodoQL(Entity<TodoId, TodoAggregate>);

graphql_object!(TodoQL: Context |&self| {
    field id() -> FieldResult<ID> {
        Ok(self.0.id().to_string().into())
    }

    field description() -> FieldResult<&str> {
        Ok(self.0.aggregate().state().get_data().ok_or("uninitialized")?.description.as_str())
    }

    field reminder() -> FieldResult<Option<DateTime<Utc>>> {
        Ok(self.0.aggregate().state().get_data().ok_or("uninitialized")?.reminder.map(|r| r.get_time()))
    }

    field completed() -> FieldResult<bool> {
        Ok(self.0.aggregate().state().get_data().ok_or("uninitialized")?.status == TodoStatus::Completed)
    }

    field version() -> FieldResult<i32> {
        Ok(self.0.aggregate().version().get() as i32)
    }
});

#[derive(Debug)]
struct TodoPage {
    total_count: u64,
    offset: u32,
    limit: u32,
    edges: Vec<TodoEdge>,
}

graphql_object!(TodoPage: Context |&self| {
    field total_count() -> FieldResult<i32> {
        Ok(self.total_count as i32)
    }

    field edges() -> FieldResult<&[TodoEdge]> {
        Ok(&*self.edges)
    }

    field page_info() -> FieldResult<PageInfo> {
        Ok(PageInfo(&self))
    }
});

#[derive(Clone, Debug)]
struct TodoEdge {
    agg_id: ID,
    cursor: Cursor,
}

graphql_object!(TodoEdge: Context |&self| {
    field node(&executor) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let id = TodoId(self.agg_id.to_string());

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let entity = store.rehydrate(&id)?
            .map(|agg| TodoQL(Entity::new(id, agg)));

        Ok(entity)
    }

    field cursor() -> FieldResult<Cursor> {
        Ok(self.cursor)
    }
});

#[derive(Clone, Copy, Debug)]
struct PageInfo<'a>(&'a TodoPage);

graphql_object!(<'a> PageInfo<'a>: Context as "PageInfo" |&self| {
    field has_next_page(&executor) -> bool {
        self.0.edges.len() as u64 + u64::from(self.0.offset) < self.0.total_count
    }

    field end_cursor() -> Option<Cursor> {
        self.0.edges.last().map(|e| e.cursor)
    }
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cursor(u32);

impl ToString for Cursor {
    fn to_string(&self) -> String {
        base64::encode(&self.0.to_string())
    }
}

graphql_scalar!(Cursor {
    description: "An opaque identifier, represented as a location in an enumeration"

    resolve(&self) -> Value {
        Value::scalar(self.to_string())
    }

    from_input_value(v: &InputValue) -> Option<Cursor> {
        v.as_scalar_value::<String>()
            .and_then(|v| base64::decode(v).ok())
            .and_then(|v| String::from_utf8_lossy(&v).parse::<u32>().ok())
            .map(Cursor)
    }

    from_str<'a>(value: ScalarToken<'a>) -> juniper::ParseScalarResult<'a> {
        <String as juniper::ParseScalarValue>::from_str(value)
    }
});

#[derive(Clone, Copy, Debug)]
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


        let command = commands::CreateTodo {
            description,
            reminder,
        };


        let new_id = context.id_provider.new_id();

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let metadata = TodoMetadata {
            initiated_by: String::from("graphql"),
        };

        let aggregate = store.exec_and_persist(
            &new_id,
            Default::default(),
            command,
            Some(Precondition::New),
            metadata,
        )?;

        Ok(TodoQL(Entity::new(new_id, aggregate)))
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

        let new_description = domain::Description::new(text)?;

        let command = commands::UpdateDescription { new_description };

        let id = TodoId(self.0.to_string());

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let metadata = TodoMetadata {
            initiated_by: String::from("graphql"),
        };

        let entity = store.load_exec_and_persist(
            &id,
            command,
            Some(precondition),
            metadata,
        )?.map(move |agg| Entity::new(id, agg));

        Ok(entity.map(TodoQL))
    }

    field set_reminder(&executor, time: DateTime<Utc>, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let new_reminder = domain::Reminder::new(time, Utc::now())?;

        let command = commands::SetReminder { new_reminder };

        let id = TodoId(self.0.to_string());

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let metadata = TodoMetadata {
            initiated_by: String::from("graphql"),
        };

        let entity = store.load_exec_and_persist(
            &id,
            command,
            Some(precondition),
            metadata,
        )?.map(move |agg| Entity::new(id, agg));

        Ok(entity.map(TodoQL))
    }

    field cancel_reminder(&executor, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let command = commands::CancelReminder;

        let id = TodoId(self.0.to_string());

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let metadata = TodoMetadata {
            initiated_by: String::from("graphql"),
        };

        let entity = store.load_exec_and_persist(
            &id,
            command,
            Some(precondition),
            metadata,
        )?.map(move |agg| Entity::new(id, agg));

        Ok(entity.map(TodoQL))
    }

    field toggle(&executor, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let command = commands::ToggleCompletion;

        let id = TodoId(self.0.to_string());

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let metadata = TodoMetadata {
            initiated_by: String::from("graphql"),
        };

        let entity = store.load_exec_and_persist(
            &id,
            command,
            Some(precondition),
            metadata,
        )?.map(move |agg| Entity::new(id, agg));

        Ok(entity.map(TodoQL))
    }

    field reset(&executor, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let command = commands::ResetCompleted;

        let id = TodoId(self.0.to_string());

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let metadata = TodoMetadata {
            initiated_by: String::from("graphql"),
        };

        let entity = store.load_exec_and_persist(
            &id,
            command,
            Some(precondition),
            metadata,
        )?.map(move |agg| Entity::new(id, agg));

        Ok(entity.map(TodoQL))
    }

    field complete(&executor, expected_version: Option<i32>) -> FieldResult<Option<TodoQL>> {
        let context = executor.context();

        let precondition = expect_exists_or(expected_version);

        let command = commands::MarkCompleted;

        let id = TodoId(self.0.to_string());

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let metadata = TodoMetadata {
            initiated_by: String::from("graphql"),
        };

        let entity = store.load_exec_and_persist(
            &id,
            command,
            Some(precondition),
            metadata,
        )?.map(move |agg| Entity::new(id, agg));

        Ok(entity.map(TodoQL))
    }
});
