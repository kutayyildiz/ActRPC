use crate::{
    action::{NoOk, NoParams},
    descriptor::{
        traits::{DescribeOk, DescribeParams, DescribeValue},
        types::{
            FieldDescriptor, NestedObjectDescriptor, OkDescriptor, ParamsDescriptor,
            PrimitiveDescriptor, ValueDescriptor,
        },
    },
    json_rpc::{
        JsonRpcError, JsonRpcErrorResponse, JsonRpcId, JsonRpcParams, JsonRpcResponse,
        JsonRpcSuccessResponse,
    },
};

macro_rules! impl_integer_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl DescribeValue for $ty {
                fn describe_value() -> ValueDescriptor {
                    ValueDescriptor::Primitive(PrimitiveDescriptor::Integer)
                }
            }

            impl DescribeParams for $ty {
                fn describe_params() -> Option<ParamsDescriptor> {
                    Some(ParamsDescriptor::Value(<Self as DescribeValue>::describe_value()))
                }
            }

            impl DescribeOk for $ty {
                fn describe_ok() -> Option<OkDescriptor> {
                    Some(<Self as DescribeValue>::describe_value())
                }
            }
        )*
    };
}

macro_rules! impl_number_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl DescribeValue for $ty {
                fn describe_value() -> ValueDescriptor {
                    ValueDescriptor::Primitive(PrimitiveDescriptor::Number)
                }
            }

            impl DescribeParams for $ty {
                fn describe_params() -> Option<ParamsDescriptor> {
                    Some(ParamsDescriptor::Value(<Self as DescribeValue>::describe_value()))
                }
            }

            impl DescribeOk for $ty {
                fn describe_ok() -> Option<OkDescriptor> {
                    Some(<Self as DescribeValue>::describe_value())
                }
            }
        )*
    };
}

impl DescribeValue for bool {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::Primitive(PrimitiveDescriptor::Bool)
    }
}

impl DescribeParams for bool {
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl DescribeOk for bool {
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}

impl DescribeValue for String {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::Primitive(PrimitiveDescriptor::String)
    }
}

impl DescribeParams for String {
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl DescribeOk for String {
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}

impl DescribeValue for str {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::Primitive(PrimitiveDescriptor::String)
    }
}

impl_integer_value!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);
impl_number_value!(f32, f64);

impl<T> DescribeValue for Vec<T>
where
    T: DescribeValue,
{
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::Array(Box::new(T::describe_value()))
    }
}

impl<T> DescribeParams for Vec<T>
where
    T: DescribeValue,
{
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl<T> DescribeOk for Vec<T>
where
    T: DescribeValue,
{
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}

impl DescribeValue for serde_json::Value {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::Any
    }
}

impl DescribeParams for serde_json::Value {
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(ValueDescriptor::Any))
    }
}

impl DescribeOk for serde_json::Value {
    fn describe_ok() -> Option<OkDescriptor> {
        Some(ValueDescriptor::Any)
    }
}

impl DescribeValue for JsonRpcId {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::OneOf(vec![
            ValueDescriptor::Primitive(PrimitiveDescriptor::String),
            ValueDescriptor::Primitive(PrimitiveDescriptor::Number),
            ValueDescriptor::Primitive(PrimitiveDescriptor::Null),
        ])
    }
}

impl DescribeParams for JsonRpcId {
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl DescribeOk for JsonRpcId {
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}

impl DescribeValue for JsonRpcParams {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::OneOf(vec![
            ValueDescriptor::Array(Box::new(ValueDescriptor::Any)),
            ValueDescriptor::Map(Box::new(ValueDescriptor::Any)),
        ])
    }
}

impl DescribeParams for JsonRpcParams {
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl DescribeOk for JsonRpcParams {
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}

impl DescribeValue for JsonRpcError {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::Object(NestedObjectDescriptor {
            fields: vec![
                FieldDescriptor {
                    name: "code".to_string(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::Integer),
                },
                FieldDescriptor {
                    name: "message".to_string(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::String),
                },
                FieldDescriptor {
                    name: "data".to_string(),
                    ty: ValueDescriptor::Any,
                },
            ],
        })
    }
}

impl DescribeParams for JsonRpcError {
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl DescribeOk for JsonRpcError {
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}

impl DescribeParams for NoParams {
    fn describe_params() -> Option<ParamsDescriptor> {
        None
    }
}

impl DescribeOk for NoOk {
    fn describe_ok() -> Option<OkDescriptor> {
        None
    }
}

impl<T> DescribeValue for std::collections::HashSet<T>
where
    T: DescribeValue,
{
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::Array(Box::new(T::describe_value()))
    }
}

impl<T> DescribeParams for std::collections::HashSet<T>
where
    T: DescribeValue,
{
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl<T> DescribeOk for std::collections::HashSet<T>
where
    T: DescribeValue,
{
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}

impl DescribeValue for JsonRpcSuccessResponse {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::Object(NestedObjectDescriptor {
            fields: vec![
                FieldDescriptor {
                    name: "jsonrpc".to_string(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::String),
                },
                FieldDescriptor {
                    name: "id".to_string(),
                    ty: <JsonRpcId as DescribeValue>::describe_value(),
                },
                FieldDescriptor {
                    name: "result".to_string(),
                    ty: ValueDescriptor::Any,
                },
            ],
        })
    }
}

impl DescribeParams for JsonRpcSuccessResponse {
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl DescribeOk for JsonRpcSuccessResponse {
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}

impl DescribeValue for JsonRpcErrorResponse {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::Object(NestedObjectDescriptor {
            fields: vec![
                FieldDescriptor {
                    name: "jsonrpc".to_string(),
                    ty: ValueDescriptor::Primitive(PrimitiveDescriptor::String),
                },
                FieldDescriptor {
                    name: "id".to_string(),
                    ty: <JsonRpcId as DescribeValue>::describe_value(),
                },
                FieldDescriptor {
                    name: "error".to_string(),
                    ty: <JsonRpcError as DescribeValue>::describe_value(),
                },
            ],
        })
    }
}

impl DescribeParams for JsonRpcErrorResponse {
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl DescribeOk for JsonRpcErrorResponse {
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}

impl DescribeValue for JsonRpcResponse {
    fn describe_value() -> ValueDescriptor {
        ValueDescriptor::OneOf(vec![
            <JsonRpcSuccessResponse as DescribeValue>::describe_value(),
            <JsonRpcErrorResponse as DescribeValue>::describe_value(),
        ])
    }
}

impl DescribeParams for JsonRpcResponse {
    fn describe_params() -> Option<ParamsDescriptor> {
        Some(ParamsDescriptor::Value(
            <Self as DescribeValue>::describe_value(),
        ))
    }
}

impl DescribeOk for JsonRpcResponse {
    fn describe_ok() -> Option<OkDescriptor> {
        Some(<Self as DescribeValue>::describe_value())
    }
}
