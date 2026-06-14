#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Register {
    Zero, // x0
    Ra,   // x1
    Sp,   // x2
    Gp,   // x3
    Tp,   // x4
    T0,   // x5
    T1,   // x6
    T2,   // x7
    S0,   // x8 / fp
    S1,   // x9
    A0,   // x10
    A1,   // x11
    A2,   // x12
    A3,   // x13
    A4,   // x14
    A5,   // x15
    A6,   // x16
    A7,   // x17
    S2,   // x18
    S3,   // x19
    S4,   // x20
    S5,   // x21
    S6,   // x22
    S7,   // x23
    S8,   // x24
    S9,   // x25
    S10,  // x26
    S11,  // x27
    T3,   // x28
    T4,   // x29
    T5,   // x30
    T6,   // x31
}

impl Register {
    pub fn name(self) -> &'static str {
        match self {
            Register::Zero => "zero",
            Register::Ra => "ra",
            Register::Sp => "sp",
            Register::Gp => "gp",
            Register::Tp => "tp",
            Register::T0 => "t0",
            Register::T1 => "t1",
            Register::T2 => "t2",
            Register::S0 => "s0",
            Register::S1 => "s1",
            Register::A0 => "a0",
            Register::A1 => "a1",
            Register::A2 => "a2",
            Register::A3 => "a3",
            Register::A4 => "a4",
            Register::A5 => "a5",
            Register::A6 => "a6",
            Register::A7 => "a7",
            Register::S2 => "s2",
            Register::S3 => "s3",
            Register::S4 => "s4",
            Register::S5 => "s5",
            Register::S6 => "s6",
            Register::S7 => "s7",
            Register::S8 => "s8",
            Register::S9 => "s9",
            Register::S10 => "s10",
            Register::S11 => "s11",
            Register::T3 => "t3",
            Register::T4 => "t4",
            Register::T5 => "t5",
            Register::T6 => "t6",
        }
    }
}
