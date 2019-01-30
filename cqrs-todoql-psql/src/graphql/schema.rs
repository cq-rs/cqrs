use crate::TodoStore;
use base64;
use cqrs::{Entity, Precondition, Version, EntityStore, EntitySink, EntitySource};
use cqrs_todo_core::{domain, commands, TodoAggregate, TodoId, TodoStatus, TodoMetadata};
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

        let reader = context.stream_index.read();
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
                    id.0.as_str() != &**first_id
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
                agg_id: ID::from(agg_id.0.clone()),
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

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let id = TodoId(id.to_string());

        let entity = store.rehydrate(&id)?
            .map(|agg| TodoQL(agg.into_entity_with_id(id)));

        Ok(entity)
    }
});

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

        let id = TodoId(self.agg_id.to_string());

        let conn = context.backend.get()?;
        let store = TodoStore::new(&*conn);

        let entity = store.rehydrate(&id)?
            .map(|agg| TodoQL(agg.into_entity_with_id(id)));

        Ok(entity)
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

        let entity = store.exec_and_persist(
            &new_id,
            Default::default(),
            command,
            Some(Precondition::New),
            metadata,
        )?.into_entity_with_id(new_id.clone());

        context.stream_index.write().push(new_id);

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
        )?.map(move |agg| agg.into_entity_with_id(id));

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
        )?.map(move |agg| agg.into_entity_with_id(id));

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
        )?.map(move |agg| agg.into_entity_with_id(id));

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
        )?.map(move |agg| agg.into_entity_with_id(id));

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
        )?.map(move |agg| agg.into_entity_with_id(id));

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
        )?.map(move |agg| agg.into_entity_with_id(id));

        Ok(entity.map(TodoQL))
    }
});
