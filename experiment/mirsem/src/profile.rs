#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionProfile {
    pub max_instructions: u64,
    pub max_call_depth: usize,
    pub linear_memory_size: u32,
    pub stack_size: u32,
    pub endianness: Endianness,
    pub host_pointers: HostPointerPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Endianness {
    Little,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostPointerPolicy {
    Forbidden,
}

impl Default for ExecutionProfile {
    fn default() -> Self {
        Self {
            max_instructions: 1_000_000,
            max_call_depth: 1024,
            linear_memory_size: 1024 * 1024,
            stack_size: 64 * 1024,
            endianness: Endianness::Little,
            host_pointers: HostPointerPolicy::Forbidden,
        }
    }
}

