//! Special types that may require validation to ensure they don't contain invalid values.

use chrono::{DateTime,Utc};
use error::{InvalidDescription, InvalidReminderTime};
use std::borrow::Borrow;

/// A time at which a user should be reminded that they have something to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reminder {
    time: DateTime<Utc>,
}

impl Reminder {
    /// Creates a new reminder, verifying that the time is after the supplied `current_time`.
    pub fn new(reminder_time: DateTime<Utc>, current_time: DateTime<Utc>) -> Result<Reminder, InvalidReminderTime> {
        if reminder_time <= current_time {
            Err(InvalidReminderTime)
        } else {
            Ok(Reminder {
                time: reminder_time,
            })
        }
    }

    /// Gets the underlying time of the reminder.
    pub fn get_time(&self) -> DateTime<Utc> {
        self.time
    }
}

/// A description of a task to be done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Description {
    text: String,
}

impl Description {
    /// Creates a new description, verifying that the text provided is not empty.
    pub fn new<S: Into<String>>(text: S) -> Result<Description, InvalidDescription> {
        let text = text.into();
        if text.is_empty() {
            Err(InvalidDescription)
        } else {
            Ok(Description {
                text,
            })
        }
    }

    /// Provides access to the description text as a `&str`.
    pub fn as_str(&self) -> &str {
        self.text.borrow()
    }
}

impl Borrow<str> for Description {
    fn borrow(&self) -> &str {
        self.text.borrow()
    }
}