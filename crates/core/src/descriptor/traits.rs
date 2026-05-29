use crate::descriptor::types::{OkDescriptor, ParamsDescriptor, ValueDescriptor};

pub trait DescribeValue {
    fn describe_value() -> ValueDescriptor;
}

pub trait DescribeParams {
    fn describe_params() -> Option<ParamsDescriptor>;
}

pub trait DescribeOk {
    fn describe_ok() -> Option<OkDescriptor>;
}
