//! Error types for the domain and aggregate.

use std::{error, fmt};

/// The provided reminder time is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvalidReminderTime;

impl error::Error for InvalidReminderTime {
    fn description(&self) -> &str {
        "reminder time cannot be in the past"
    }
}

impl fmt::Display for InvalidReminderTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err: &error::Error = self;
        f.write_str(err.description())
    }
}

/// The provided description is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvalidDescription;

impl error::Error for InvalidDescription {
    fn description(&self) -> &str {
        "description cannot be empty"
    }
}

impl fmt::Display for InvalidDescription {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err: &error::Error = self;
        f.write_str(err.description())
    }
}

/// The command failed due to being in a state where the event could not be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandError {
    /// The aggregate was not initialized, and it should have been.
    NotInitialized,

    /// The aggregate was already created, and it should not have been.
    AlreadyCreated,
}

impl error::Error for CommandError {
    fn description(&self) -> &str {
        match *self {
            CommandError::NotInitialized => "attempt to execute command before creation",
            CommandError::AlreadyCreated => "attempt to create when already created",
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err: &error::Error = self;
        f.write_str(err.description())
    }
}
