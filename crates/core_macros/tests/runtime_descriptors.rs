use std::collections::HashMap;

use actrpc_core::{
    DescribeParams, DescribeValue,
    action::action_descriptor::{
        traits::{DescribeParams as _, DescribeValue as _},
        types::{ParamsDescriptor, PrimitiveDescriptor, ValueDescriptor},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct ValueStruct {
    enabled: bool,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeParams)]
struct ParamsStruct {
    required_name: String,
    optional_count: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
enum UnionValue {
    Text(String),
    Count(i32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, DescribeValue)]
struct MapHolder {
    labels: HashMap<String, String>,
}

#[test]
fn describe_value_struct_produces_object_descriptor() {
    let descriptor = ValueStruct::describe_value();

    match descriptor {
        ValueDescriptor::Object(obj) => {
            assert_eq!(obj.fields.len(), 2);
            assert_eq!(obj.fields[0].name, "enabled");
            assert_eq!(
                obj.fields[0].ty,
                ValueDescriptor::Primitive(PrimitiveDescriptor::Bool)
            );
            assert_eq!(obj.fields[1].name, "name");
            assert_eq!(
                obj.fields[1].ty,
                ValueDescriptor::Primitive(PrimitiveDescriptor::String)
            );
        }
        other => panic!("expected object descriptor, got {other:?}"),
    }
}

#[test]
fn describe_params_struct_separates_required_and_optional_fields() {
    let descriptor = ParamsStruct::describe_params().expect("descriptor");

    match descriptor {
        ParamsDescriptor::Object(obj) => {
            assert_eq!(obj.required_fields.len(), 1);
            assert_eq!(obj.optional_fields.len(), 1);

            assert_eq!(obj.required_fields[0].name, "required_name");
            assert_eq!(
                obj.required_fields[0].ty,
                ValueDescriptor::Primitive(PrimitiveDescriptor::String)
            );

            assert_eq!(obj.optional_fields[0].name, "optional_count");
            assert_eq!(
                obj.optional_fields[0].ty,
                ValueDescriptor::Primitive(PrimitiveDescriptor::Integer)
            );
        }
        other => panic!("expected params object descriptor, got {other:?}"),
    }
}

#[test]
fn describe_value_enum_produces_oneof_descriptor() {
    let descriptor = UnionValue::describe_value();

    match descriptor {
        ValueDescriptor::OneOf(values) => {
            assert_eq!(values.len(), 2);
            assert_eq!(
                values[0],
                ValueDescriptor::Primitive(PrimitiveDescriptor::String)
            );
            assert_eq!(
                values[1],
                ValueDescriptor::Primitive(PrimitiveDescriptor::Integer)
            );
        }
        other => panic!("expected oneof descriptor, got {other:?}"),
    }
}

#[test]
fn describe_value_map_produces_map_descriptor() {
    let descriptor = MapHolder::describe_value();

    match descriptor {
        ValueDescriptor::Object(obj) => {
            assert_eq!(obj.fields.len(), 1);
            assert_eq!(obj.fields[0].name, "labels");

            match &obj.fields[0].ty {
                ValueDescriptor::Map(inner) => {
                    assert_eq!(
                        **inner,
                        ValueDescriptor::Primitive(PrimitiveDescriptor::String)
                    );
                }
                other => panic!("expected map descriptor, got {other:?}"),
            }
        }
        other => panic!("expected object descriptor, got {other:?}"),
    }
}
