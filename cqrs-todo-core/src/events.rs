//! Events in the to-do system

use crate::domain;
use cqrs_core::Event;
use serde::{Deserialize, Serialize};

/// A to-do was created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Created {
    /// The initial description assigned to the to-do item.
    pub initial_description: domain::Description,
}

impl Event for Created {
    fn event_type(&self) -> &'static str {
        "todo_created"
    }
}

/// The description was updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescriptionUpdated {
    /// The new description assigned to the to-do item.
    pub new_description: domain::Description,
}

impl Event for DescriptionUpdated {
    fn event_type(&self) -> &'static str {
        "todo_description_updated"
    }
}

/// The reminder was updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderUpdated {
    /// The new reminder assigned to the to-do item.
    pub new_reminder: Option<domain::Reminder>,
}

impl Event for ReminderUpdated {
    fn event_type(&self) -> &'static str {
        "todo_reminder_updated"
    }
}

/// The activity was completed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Completed {}

impl Event for Completed {
    fn event_type(&self) -> &'static str {
        "todo_completed"
    }
}

/// The activity's completion was undone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Uncompleted {}

impl Event for Uncompleted {
    fn event_type(&self) -> &'static str {
        "todo_uncompleted"
    }
}
