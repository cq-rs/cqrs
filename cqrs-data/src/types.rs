use cqrs;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Since {
    BeginningOfStream,
    Version(cqrs::Version),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Precondition {
    NewStream,
    EmptyStream,
    LatestVersion(cqrs::Version),
}

