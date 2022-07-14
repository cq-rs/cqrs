#![allow(dead_code)]

use cqrs::Aggregate as _;
use cqrs_codegen::Aggregate;

#[test]
fn derives_for_struct_with_inferred_id_field() {
    #[derive(Default, Aggregate)]
    #[aggregate(type = "test.aggregate")]
    struct TestAggregate {
        id: i32,
        field: i32,
    }

    assert_eq!(TestAggregate::AGGREGATE_TYPE, "test.aggregate");
    assert_eq!(TestAggregate::default().aggregate_type(), "test.aggregate");
    assert_eq!(*TestAggregate::default().id(), 0);
}

#[test]
fn derives_for_struct_with_explicit_id_field() {
    #[derive(Default, Aggregate)]
    #[aggregate(type = "test.aggregate")]
    struct TestAggregate {
        #[aggregate(id)]
        explicit_id: i32,
        field: i32,
    }

    assert_eq!(TestAggregate::AGGREGATE_TYPE, "test.aggregate");
    assert_eq!(TestAggregate::default().aggregate_type(), "test.aggregate");
    assert_eq!(*TestAggregate::default().id(), 0);
}

#[test]
fn derives_for_struct_with_redundantly_explicit_id_field() {
    #[derive(Default, Aggregate)]
    #[aggregate(type = "test.aggregate")]
    struct TestAggregate {
        #[aggregate(id)]
        id: i32,
        field: i32,
    }

    assert_eq!(TestAggregate::AGGREGATE_TYPE, "test.aggregate");
    assert_eq!(TestAggregate::default().aggregate_type(), "test.aggregate");
    assert_eq!(*TestAggregate::default().id(), 0);
}

#[test]
fn derives_for_tuple_struct_with_explicit_id_field() {
    #[derive(Default, Aggregate)]
    #[aggregate(type = "test.aggregate")]
    struct TestAggregate(#[aggregate(id)] i32, i32);

    assert_eq!(TestAggregate::AGGREGATE_TYPE, "test.aggregate");
    assert_eq!(TestAggregate::default().aggregate_type(), "test.aggregate");
    assert_eq!(*TestAggregate::default().id(), 0);
}

#[test]
fn derives_for_generic_struct_with_inferred_id_field() {
    #[derive(Default, Aggregate)]
    #[aggregate(type = "test.aggregate")]
    struct TestAggregate<T: Default> {
        id: T,
        field: T,
    }

    assert_eq!(TestAggregate::<i32>::AGGREGATE_TYPE, "test.aggregate");
    assert_eq!(
        TestAggregate::<i32>::default().aggregate_type(),
        "test.aggregate"
    );
    assert_eq!(*TestAggregate::<i32>::default().id(), 0);
}

#[test]
fn derives_for_generic_struct_with_explicit_id_field() {
    #[derive(Default, Aggregate)]
    #[aggregate(type = "test.aggregate")]
    struct TestAggregate<T: Default> {
        #[aggregate(id)]
        explicit_id: T,
        field: T,
    }

    assert_eq!(TestAggregate::<i32>::AGGREGATE_TYPE, "test.aggregate");
    assert_eq!(
        TestAggregate::<i32>::default().aggregate_type(),
        "test.aggregate"
    );
    assert_eq!(*TestAggregate::<i32>::default().id(), 0);
}

#[test]
fn derives_for_generic_struct_with_redundantly_explicit_id_field() {
    #[derive(Default, Aggregate)]
    #[aggregate(type = "test.aggregate")]
    struct TestAggregate<T: Default> {
        #[aggregate(id)]
        id: T,
        field: T,
    }

    assert_eq!(TestAggregate::<i32>::AGGREGATE_TYPE, "test.aggregate");
    assert_eq!(
        TestAggregate::<i32>::default().aggregate_type(),
        "test.aggregate"
    );
    assert_eq!(*TestAggregate::<i32>::default().id(), 0);
}

#[test]
fn derives_for_generic_tuple_struct_with_explicit_id_field() {
    #[derive(Default, Aggregate)]
    #[aggregate(type = "test.aggregate")]
    struct TestAggregate<T: Default = ()>(#[aggregate(id)] T, T);

    assert_eq!(TestAggregate::<i32>::AGGREGATE_TYPE, "test.aggregate");
    assert_eq!(
        TestAggregate::<i32>::default().aggregate_type(),
        "test.aggregate"
    );
    assert_eq!(*TestAggregate::<i32>::default().id(), 0);
}

#[test]
fn derives_for_struct_with_const_generic_parameters() {
    #[derive(Default, Aggregate)]
    #[aggregate(type = "test.aggregate")]
    struct TestAggregate<const T: u8 = 0>(#[aggregate(id)] i32);

    assert_eq!(TestAggregate::<1>::AGGREGATE_TYPE, "test.aggregate");
    assert_eq!(
        TestAggregate::<1>::default().aggregate_type(),
        "test.aggregate"
    );
    assert_eq!(*TestAggregate::<1>::default().id(), 0);
}
