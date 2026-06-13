use core::fmt;

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u32);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

stable_id!(FunctionId);
stable_id!(BlockId);
stable_id!(InstructionId);
stable_id!(ValueId);
stable_id!(TypeId);
stable_id!(SymbolId);
stable_id!(SourceSpanId);
