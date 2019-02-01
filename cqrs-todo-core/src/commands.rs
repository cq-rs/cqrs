//! Commands for the to-do system.

use crate::{
    domain,
    error::CommandError,
    events::{Completed, Created, DescriptionUpdated, ReminderUpdated, Uncompleted},
    TodoAggregate, TodoEvent, TodoStatus,
};
use arrayvec::ArrayVec;
use cqrs_core::AggregateCommand;

/// Create a new to-do item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTodo {
    /// The initial description to use.
    pub description: domain::Description,

    /// The initial reminder to set. `None` indicates no initial reminder.
    pub reminder: Option<domain::Reminder>,
}

impl AggregateCommand<TodoAggregate> for CreateTodo {
    type Error = CommandError;
    type Event = TodoEvent;
    type Events = ArrayVec<[Self::Event; 2]>;

    fn execute_on(self, aggregate: &TodoAggregate) -> Result<Self::Events, Self::Error> {
        if let TodoAggregate::Created(_) = aggregate {
            return Err(CommandError::AlreadyCreated);
        }

        let mut events = ArrayVec::new();
        events.push(TodoEvent::Created(Created {
            initial_description: self.description,
        }));
        if let Some(reminder) = self.reminder {
            events.push(TodoEvent::ReminderUpdated(ReminderUpdated {
                new_reminder: Some(reminder),
            }));
        }
        Ok(events)
    }
}

/// Update the description of a to-do item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateDescription {
    /// The new description to use.
    pub new_description: domain::Description,
}

impl AggregateCommand<TodoAggregate> for UpdateDescription {
    type Error = CommandError;
    type Event = TodoEvent;
    type Events = ArrayVec<[Self::Event; 1]>;

    fn execute_on(self, aggregate: &TodoAggregate) -> Result<Self::Events, Self::Error> {
        if let TodoAggregate::Created(ref data) = aggregate {
            if data.description != self.new_description {
                let mut events = ArrayVec::new();
                events.push(TodoEvent::DescriptionUpdated(DescriptionUpdated {
                    new_description: self.new_description,
                }));
                Ok(events)
            } else {
                Ok(ArrayVec::new())
            }
        } else {
            return Err(CommandError::NotInitialized);
        }
    }
}

/// Set the reminder for a to-do item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetReminder {
    /// The new reminder to use.
    pub new_reminder: domain::Reminder,
}

impl AggregateCommand<TodoAggregate> for SetReminder {
    type Error = CommandError;
    type Event = TodoEvent;
    type Events = ArrayVec<[Self::Event; 1]>;

    fn execute_on(self, aggregate: &TodoAggregate) -> Result<Self::Events, Self::Error> {
        if let TodoAggregate::Created(ref data) = aggregate {
            let new_reminder = Some(self.new_reminder);

            if data.reminder != new_reminder {
                let mut events = ArrayVec::new();
                events.push(TodoEvent::ReminderUpdated(ReminderUpdated { new_reminder }));
                Ok(events)
            } else {
                Ok(ArrayVec::new())
            }
        } else {
            return Err(CommandError::NotInitialized);
        }
    }
}

/// Cancel any reminder set for a to-do item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelReminder;

impl AggregateCommand<TodoAggregate> for CancelReminder {
    type Error = CommandError;
    type Event = TodoEvent;
    type Events = ArrayVec<[Self::Event; 1]>;

    fn execute_on(self, aggregate: &TodoAggregate) -> Result<Self::Events, Self::Error> {
        if let TodoAggregate::Created(ref data) = aggregate {
            if data.reminder.is_some() {
                let mut events = ArrayVec::new();
                events.push(TodoEvent::ReminderUpdated(ReminderUpdated {
                    new_reminder: None,
                }));
                Ok(events)
            } else {
                Ok(ArrayVec::new())
            }
        } else {
            return Err(CommandError::NotInitialized);
        }
    }
}

/// Toggle the completion status of a to-do item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleCompletion;

impl AggregateCommand<TodoAggregate> for ToggleCompletion {
    type Error = CommandError;
    type Event = TodoEvent;
    type Events = ArrayVec<[Self::Event; 1]>;

    fn execute_on(self, aggregate: &TodoAggregate) -> Result<Self::Events, Self::Error> {
        if let TodoAggregate::Created(ref data) = aggregate {
            let mut events = ArrayVec::new();
            match data.status {
                TodoStatus::Completed => events.push(TodoEvent::Uncompleted(Uncompleted {})),
                TodoStatus::NotCompleted => events.push(TodoEvent::Completed(Completed {})),
            }
            Ok(events)
        } else {
            return Err(CommandError::NotInitialized);
        }
    }
}

/// Mark a to-do item as completed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkCompleted;

impl AggregateCommand<TodoAggregate> for MarkCompleted {
    type Error = CommandError;
    type Event = TodoEvent;
    type Events = ArrayVec<[Self::Event; 1]>;

    fn execute_on(self, aggregate: &TodoAggregate) -> Result<Self::Events, Self::Error> {
        if let TodoAggregate::Created(ref data) = aggregate {
            let mut events = ArrayVec::new();
            if data.status == TodoStatus::NotCompleted {
                events.push(TodoEvent::Completed(Completed {}))
            }
            Ok(events)
        } else {
            return Err(CommandError::NotInitialized);
        }
    }
}

/// Mark a to-do item as not completed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetCompleted;

impl AggregateCommand<TodoAggregate> for ResetCompleted {
    type Error = CommandError;
    type Event = TodoEvent;
    type Events = ArrayVec<[Self::Event; 1]>;

    fn execute_on(self, aggregate: &TodoAggregate) -> Result<Self::Events, Self::Error> {
        if let TodoAggregate::Created(ref data) = aggregate {
            let mut events = ArrayVec::new();
            if data.status == TodoStatus::Completed {
                events.push(TodoEvent::Uncompleted(Uncompleted {}))
            }
            Ok(events)
        } else {
            return Err(CommandError::NotInitialized);
        }
    }
}
