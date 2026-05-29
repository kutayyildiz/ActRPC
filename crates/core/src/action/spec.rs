use crate::{
    action::{ActionDescriptor, ActionKind},
    descriptor::traits::{DescribeOk, DescribeParams},
};
use serde::{Serialize, de::DeserializeOwned};

pub trait ActionSpec {
    type Params: Serialize + DeserializeOwned + DescribeParams;
    type Result: Serialize + DeserializeOwned + DescribeOk;

    const KIND: &'static str;

    fn action_kind() -> ActionKind {
        ActionKind::from(Self::KIND)
    }

    fn descriptor() -> ActionDescriptor {
        ActionDescriptor {
            kind: Self::action_kind(),
            params: <Self::Params as DescribeParams>::describe_params(),
            ok: <Self::Result as DescribeOk>::describe_ok(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NoParams;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NoOk;
