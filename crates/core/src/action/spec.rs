use serde::{Serialize, de::DeserializeOwned};

use crate::action::ActionKind;

pub trait ActionSpec {
    type Params: Serialize + DeserializeOwned;
    type Result: Serialize + DeserializeOwned;

    const KIND: &'static str;

    fn action_kind() -> ActionKind {
        ActionKind::from(Self::KIND)
    }
}
