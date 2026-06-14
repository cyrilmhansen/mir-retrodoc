#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Void,
    I32(i32),
    U32(u32),
    I64(i64),
    Addr32(u32),
    F32(u32),
    F64(u64),
}

impl Value {
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Value::I32(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Value::F32(bits) => Some(f32::from_bits(*bits)),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F64(bits) => Some(f64::from_bits(*bits)),
            _ => None,
        }
    }

    pub fn truthy(&self) -> Option<bool> {
        match self {
            Value::I32(value) => Some(*value != 0),
            Value::U32(value) => Some(*value != 0),
            Value::I64(value) => Some(*value != 0),
            Value::Addr32(value) => Some(*value != 0),
            Value::F32(value) => Some(*value != 0),
            Value::F64(value) => Some(*value != 0),
            Value::Void => None,
        }
    }
}
