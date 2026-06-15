use crate::allocator::StackFrame;
use mircap::Opcode;
use mirplan::{LoweredFunction, LoweredInstruction, LoweredOperand, LoweredProgram, LoweredValue};
use std::error::Error;
use std::fmt;
use std::fmt::Write;

#[derive(Debug)]
pub enum CodegenError {
    Format(fmt::Error),
    UnsupportedOpcode(Opcode),
    MultipleResultsNotSupported,
    InvalidOperandIndex(usize),
    Generic(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::Format(e) => write!(f, "Format error: {}", e),
            CodegenError::UnsupportedOpcode(op) => write!(f, "Unsupported opcode: {:?}", op),
            CodegenError::MultipleResultsNotSupported => {
                write!(f, "Multiple results not supported")
            }
            CodegenError::InvalidOperandIndex(idx) => write!(f, "Invalid operand index: {}", idx),
            CodegenError::Generic(s) => write!(f, "Codegen error: {}", s),
        }
    }
}

impl Error for CodegenError {}

impl From<fmt::Error> for CodegenError {
    fn from(err: fmt::Error) -> Self {
        CodegenError::Format(err)
    }
}

pub struct Riscv32Backend;

impl mirplan::Backend for Riscv32Backend {
    type Output = String;
    type Error = CodegenError;

    fn compile(&self, program: &LoweredProgram) -> Result<Self::Output, Self::Error> {
        let mut asm = String::new();

        writeln!(&mut asm, ".attribute arch, \"rv32imafd\"")?;
        writeln!(&mut asm, ".section .text")?;

        // 1. Forward declaration globals
        for function in &program.functions {
            writeln!(&mut asm, ".global mir_fn_{}", function.id.0)?;
        }
        writeln!(&mut asm)?;

        // 2. Generate each function
        for function in &program.functions {
            emit_function(&mut asm, function, &program.data_segments)?;
            writeln!(&mut asm)?;
        }

        // 3. Generate data segments
        if !program.data_segments.is_empty() {
            writeln!(&mut asm, ".section .data")?;
            for segment in &program.data_segments {
                writeln!(&mut asm, ".align 4")?;
                writeln!(&mut asm, "sym_{}:", segment.symbol.0)?;
                if segment.bytes.is_empty() {
                    writeln!(&mut asm, "    .zero {}", segment.zero_fill)?;
                } else {
                    for byte in &segment.bytes {
                        writeln!(&mut asm, "    .byte {}", byte)?;
                    }
                    if segment.zero_fill > 0 {
                        writeln!(&mut asm, "    .zero {}", segment.zero_fill)?;
                    }
                }
            }
        }

        Ok(asm)
    }
}

fn emit_function(
    asm: &mut String,
    function: &LoweredFunction,
    data_segments: &[mirplan::DataSegmentPlan],
) -> Result<(), CodegenError> {
    let frame = StackFrame::new(function);

    writeln!(asm, ".type mir_fn_{0}, @function", function.id.0)?;
    writeln!(asm, "mir_fn_{}:", function.id.0)?;

    // Prologue
    writeln!(asm, "    # Prologue")?;
    writeln!(asm, "    addi sp, sp, -{}", frame.frame_size)?;
    writeln!(asm, "    sw ra, {}(sp)", frame.frame_size + frame.ra_offset)?;
    writeln!(asm, "    sw s0, {}(sp)", frame.frame_size + frame.fp_offset)?;
    writeln!(asm, "    addi s0, sp, {}", frame.frame_size)?;

    // Save used saved registers
    for &reg in &frame.used_saved_regs {
        let offset = frame.saved_reg_offsets.get(&reg).unwrap();
        writeln!(asm, "    sw {}, {}(s0)", reg.name(), offset)?;
    }

    // Move or spill argument registers a0-a7
    writeln!(asm, "    # Handle arguments")?;
    let mut arg_reg_idx = 0;
    for param in &function.params {
        let is_i64 = param.type_kind == mircap::TypeKind::I64;
        if is_i64 {
            if arg_reg_idx < 8 && arg_reg_idx + 1 < 8 {
                let offset = frame.offset_of(param.id);
                writeln!(asm, "    sw a{}, {}(s0)", arg_reg_idx, offset)?;
                writeln!(asm, "    sw a{}, {}(s0)", arg_reg_idx + 1, offset + 4)?;
            }
            arg_reg_idx += 2;
        } else {
            if arg_reg_idx < 8 {
                if let Some(reg) = frame.registers.get(&param.id) {
                    writeln!(asm, "    mv {}, a{}", reg.name(), arg_reg_idx)?;
                } else {
                    let offset = frame.offset_of(param.id);
                    writeln!(asm, "    sw a{}, {}(s0)", arg_reg_idx, offset)?;
                }
            }
            arg_reg_idx += 1;
        }
    }

    // Emit block instructions
    for block in &function.blocks {
        writeln!(asm, "block_{}_{}:", function.id.0, block.label.id.0)?;
        for instruction in &block.instructions {
            emit_instruction(asm, instruction, &frame, function.id.0, data_segments)?;
        }
    }

    Ok(())
}

fn emit_instruction(
    asm: &mut String,
    insn: &LoweredInstruction,
    frame: &StackFrame,
    func_id: u32,
    data_segments: &[mirplan::DataSegmentPlan],
) -> Result<(), CodegenError> {
    writeln!(asm, "    # {:?}", insn.opcode)?;
    match insn.opcode {
        Opcode::ConstI32 | Opcode::ConstU32 => {
            let dest = one_write(insn)?;
            let imm = match insn
                .operands
                .first()
                .ok_or(CodegenError::InvalidOperandIndex(0))?
            {
                LoweredOperand::ImmI32(val) => *val as u32,
                LoweredOperand::ImmU32(val) => *val,
                _ => {
                    return Err(CodegenError::Generic(
                        "Expected immediate operand".to_string(),
                    ))
                }
            };
            let (d_reg, spill) = resolve_dest(dest, "t0", frame);
            writeln!(asm, "    li {}, {}", d_reg, imm)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::ConstI64 => {
            let dest = one_write(insn)?;
            let offset = frame.offset_of(dest.id);
            let imm = match insn.operands.first().unwrap() {
                LoweredOperand::ImmI64(val) => *val,
                _ => return Err(CodegenError::Generic("Expected ImmI64".to_string())),
            };
            let low = (imm & 0xFFFFFFFF) as u32;
            let high = ((imm >> 32) & 0xFFFFFFFF) as u32;
            writeln!(asm, "    li t0, {}", low)?;
            writeln!(asm, "    sw t0, {}(s0)", offset)?;
            writeln!(asm, "    li t0, {}", high)?;
            writeln!(asm, "    sw t0, {}(s0)", offset + 4)?;
        }
        Opcode::Copy => {
            let dest = one_write(insn)?;
            let src = insn
                .operands
                .first()
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            if dest.type_kind == mircap::TypeKind::I64 {
                let d_offset = frame.offset_of(dest.id);
                match src {
                    LoweredOperand::ImmI64(val) => {
                        let low = (*val & 0xFFFFFFFF) as u32;
                        let high = ((*val >> 32) & 0xFFFFFFFF) as u32;
                        writeln!(asm, "    li t0, {}", low)?;
                        writeln!(asm, "    sw t0, {}(s0)", d_offset)?;
                        writeln!(asm, "    li t0, {}", high)?;
                        writeln!(asm, "    sw t0, {}(s0)", d_offset + 4)?;
                    }
                    LoweredOperand::Value(val) => {
                        let s_offset = frame.offset_of(val.id);
                        writeln!(asm, "    lw t0, {}(s0)", s_offset)?;
                        writeln!(asm, "    sw t0, {}(s0)", d_offset)?;
                        writeln!(asm, "    lw t0, {}(s0)", s_offset + 4)?;
                        writeln!(asm, "    sw t0, {}(s0)", d_offset + 4)?;
                    }
                    _ => {
                        return Err(CodegenError::Generic(
                            "Unsupported copy src for i64".to_string(),
                        ))
                    }
                }
            } else {
                let s_reg = resolve_operand(asm, src, "t0", frame)?;
                let (d_reg, spill) = resolve_dest(dest, "t1", frame);
                if s_reg != d_reg {
                    writeln!(asm, "    mv {}, {}", d_reg, s_reg)?;
                }
                if spill {
                    commit_dest(asm, dest, d_reg, frame)?;
                }
            }
        }
        Opcode::AddI32
        | Opcode::AddU32
        | Opcode::SubI32
        | Opcode::SubU32
        | Opcode::MulI32
        | Opcode::MulU32 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let s1_reg = resolve_operand(asm, lhs, "t0", frame)?;
            let s2_reg = resolve_operand(asm, rhs, "t1", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t2", frame);

            match insn.opcode {
                Opcode::AddI32 | Opcode::AddU32 => {
                    writeln!(asm, "    add {}, {}, {}", d_reg, s1_reg, s2_reg)?
                }
                Opcode::SubI32 | Opcode::SubU32 => {
                    writeln!(asm, "    sub {}, {}, {}", d_reg, s1_reg, s2_reg)?
                }
                Opcode::MulI32 | Opcode::MulU32 => {
                    writeln!(asm, "    mul {}, {}, {}", d_reg, s1_reg, s2_reg)?
                }
                _ => unreachable!(),
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::AddI64 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            load_i64_operand(asm, lhs, "t0", "t1", frame)?;
            load_i64_operand(asm, rhs, "t2", "t3", frame)?;
            writeln!(asm, "    add t4, t0, t2")?;
            writeln!(asm, "    sltu t5, t4, t0")?;
            writeln!(asm, "    add t6, t1, t3")?;
            writeln!(asm, "    add t6, t6, t5")?;
            let d_offset = frame.offset_of(dest.id);
            writeln!(asm, "    sw t4, {}(s0)", d_offset)?;
            writeln!(asm, "    sw t6, {}(s0)", d_offset + 4)?;
        }
        Opcode::SubI64 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            load_i64_operand(asm, lhs, "t0", "t1", frame)?;
            load_i64_operand(asm, rhs, "t2", "t3", frame)?;
            writeln!(asm, "    sltu t5, t0, t2")?;
            writeln!(asm, "    sub t4, t0, t2")?;
            writeln!(asm, "    sub t6, t1, t3")?;
            writeln!(asm, "    sub t6, t6, t5")?;
            let d_offset = frame.offset_of(dest.id);
            writeln!(asm, "    sw t4, {}(s0)", d_offset)?;
            writeln!(asm, "    sw t6, {}(s0)", d_offset + 4)?;
        }
        Opcode::MulI64 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            load_i64_operand(asm, lhs, "t0", "t1", frame)?;
            load_i64_operand(asm, rhs, "t2", "t3", frame)?;
            writeln!(asm, "    mulhu t4, t0, t2")?;
            writeln!(asm, "    mul t5, t0, t2")?;
            writeln!(asm, "    mul t6, t1, t2")?;
            writeln!(asm, "    add t4, t4, t6")?;
            writeln!(asm, "    mul t6, t0, t3")?;
            writeln!(asm, "    add t4, t4, t6")?;
            let d_offset = frame.offset_of(dest.id);
            writeln!(asm, "    sw t5, {}(s0)", d_offset)?;
            writeln!(asm, "    sw t4, {}(s0)", d_offset + 4)?;
        }
        Opcode::LtI32 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let s1_reg = resolve_operand(asm, lhs, "t0", frame)?;
            let s2_reg = resolve_operand(asm, rhs, "t1", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t2", frame);
            writeln!(asm, "    slt {}, {}, {}", d_reg, s1_reg, s2_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::LtU32 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let s1_reg = resolve_operand(asm, lhs, "t0", frame)?;
            let s2_reg = resolve_operand(asm, rhs, "t1", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t2", frame);
            writeln!(asm, "    sltu {}, {}, {}", d_reg, s1_reg, s2_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::EqI32 | Opcode::EqU32 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let s1_reg = resolve_operand(asm, lhs, "t0", frame)?;
            let s2_reg = resolve_operand(asm, rhs, "t1", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t2", frame);
            writeln!(asm, "    sub {}, {}, {}", d_reg, s1_reg, s2_reg)?;
            writeln!(asm, "    seqz {}, {}", d_reg, d_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::NeI32 | Opcode::NeU32 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let s1_reg = resolve_operand(asm, lhs, "t0", frame)?;
            let s2_reg = resolve_operand(asm, rhs, "t1", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t2", frame);
            writeln!(asm, "    sub {}, {}, {}", d_reg, s1_reg, s2_reg)?;
            writeln!(asm, "    snez {}, {}", d_reg, d_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::EqI64 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            load_i64_operand(asm, lhs, "t0", "t1", frame)?;
            load_i64_operand(asm, rhs, "t2", "t3", frame)?;
            writeln!(asm, "    xor t4, t0, t2")?;
            writeln!(asm, "    xor t5, t1, t3")?;
            writeln!(asm, "    or t4, t4, t5")?;
            let (d_reg, spill) = resolve_dest(dest, "t6", frame);
            writeln!(asm, "    seqz {}, t4", d_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::NeI64 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            load_i64_operand(asm, lhs, "t0", "t1", frame)?;
            load_i64_operand(asm, rhs, "t2", "t3", frame)?;
            writeln!(asm, "    xor t4, t0, t2")?;
            writeln!(asm, "    xor t5, t1, t3")?;
            writeln!(asm, "    or t4, t4, t5")?;
            let (d_reg, spill) = resolve_dest(dest, "t6", frame);
            writeln!(asm, "    snez {}, t4", d_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::LtI64 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            load_i64_operand(asm, lhs, "t0", "t1", frame)?;
            load_i64_operand(asm, rhs, "t2", "t3", frame)?;
            writeln!(asm, "    slt t4, t1, t3")?;
            writeln!(asm, "    sub t5, t1, t3")?;
            writeln!(asm, "    seqz t5, t5")?;
            writeln!(asm, "    sltu t6, t0, t2")?;
            writeln!(asm, "    and t5, t5, t6")?;
            writeln!(asm, "    or t4, t4, t5")?;
            let (d_reg, spill) = resolve_dest(dest, "t6", frame);
            writeln!(asm, "    mv {}, t4", d_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::LeU32 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let s1_reg = resolve_operand(asm, lhs, "t0", frame)?;
            let s2_reg = resolve_operand(asm, rhs, "t1", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t2", frame);
            writeln!(asm, "    sltu {}, {}, {}", d_reg, s2_reg, s1_reg)?;
            writeln!(asm, "    xori {}, {}, 1", d_reg, d_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::GtU32 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let s1_reg = resolve_operand(asm, lhs, "t0", frame)?;
            let s2_reg = resolve_operand(asm, rhs, "t1", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t2", frame);
            writeln!(asm, "    sltu {}, {}, {}", d_reg, s2_reg, s1_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::GeU32 => {
            let dest = one_write(insn)?;
            let lhs = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let rhs = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let s1_reg = resolve_operand(asm, lhs, "t0", frame)?;
            let s2_reg = resolve_operand(asm, rhs, "t1", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t2", frame);
            writeln!(asm, "    sltu {}, {}, {}", d_reg, s1_reg, s2_reg)?;
            writeln!(asm, "    xori {}, {}, 1", d_reg, d_reg)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::Branch => {
            let target = match insn
                .operands
                .first()
                .ok_or(CodegenError::InvalidOperandIndex(0))?
            {
                LoweredOperand::Block(label) => label.id.0,
                _ => return Err(CodegenError::Generic("Expected block operand".to_string())),
            };
            writeln!(asm, "    j block_{}_{}", func_id, target)?;
        }
        Opcode::BranchIf => {
            let cond = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let true_target = match insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?
            {
                LoweredOperand::Block(label) => label.id.0,
                _ => return Err(CodegenError::Generic("Expected block operand".to_string())),
            };
            let false_target = match insn
                .operands
                .get(2)
                .ok_or(CodegenError::InvalidOperandIndex(2))?
            {
                LoweredOperand::Block(label) => label.id.0,
                _ => return Err(CodegenError::Generic("Expected block operand".to_string())),
            };
            let cond_reg = resolve_operand(asm, cond, "t0", frame)?;
            writeln!(
                asm,
                "    bne {}, zero, block_{}_{}",
                cond_reg, func_id, true_target
            )?;
            writeln!(asm, "    j block_{}_{}", func_id, false_target)?;
        }
        Opcode::Call => {
            let callee = match insn
                .operands
                .first()
                .ok_or(CodegenError::InvalidOperandIndex(0))?
            {
                LoweredOperand::Function(func_ref) => func_ref.id.0,
                _ => {
                    return Err(CodegenError::Generic(
                        "Expected function operand".to_string(),
                    ))
                }
            };
            // Load parameters into registers a0-a7
            let mut arg_reg_idx = 0;
            for idx in 1..insn.operands.len() {
                let op = &insn.operands[idx];
                if is_i64_operand(op, frame) {
                    if arg_reg_idx < 8 && arg_reg_idx + 1 < 8 {
                        match op {
                            LoweredOperand::ImmI64(val) => {
                                let low = (*val & 0xFFFFFFFF) as u32;
                                let high = ((*val >> 32) & 0xFFFFFFFF) as u32;
                                writeln!(asm, "    li a{}, {}", arg_reg_idx, low)?;
                                writeln!(asm, "    li a{}, {}", arg_reg_idx + 1, high)?;
                            }
                            LoweredOperand::Value(val) => {
                                let offset = frame.offset_of(val.id);
                                writeln!(asm, "    lw a{}, {}(s0)", arg_reg_idx, offset)?;
                                writeln!(asm, "    lw a{}, {}(s0)", arg_reg_idx + 1, offset + 4)?;
                            }
                            _ => unreachable!(),
                        }
                    }
                    arg_reg_idx += 2;
                } else {
                    if arg_reg_idx < 8 {
                        let scratch = match arg_reg_idx {
                            0 => "a0",
                            1 => "a1",
                            2 => "a2",
                            3 => "a3",
                            4 => "a4",
                            5 => "a5",
                            6 => "a6",
                            7 => "a7",
                            _ => unreachable!(),
                        };
                        let p_reg = resolve_operand(asm, op, scratch, frame)?;
                        if p_reg != scratch {
                            writeln!(asm, "    mv {}, {}", scratch, p_reg)?;
                        }
                    }
                    arg_reg_idx += 1;
                }
            }
            writeln!(asm, "    jal ra, mir_fn_{}", callee)?;
            if !insn.writes.is_empty() {
                let dest = one_write(insn)?;
                if dest.type_kind == mircap::TypeKind::I64 {
                    let offset = frame.offset_of(dest.id);
                    writeln!(asm, "    sw a0, {}(s0)", offset)?;
                    writeln!(asm, "    sw a1, {}(s0)", offset + 4)?;
                } else {
                    let (d_reg, spill) = resolve_dest(dest, "a0", frame);
                    if d_reg != "a0" {
                        writeln!(asm, "    mv {}, a0", d_reg)?;
                    }
                    if spill {
                        commit_dest(asm, dest, d_reg, frame)?;
                    }
                }
            }
        }
        Opcode::Ret => {
            if !insn.operands.is_empty() {
                let op = &insn.operands[0];
                if is_i64_operand(op, frame) {
                    match op {
                        LoweredOperand::ImmI64(val) => {
                            let low = (*val & 0xFFFFFFFF) as u32;
                            let high = ((*val >> 32) & 0xFFFFFFFF) as u32;
                            writeln!(asm, "    li a0, {}", low)?;
                            writeln!(asm, "    li a1, {}", high)?;
                        }
                        LoweredOperand::Value(val) => {
                            let offset = frame.offset_of(val.id);
                            writeln!(asm, "    lw a0, {}(s0)", offset)?;
                            writeln!(asm, "    lw a1, {}(s0)", offset + 4)?;
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let r = resolve_operand(asm, op, "a0", frame)?;
                    if r != "a0" {
                        writeln!(asm, "    mv a0, {}", r)?;
                    }
                }
            }
            // Epilogue
            writeln!(asm, "    # Epilogue")?;
            for &reg in &frame.used_saved_regs {
                let offset = frame.saved_reg_offsets.get(&reg).unwrap();
                writeln!(asm, "    lw {}, {}(s0)", reg.name(), offset)?;
            }
            writeln!(asm, "    lw ra, {}(sp)", frame.frame_size + frame.ra_offset)?;
            writeln!(asm, "    lw s0, {}(sp)", frame.frame_size + frame.fp_offset)?;
            writeln!(asm, "    addi sp, sp, {}", frame.frame_size)?;
            writeln!(asm, "    jr ra")?;
        }
        Opcode::Trap => {
            writeln!(asm, "    ebreak")?;
        }
        Opcode::Alloc => {
            let dest = one_write(insn)?;
            let size = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let align = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let r_size = resolve_operand(asm, size, "a0", frame)?;
            if r_size != "a0" {
                writeln!(asm, "    mv a0, {}", r_size)?;
            }
            let r_align = resolve_operand(asm, align, "a1", frame)?;
            if r_align != "a1" {
                writeln!(asm, "    mv a1, {}", r_align)?;
            }
            writeln!(asm, "    jal ra, mir_alloc")?;
            let (d_reg, spill) = resolve_dest(dest, "a0", frame);
            if d_reg != "a0" {
                writeln!(asm, "    mv {}, a0", d_reg)?;
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::LoadI32 | Opcode::LoadU32 => {
            let dest = one_write(insn)?;
            let addr = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let r_addr = resolve_operand(asm, addr, "t0", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            writeln!(asm, "    lw {}, 0({})", d_reg, r_addr)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::LoadI64 => {
            let dest = one_write(insn)?;
            let addr = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let r_addr = resolve_operand(asm, addr, "t0", frame)?;
            writeln!(asm, "    lw t1, 0({})", r_addr)?;
            writeln!(asm, "    lw t2, 4({})", r_addr)?;
            let d_offset = frame.offset_of(dest.id);
            writeln!(asm, "    sw t1, {}(s0)", d_offset)?;
            writeln!(asm, "    sw t2, {}(s0)", d_offset + 4)?;
        }
        Opcode::LoadU8 => {
            let dest = one_write(insn)?;
            let addr = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let r_addr = resolve_operand(asm, addr, "t0", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            writeln!(asm, "    lbu {}, 0({})", d_reg, r_addr)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::StoreI32 | Opcode::StoreU32 => {
            let addr = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let val = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let r_addr = resolve_operand(asm, addr, "t0", frame)?;
            let r_val = resolve_operand(asm, val, "t1", frame)?;
            writeln!(asm, "    sw {}, 0({})", r_val, r_addr)?;
        }
        Opcode::StoreI64 => {
            let addr = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let val = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let r_addr = resolve_operand(asm, addr, "t0", frame)?;
            load_i64_operand(asm, val, "t1", "t2", frame)?;
            writeln!(asm, "    sw t1, 0({})", r_addr)?;
            writeln!(asm, "    sw t2, 4({})", r_addr)?;
        }
        Opcode::StoreU8 => {
            let addr = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let val = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let r_addr = resolve_operand(asm, addr, "t0", frame)?;
            let r_val = resolve_operand(asm, val, "t1", frame)?;
            writeln!(asm, "    sb {}, 0({})", r_val, r_addr)?;
        }
        Opcode::AddrAdd => {
            let dest = one_write(insn)?;
            let base = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            let offset = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;
            let r_base = resolve_operand(asm, base, "t0", frame)?;
            let r_offset = resolve_operand(asm, offset, "t1", frame)?;
            let (d_reg, spill) = resolve_dest(dest, "t2", frame);
            writeln!(asm, "    add {}, {}, {}", d_reg, r_base, r_offset)?;
            writeln!(asm, "    sltu t3, {}, {}", d_reg, r_offset)?;
            writeln!(asm, "    beq t3, zero, .Lno_overflow_{}", insn.id.0)?;
            writeln!(asm, "    ebreak")?;
            writeln!(asm, ".Lno_overflow_{}:", insn.id.0)?;
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::DataAddr => {
            let dest = one_write(insn)?;
            let symbol = match insn
                .operands
                .first()
                .ok_or(CodegenError::InvalidOperandIndex(0))?
            {
                LoweredOperand::Symbol { id, .. } => *id,
                _ => return Err(CodegenError::Generic("Expected symbol operand".to_string())),
            };
            let offset = insn
                .operands
                .get(1)
                .ok_or(CodegenError::InvalidOperandIndex(1))?;

            let segment = data_segments
                .iter()
                .find(|seg| seg.symbol == symbol)
                .ok_or_else(|| {
                    CodegenError::Generic(format!("missing data segment symbol {:?}", symbol))
                })?;
            let segment_len = segment.length;

            let r_offset = resolve_operand(asm, offset, "t1", frame)?;

            // 1. Check if offset > segment_len
            writeln!(asm, "    li t3, {}", segment_len)?;
            writeln!(asm, "    sltu t4, t3, {}", r_offset)?;
            writeln!(asm, "    beq t4, zero, .Lbounds_ok_{}", insn.id.0)?;
            writeln!(asm, "    ebreak")?;
            writeln!(asm, ".Lbounds_ok_{}:", insn.id.0)?;

            // 2. Load symbol address and add offset
            writeln!(asm, "    la t0, sym_{}", symbol.0)?;
            writeln!(asm, "    add t0, t0, {}", r_offset)?;

            // 3. Check for address overflow
            writeln!(asm, "    sltu t3, t0, {}", r_offset)?;
            writeln!(asm, "    beq t3, zero, .Lno_overflow_{}", insn.id.0)?;
            writeln!(asm, "    ebreak")?;
            writeln!(asm, ".Lno_overflow_{}:", insn.id.0)?;

            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            if d_reg != "t0" {
                writeln!(asm, "    mv {}, t0", d_reg)?;
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::ConstF32 => {
            let dest = one_write(insn)?;
            let bits = match insn.operands.first().unwrap() {
                LoweredOperand::ImmF32(val) => *val,
                _ => return Err(CodegenError::Generic("Expected ImmF32".to_string())),
            };
            writeln!(asm, "    li t0, {}", bits as i32)?;
            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            if d_reg != "t0" {
                writeln!(asm, "    mv {}, t0", d_reg)?;
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::ConstF64 => {
            let dest = one_write(insn)?;
            let offset = frame.offset_of(dest.id);
            let bits = match insn.operands.first().unwrap() {
                LoweredOperand::ImmF64(val) => *val,
                _ => return Err(CodegenError::Generic("Expected ImmF64".to_string())),
            };
            let low = (bits & 0xFFFFFFFF) as u32;
            let high = ((bits >> 32) & 0xFFFFFFFF) as u32;
            writeln!(asm, "    li t0, {}", low as i32)?;
            writeln!(asm, "    sw t0, {}(s0)", offset)?;
            writeln!(asm, "    li t0, {}", high as i32)?;
            writeln!(asm, "    sw t0, {}(s0)", offset + 4)?;
        }
        Opcode::AddF32 | Opcode::SubF32 | Opcode::MulF32 | Opcode::DivF32 => {
            let dest = one_write(insn)?;
            resolve_f32_operand(asm, &insn.operands[0], "ft0", frame)?;
            resolve_f32_operand(asm, &insn.operands[1], "ft1", frame)?;
            let op_str = match insn.opcode {
                Opcode::AddF32 => "fadd.s",
                Opcode::SubF32 => "fsub.s",
                Opcode::MulF32 => "fmul.s",
                Opcode::DivF32 => "fdiv.s",
                _ => unreachable!(),
            };
            writeln!(asm, "    {} ft2, ft0, ft1", op_str)?;
            let offset = frame.offset_of(dest.id);
            writeln!(asm, "    fsw ft2, {}(s0)", offset)?;
        }
        Opcode::AddF64 | Opcode::SubF64 | Opcode::MulF64 | Opcode::DivF64 => {
            let dest = one_write(insn)?;
            resolve_f64_operand(asm, &insn.operands[0], "ft0", frame)?;
            resolve_f64_operand(asm, &insn.operands[1], "ft1", frame)?;
            let op_str = match insn.opcode {
                Opcode::AddF64 => "fadd.d",
                Opcode::SubF64 => "fsub.d",
                Opcode::MulF64 => "fmul.d",
                Opcode::DivF64 => "fdiv.d",
                _ => unreachable!(),
            };
            writeln!(asm, "    {} ft2, ft0, ft1", op_str)?;
            let offset = frame.offset_of(dest.id);
            writeln!(asm, "    fsd ft2, {}(s0)", offset)?;
        }
        Opcode::NegF32 => {
            let dest = one_write(insn)?;
            resolve_f32_operand(asm, &insn.operands[0], "ft0", frame)?;
            writeln!(asm, "    fneg.s ft1, ft0")?;
            let offset = frame.offset_of(dest.id);
            writeln!(asm, "    fsw ft1, {}(s0)", offset)?;
        }
        Opcode::NegF64 => {
            let dest = one_write(insn)?;
            resolve_f64_operand(asm, &insn.operands[0], "ft0", frame)?;
            writeln!(asm, "    fneg.d ft1, ft0")?;
            let offset = frame.offset_of(dest.id);
            writeln!(asm, "    fsd ft1, {}(s0)", offset)?;
        }
        Opcode::EqF32 | Opcode::LtF32 | Opcode::LeF32 => {
            let dest = one_write(insn)?;
            resolve_f32_operand(asm, &insn.operands[0], "ft0", frame)?;
            resolve_f32_operand(asm, &insn.operands[1], "ft1", frame)?;
            let (op_str, reg1, reg2) = match insn.opcode {
                Opcode::EqF32 => ("feq.s", "ft0", "ft1"),
                Opcode::LtF32 => ("flt.s", "ft0", "ft1"),
                Opcode::LeF32 => ("fle.s", "ft0", "ft1"),
                _ => unreachable!(),
            };
            writeln!(asm, "    {} t0, {}, {}", op_str, reg1, reg2)?;
            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            if d_reg != "t0" {
                writeln!(asm, "    mv {}, t0", d_reg)?;
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::NeF32 | Opcode::GtF32 | Opcode::GeF32 => {
            let dest = one_write(insn)?;
            resolve_f32_operand(asm, &insn.operands[0], "ft0", frame)?;
            resolve_f32_operand(asm, &insn.operands[1], "ft1", frame)?;
            match insn.opcode {
                Opcode::NeF32 => {
                    writeln!(asm, "    feq.s t0, ft0, ft1")?;
                    writeln!(asm, "    seqz t0, t0")?;
                }
                Opcode::GtF32 => {
                    writeln!(asm, "    flt.s t0, ft1, ft0")?;
                }
                Opcode::GeF32 => {
                    writeln!(asm, "    fle.s t0, ft1, ft0")?;
                }
                _ => unreachable!(),
            };
            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            if d_reg != "t0" {
                writeln!(asm, "    mv {}, t0", d_reg)?;
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::EqF64 | Opcode::LtF64 | Opcode::LeF64 => {
            let dest = one_write(insn)?;
            resolve_f64_operand(asm, &insn.operands[0], "ft0", frame)?;
            resolve_f64_operand(asm, &insn.operands[1], "ft1", frame)?;
            let (op_str, reg1, reg2) = match insn.opcode {
                Opcode::EqF64 => ("feq.d", "ft0", "ft1"),
                Opcode::LtF64 => ("flt.d", "ft0", "ft1"),
                Opcode::LeF64 => ("fle.d", "ft0", "ft1"),
                _ => unreachable!(),
            };
            writeln!(asm, "    {} t0, {}, {}", op_str, reg1, reg2)?;
            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            if d_reg != "t0" {
                writeln!(asm, "    mv {}, t0", d_reg)?;
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::NeF64 | Opcode::GtF64 | Opcode::GeF64 => {
            let dest = one_write(insn)?;
            resolve_f64_operand(asm, &insn.operands[0], "ft0", frame)?;
            resolve_f64_operand(asm, &insn.operands[1], "ft1", frame)?;
            match insn.opcode {
                Opcode::NeF64 => {
                    writeln!(asm, "    feq.d t0, ft0, ft1")?;
                    writeln!(asm, "    seqz t0, t0")?;
                }
                Opcode::GtF64 => {
                    writeln!(asm, "    flt.d t0, ft1, ft0")?;
                }
                Opcode::GeF64 => {
                    writeln!(asm, "    fle.d t0, ft1, ft0")?;
                }
                _ => unreachable!(),
            };
            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            if d_reg != "t0" {
                writeln!(asm, "    mv {}, t0", d_reg)?;
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::I32ToF32 => {
            let dest = one_write(insn)?;
            let s_reg = resolve_operand(asm, &insn.operands[0], "t0", frame)?;
            writeln!(asm, "    fcvt.s.w ft0, {}", s_reg)?;
            let offset = frame.offset_of(dest.id);
            writeln!(asm, "    fsw ft0, {}(s0)", offset)?;
        }
        Opcode::F32ToI32 => {
            let dest = one_write(insn)?;
            resolve_f32_operand(asm, &insn.operands[0], "ft0", frame)?;
            writeln!(asm, "    fcvt.w.s t0, ft0, rtz")?;
            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            if d_reg != "t0" {
                writeln!(asm, "    mv {}, t0", d_reg)?;
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::I32ToF64 => {
            let dest = one_write(insn)?;
            let s_reg = resolve_operand(asm, &insn.operands[0], "t0", frame)?;
            writeln!(asm, "    fcvt.d.w ft0, {}", s_reg)?;
            let offset = frame.offset_of(dest.id);
            writeln!(asm, "    fsd ft0, {}(s0)", offset)?;
        }
        Opcode::F64ToI32 => {
            let dest = one_write(insn)?;
            resolve_f64_operand(asm, &insn.operands[0], "ft0", frame)?;
            writeln!(asm, "    fcvt.w.d t0, ft0, rtz")?;
            let (d_reg, spill) = resolve_dest(dest, "t1", frame);
            if d_reg != "t0" {
                writeln!(asm, "    mv {}, t0", d_reg)?;
            }
            if spill {
                commit_dest(asm, dest, d_reg, frame)?;
            }
        }
        Opcode::F32ToF64 => {
            let dest = one_write(insn)?;
            resolve_f32_operand(asm, &insn.operands[0], "ft0", frame)?;
            writeln!(asm, "    fcvt.d.s ft1, ft0")?;
            let offset = frame.offset_of(dest.id);
            writeln!(asm, "    fsd ft1, {}(s0)", offset)?;
        }
        Opcode::F64ToF32 => {
            let dest = one_write(insn)?;
            resolve_f64_operand(asm, &insn.operands[0], "ft0", frame)?;
            writeln!(asm, "    fcvt.s.d ft1, ft0")?;
            let offset = frame.offset_of(dest.id);
            writeln!(asm, "    fsw ft1, {}(s0)", offset)?;
        }
        Opcode::UnsupportedIndirectCall => {
            return Err(CodegenError::UnsupportedOpcode(insn.opcode))
        }
    }
    Ok(())
}

fn resolve_f32_operand(
    asm: &mut String,
    op: &LoweredOperand,
    freg: &str,
    frame: &StackFrame,
) -> Result<(), CodegenError> {
    match op {
        LoweredOperand::Value(val) => {
            if let Some(reg) = frame.registers.get(&val.id) {
                writeln!(asm, "    fmv.w.x {}, {}", freg, reg.name())?;
            } else {
                let offset = frame.offset_of(val.id);
                writeln!(asm, "    flw {}, {}(s0)", freg, offset)?;
            }
        }
        LoweredOperand::ImmF32(bits) => {
            writeln!(asm, "    li t0, {}", *bits as i32)?;
            writeln!(asm, "    fmv.w.x {}, t0", freg)?;
        }
        _ => return Err(CodegenError::Generic("Expected F32 operand".to_string())),
    }
    Ok(())
}

fn resolve_f64_operand(
    asm: &mut String,
    op: &LoweredOperand,
    freg: &str,
    frame: &StackFrame,
) -> Result<(), CodegenError> {
    match op {
        LoweredOperand::Value(val) => {
            let offset = frame.offset_of(val.id);
            writeln!(asm, "    fld {}, {}(s0)", freg, offset)?;
        }
        LoweredOperand::ImmF64(bits) => {
            let low = (*bits & 0xFFFFFFFF) as u32;
            let high = ((*bits >> 32) & 0xFFFFFFFF) as u32;
            writeln!(asm, "    li t0, {}", low as i32)?;
            writeln!(asm, "    li t1, {}", high as i32)?;
            writeln!(asm, "    addi sp, sp, -8")?;
            writeln!(asm, "    sw t0, 0(sp)")?;
            writeln!(asm, "    sw t1, 4(sp)")?;
            writeln!(asm, "    fld {}, 0(sp)", freg)?;
            writeln!(asm, "    addi sp, sp, 8")?;
        }
        _ => return Err(CodegenError::Generic("Expected F64 operand".to_string())),
    }
    Ok(())
}

fn one_write(insn: &LoweredInstruction) -> Result<&LoweredValue, CodegenError> {
    if insn.writes.len() == 1 {
        Ok(&insn.writes[0])
    } else {
        Err(CodegenError::MultipleResultsNotSupported)
    }
}

fn is_i64_operand(op: &LoweredOperand, frame: &StackFrame) -> bool {
    match op {
        LoweredOperand::Value(val) => frame.val_types.get(&val.id) == Some(&mircap::TypeKind::I64),
        LoweredOperand::ImmI64(_) => true,
        _ => false,
    }
}

fn load_i64_operand(
    asm: &mut String,
    op: &LoweredOperand,
    reg_low: &str,
    reg_high: &str,
    frame: &StackFrame,
) -> Result<(), CodegenError> {
    match op {
        LoweredOperand::ImmI64(val) => {
            let low = (*val & 0xFFFFFFFF) as u32;
            let high = ((*val >> 32) & 0xFFFFFFFF) as u32;
            writeln!(asm, "    li {}, {}", reg_low, low)?;
            writeln!(asm, "    li {}, {}", reg_high, high)?;
        }
        LoweredOperand::Value(val) => {
            let offset = frame.offset_of(val.id);
            writeln!(asm, "    lw {}, {}(s0)", reg_low, offset)?;
            writeln!(asm, "    lw {}, {}(s0)", reg_high, offset + 4)?;
        }
        _ => {
            return Err(CodegenError::Generic(format!(
                "Expected i64 operand, got {:?}",
                op
            )))
        }
    }
    Ok(())
}

fn resolve_operand(
    asm: &mut String,
    operand: &LoweredOperand,
    scratch: &'static str,
    frame: &StackFrame,
) -> Result<&'static str, CodegenError> {
    match operand {
        LoweredOperand::Value(val) => {
            if let Some(reg) = frame.registers.get(&val.id) {
                Ok(reg.name())
            } else {
                let offset = frame.offset_of(val.id);
                writeln!(asm, "    lw {}, {}(s0)", scratch, offset)?;
                Ok(scratch)
            }
        }
        LoweredOperand::ImmI32(val) => {
            writeln!(asm, "    li {}, {}", scratch, *val)?;
            Ok(scratch)
        }
        LoweredOperand::ImmU32(val) => {
            writeln!(asm, "    li {}, {}", scratch, *val)?;
            Ok(scratch)
        }
        _ => Err(CodegenError::Generic(format!(
            "Unsupported operand for resolving: {:?}",
            operand
        ))),
    }
}

fn resolve_dest(
    dest: &LoweredValue,
    scratch: &'static str,
    frame: &StackFrame,
) -> (&'static str, bool) {
    if let Some(reg) = frame.registers.get(&dest.id) {
        (reg.name(), false)
    } else {
        (scratch, true)
    }
}

fn commit_dest(
    asm: &mut String,
    dest: &LoweredValue,
    reg: &'static str,
    frame: &StackFrame,
) -> Result<(), CodegenError> {
    let offset = frame.offset_of(dest.id);
    writeln!(asm, "    sw {}, {}(s0)", reg, offset)?;
    Ok(())
}
