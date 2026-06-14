use crate::descriptor::traits::DescribeValue;
use crate::{
    call_id::CallId, interception_id::InterceptionId, method_target::MethodTarget,
    participant::Participant,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionContextQuery {
    Current,
    Relation {
        subject: CallId,
        other: CallId,
    },
    Lineage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call_id: Option<CallId>,
    },
    Children {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call_id: Option<CallId>,
    },
    Descendants {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call_id: Option<CallId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_depth: Option<u32>,
    },
}

impl DescribeValue for ExecutionContextQuery {
    fn describe_value() -> crate::descriptor::types::ValueDescriptor {
        use crate::descriptor::types::ValueDescriptor;

        ValueDescriptor::Any
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryExecutionContextParams {
    pub query: ExecutionContextQuery,
}

impl crate::descriptor::traits::DescribeParams for QueryExecutionContextParams {
    fn describe_params() -> Option<crate::descriptor::types::ParamsDescriptor> {
        Some(crate::descriptor::types::ParamsDescriptor::Object(
            crate::descriptor::types::ParamsObjectDescriptor {
                required_fields: vec![crate::descriptor::types::FieldDescriptor {
                    name: "query".to_owned(),
                    ty: ExecutionContextQuery::describe_value(),
                }],
                optional_fields: vec![],
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionContextQueryResult {
    Current(CurrentExecutionContext),
    Relation(CallRelation),
    Lineage(CallLineage),
    Children(Vec<CallId>),
    Descendants(Vec<CallId>),
}

impl crate::descriptor::traits::DescribeOk for ExecutionContextQueryResult {
    fn describe_ok() -> Option<crate::descriptor::types::ValueDescriptor> {
        Some(ExecutionContextQueryResult::describe_value())
    }
}

impl DescribeValue for ExecutionContextQueryResult {
    fn describe_value() -> crate::descriptor::types::ValueDescriptor {
        use crate::descriptor::types::ValueDescriptor;

        ValueDescriptor::Any
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentExecutionContext {
    pub origin: Participant,
    pub target: MethodTarget,
    pub call_id: CallId,
    pub root_call_id: CallId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_call_id: Option<CallId>,
    pub interception_id: InterceptionId,
}

impl DescribeValue for CurrentExecutionContext {
    fn describe_value() -> crate::descriptor::types::ValueDescriptor {
        use crate::descriptor::types::{
            FieldDescriptor, NestedObjectDescriptor, PrimitiveDescriptor, ValueDescriptor,
        };

        ValueDescriptor::Object(NestedObjectDescriptor {
            fields: vec![
                FieldDescriptor {
                    name: "origin".to_owned(),
                    ty: ValueDescriptor::Any,
                },
                FieldDescriptor {
                    name: "target".to_owned(),
                    ty: MethodTarget::describe_value(),
                },
                FieldDescriptor {
                    name: "call_id".to_owned(),
                    ty: CallId::describe_value(),
                },
                FieldDescriptor {
                    name: "root_call_id".to_owned(),
                    ty: CallId::describe_value(),
                },
                FieldDescriptor {
                    name: "parent_call_id".to_owned(),
                    ty: ValueDescriptor::OneOf(vec![
                        ValueDescriptor::Primitive(PrimitiveDescriptor::Null),
                        CallId::describe_value(),
                    ]),
                },
                FieldDescriptor {
                    name: "interception_id".to_owned(),
                    ty: InterceptionId::describe_value(),
                },
            ],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CallRelation {
    Same,
    Parent,
    Child,
    Ancestor,
    Descendant,
    Sibling,
    SameRoot,
    Unrelated,
    Unknown,
}

impl DescribeValue for CallRelation {
    fn describe_value() -> crate::descriptor::types::ValueDescriptor {
        use crate::descriptor::types::{PrimitiveDescriptor, ValueDescriptor};

        ValueDescriptor::Primitive(PrimitiveDescriptor::String)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallLineage {
    pub root_call_id: CallId,
    pub call_id: CallId,
    pub ancestors: Vec<CallId>,
}

impl DescribeValue for CallLineage {
    fn describe_value() -> crate::descriptor::types::ValueDescriptor {
        use crate::descriptor::types::{FieldDescriptor, NestedObjectDescriptor, ValueDescriptor};

        ValueDescriptor::Object(NestedObjectDescriptor {
            fields: vec![
                FieldDescriptor {
                    name: "root_call_id".to_owned(),
                    ty: CallId::describe_value(),
                },
                FieldDescriptor {
                    name: "call_id".to_owned(),
                    ty: CallId::describe_value(),
                },
                FieldDescriptor {
                    name: "ancestors".to_owned(),
                    ty: ValueDescriptor::Array(Box::new(CallId::describe_value())),
                },
            ],
        })
    }
}
