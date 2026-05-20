use crate::descriptor::types::{FieldDescriptor, ParamsDescriptor, ValueDescriptor};

pub trait Accepts<Rhs = Self> {
    fn accepts(&self, actual: &Rhs) -> bool;
}

impl<T> Accepts for Option<T>
where
    T: Accepts,
{
    fn accepts(&self, actual: &Self) -> bool {
        match (self, actual) {
            (None, None) => true,
            (Some(expected), Some(actual)) => expected.accepts(actual),
            _ => false,
        }
    }
}

impl Accepts for ValueDescriptor {
    fn accepts(&self, actual: &Self) -> bool {
        match (self, actual) {
            (ValueDescriptor::Any, _) => true,

            (ValueDescriptor::Primitive(expected), ValueDescriptor::Primitive(actual)) => {
                expected == actual
            }

            (ValueDescriptor::Array(expected), ValueDescriptor::Array(actual)) => {
                expected.accepts(actual)
            }

            (ValueDescriptor::Map(expected), ValueDescriptor::Map(actual)) => {
                expected.accepts(actual)
            }

            (ValueDescriptor::Object(expected), ValueDescriptor::Object(actual)) => {
                fields_accept(&expected.fields, &actual.fields)
            }

            _ => false,
        }
    }
}

impl Accepts for ParamsDescriptor {
    fn accepts(&self, actual: &Self) -> bool {
        match (self, actual) {
            (ParamsDescriptor::Value(ValueDescriptor::Any), _) => true,

            (ParamsDescriptor::Value(expected), ParamsDescriptor::Value(actual)) => {
                expected.accepts(actual)
            }

            (ParamsDescriptor::Object(expected), ParamsDescriptor::Object(actual)) => {
                fields_accept(&expected.required_fields, &actual.required_fields)
                    && fields_accept(&expected.optional_fields, &actual.optional_fields)
            }

            _ => false,
        }
    }
}

fn fields_accept(expected: &[FieldDescriptor], actual: &[FieldDescriptor]) -> bool {
    if expected.len() != actual.len() {
        return false;
    }

    expected.iter().all(|expected_field| {
        actual.iter().any(|actual_field| {
            expected_field.name == actual_field.name && expected_field.ty.accepts(&actual_field.ty)
        })
    })
}
