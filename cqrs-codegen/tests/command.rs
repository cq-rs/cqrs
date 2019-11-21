#![allow(dead_code)]

use cqrs::{Command as _, Version};
use cqrs_codegen::{Aggregate, Command};

#[derive(Aggregate, Default)]
#[aggregate(type = "aggregate")]
struct Aggregate {
    id: i32,
}

#[test]
fn derives_for_struct() {
    #[derive(Command)]
    #[command(aggregate = "Aggregate")]
    struct TestCommand {
        id: i32,
        version: Version,
    }

    let command = TestCommand {
        id: 0,
        version: Version::Initial,
    };

    assert_eq!(command.aggregate_id(), None);
    assert_eq!(command.expected_version(), None);
}

#[test]
fn derives_for_struct_with_id() {
    #[derive(Command)]
    #[command(aggregate = "Aggregate")]
    struct TestCommand {
        #[command(id)]
        id: i32,
        version: Version,
    }

    let command = TestCommand {
        id: 0,
        version: Version::Initial,
    };

    assert_eq!(command.aggregate_id(), Some(&0));
    assert_eq!(command.expected_version(), None);
}

#[test]
fn derives_for_struct_with_version() {
    #[derive(Command)]
    #[command(aggregate = "Aggregate")]
    struct TestCommand {
        id: i32,
        #[command(version)]
        version: Version,
    }

    let command = TestCommand {
        id: 0,
        version: Version::Initial,
    };

    assert_eq!(command.aggregate_id(), None);
    assert_eq!(command.expected_version(), Some(Version::Initial));
}

#[test]
fn derives_for_struct_with_id_and_version() {
    #[derive(Command)]
    #[command(aggregate = "Aggregate")]
    struct TestCommand {
        #[command(id)]
        id: i32,
        #[command(version)]
        version: Version,
    }

    let command = TestCommand {
        id: 0,
        version: Version::Initial,
    };

    assert_eq!(command.aggregate_id(), Some(&0));
    assert_eq!(command.expected_version(), Some(Version::Initial));
}

#[test]
fn derives_for_tuple_struct() {
    #[derive(Command)]
    #[command(aggregate = "Aggregate")]
    struct TestCommand(i32, Version);

    let command = TestCommand(0, Version::Initial);

    assert_eq!(command.aggregate_id(), None);
    assert_eq!(command.expected_version(), None);
}

#[test]
fn derives_for_tuple_struct_with_id() {
    #[derive(Command)]
    #[command(aggregate = "Aggregate")]
    struct TestCommand(#[command(id)] i32, Version);

    let command = TestCommand(0, Version::Initial);

    assert_eq!(command.aggregate_id(), Some(&0));
    assert_eq!(command.expected_version(), None);
}

#[test]
fn derives_for_tuple_struct_with_version() {
    #[derive(Command)]
    #[command(aggregate = "Aggregate")]
    struct TestCommand(i32, #[command(version)] Version);

    let command = TestCommand(0, Version::Initial);

    assert_eq!(command.aggregate_id(), None);
    assert_eq!(command.expected_version(), Some(Version::Initial));
}

#[test]
fn derives_for_tuple_struct_with_id_and_version() {
    #[derive(Command)]
    #[command(aggregate = "Aggregate")]
    struct TestCommand(#[command(id)] i32, #[command(version)] Version);

    let command = TestCommand(0, Version::Initial);

    assert_eq!(command.aggregate_id(), Some(&0));
    assert_eq!(command.expected_version(), Some(Version::Initial));
}
