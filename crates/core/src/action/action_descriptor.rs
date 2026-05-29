use crate::{
    action::ActionKind,
    descriptor::{
        Accepts,
        types::{OkDescriptor, ParamsDescriptor},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ActionDescriptor {
    pub kind: ActionKind,
    pub params: Option<ParamsDescriptor>,
    pub ok: Option<OkDescriptor>,
}

impl Accepts for ActionDescriptor {
    fn accepts(&self, actual: &Self) -> bool {
        self.kind == actual.kind
            && self.params.accepts(&actual.params)
            && self.ok.accepts(&actual.ok)
    }
}
