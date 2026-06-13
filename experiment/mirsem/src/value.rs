#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Void,
    I32(i32),
    U32(u32),
    Addr32(u32),
}

impl Value {
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Value::I32(value) => Some(*value),
            _ => None,
        }
    }

    pub fn truthy(&self) -> Option<bool> {
        match self {
            Value::I32(value) => Some(*value != 0),
            Value::U32(value) => Some(*value != 0),
            Value::Addr32(value) => Some(*value != 0),
            Value::Void => None,
        }
    }
}
