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

        writeln!(&mut asm, ".attribute arch, \"rv32im\"")?;
        writeln!(&mut asm, ".section .text")?;

        // 1. Forward declaration globals
        for function in &program.functions {
            writeln!(&mut asm, ".global mir_fn_{}", function.id.0)?;
        }
        writeln!(&mut asm)?;

        // 2. Generate each function
        for function in &program.functions {
            emit_function(&mut asm, function)?;
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

fn emit_function(asm: &mut String, function: &LoweredFunction) -> Result<(), CodegenError> {
    let frame = StackFrame::new(function);

    writeln!(asm, ".type mir_fn_{0}, @function", function.id.0)?;
    writeln!(asm, "mir_fn_{}:", function.id.0)?;

    // Prologue
    writeln!(asm, "    # Prologue")?;
    writeln!(asm, "    addi sp, sp, -{}", frame.frame_size)?;
    writeln!(asm, "    sw ra, {}(sp)", frame.frame_size + frame.ra_offset)?;
    writeln!(asm, "    sw s0, {}(sp)", frame.frame_size + frame.fp_offset)?;
    writeln!(asm, "    addi s0, sp, {}", frame.frame_size)?;

    // Spill argument registers a0-a7 to parameter stack slots
    writeln!(asm, "    # Spill arguments")?;
    for (idx, param) in function.params.iter().enumerate() {
        if idx < 8 {
            let offset = frame.offset_of(param.id);
            writeln!(asm, "    sw a{}, {}(s0)", idx, offset)?;
        }
    }

    // Emit block instructions
    for block in &function.blocks {
        writeln!(asm, "block_{}_{}:", function.id.0, block.label.id.0)?;
        for instruction in &block.instructions {
            emit_instruction(asm, instruction, &frame, function.id.0)?;
        }
    }

    Ok(())
}

fn emit_instruction(
    asm: &mut String,
    insn: &LoweredInstruction,
    frame: &StackFrame,
    func_id: u32,
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
            writeln!(asm, "    li t0, {}", imm)?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
        }
        Opcode::Copy => {
            let dest = one_write(insn)?;
            let src = insn
                .operands
                .first()
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            load_operand_to_reg(asm, src, "t0", frame)?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, lhs, "t0", frame)?;
            load_operand_to_reg(asm, rhs, "t1", frame)?;

            match insn.opcode {
                Opcode::AddI32 | Opcode::AddU32 => writeln!(asm, "    add t0, t0, t1")?,
                Opcode::SubI32 | Opcode::SubU32 => writeln!(asm, "    sub t0, t0, t1")?,
                Opcode::MulI32 | Opcode::MulU32 => writeln!(asm, "    mul t0, t0, t1")?,
                _ => unreachable!(),
            }
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, lhs, "t0", frame)?;
            load_operand_to_reg(asm, rhs, "t1", frame)?;
            writeln!(asm, "    slt t0, t0, t1")?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, lhs, "t0", frame)?;
            load_operand_to_reg(asm, rhs, "t1", frame)?;
            writeln!(asm, "    sltu t0, t0, t1")?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, lhs, "t0", frame)?;
            load_operand_to_reg(asm, rhs, "t1", frame)?;
            writeln!(asm, "    sub t0, t0, t1")?;
            writeln!(asm, "    seqz t0, t0")?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, lhs, "t0", frame)?;
            load_operand_to_reg(asm, rhs, "t1", frame)?;
            writeln!(asm, "    sub t0, t0, t1")?;
            writeln!(asm, "    snez t0, t0")?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, lhs, "t0", frame)?;
            load_operand_to_reg(asm, rhs, "t1", frame)?;
            writeln!(asm, "    sltu t0, t1, t0")?;
            writeln!(asm, "    xori t0, t0, 1")?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, lhs, "t0", frame)?;
            load_operand_to_reg(asm, rhs, "t1", frame)?;
            writeln!(asm, "    sltu t0, t1, t0")?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, lhs, "t0", frame)?;
            load_operand_to_reg(asm, rhs, "t1", frame)?;
            writeln!(asm, "    sltu t0, t0, t1")?;
            writeln!(asm, "    xori t0, t0, 1")?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, cond, "t0", frame)?;
            writeln!(asm, "    bne t0, zero, block_{}_{}", func_id, true_target)?;
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
            for idx in 1..insn.operands.len() {
                if idx - 1 < 8 {
                    let reg_name = match idx - 1 {
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
                    load_operand_to_reg(asm, &insn.operands[idx], reg_name, frame)?;
                }
            }
            writeln!(asm, "    jal ra, mir_fn_{}", callee)?;
            if !insn.writes.is_empty() {
                let dest = one_write(insn)?;
                writeln!(asm, "    sw a0, {}(s0)", frame.offset_of(dest.id))?;
            }
        }
        Opcode::Ret => {
            if !insn.operands.is_empty() {
                load_operand_to_reg(asm, &insn.operands[0], "a0", frame)?;
            }
            // Epilogue
            writeln!(asm, "    # Epilogue")?;
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
            load_operand_to_reg(asm, size, "a0", frame)?;
            load_operand_to_reg(asm, align, "a1", frame)?;
            writeln!(asm, "    jal ra, mir_alloc")?;
            writeln!(asm, "    sw a0, {}(s0)", frame.offset_of(dest.id))?;
        }
        Opcode::LoadI32 | Opcode::LoadU32 => {
            let dest = one_write(insn)?;
            let addr = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            load_operand_to_reg(asm, addr, "t0", frame)?;
            writeln!(asm, "    lw t1, 0(t0)")?;
            writeln!(asm, "    sw t1, {}(s0)", frame.offset_of(dest.id))?;
        }
        Opcode::LoadU8 => {
            let dest = one_write(insn)?;
            let addr = insn
                .operands
                .get(0)
                .ok_or(CodegenError::InvalidOperandIndex(0))?;
            load_operand_to_reg(asm, addr, "t0", frame)?;
            writeln!(asm, "    lbu t1, 0(t0)")?;
            writeln!(asm, "    sw t1, {}(s0)", frame.offset_of(dest.id))?;
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
            load_operand_to_reg(asm, addr, "t0", frame)?;
            load_operand_to_reg(asm, val, "t1", frame)?;
            writeln!(asm, "    sw t1, 0(t0)")?;
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
            load_operand_to_reg(asm, addr, "t0", frame)?;
            load_operand_to_reg(asm, val, "t1", frame)?;
            writeln!(asm, "    sb t1, 0(t0)")?;
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
            load_operand_to_reg(asm, base, "t0", frame)?;
            load_operand_to_reg(asm, offset, "t1", frame)?;
            writeln!(asm, "    add t0, t0, t1")?;
            writeln!(asm, "    sltu t2, t0, t1")?;
            // If overflowed (t2 != 0), trap
            writeln!(asm, "    beq t2, zero, .Lno_overflow_{}", insn.id.0)?;
            writeln!(asm, "    ebreak")?;
            writeln!(asm, ".Lno_overflow_{}:", insn.id.0)?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
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
            writeln!(asm, "    la t0, sym_{}", symbol.0)?;
            load_operand_to_reg(asm, offset, "t1", frame)?;
            writeln!(asm, "    add t0, t0, t1")?;
            writeln!(asm, "    sw t0, {}(s0)", frame.offset_of(dest.id))?;
        }
        Opcode::UnsupportedI64 | Opcode::UnsupportedIndirectCall => {
            return Err(CodegenError::UnsupportedOpcode(insn.opcode));
        }
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

fn load_operand_to_reg(
    asm: &mut String,
    operand: &LoweredOperand,
    reg: &'static str,
    frame: &StackFrame,
) -> Result<(), CodegenError> {
    match operand {
        LoweredOperand::Value(val) => {
            writeln!(asm, "    lw {}, {}(s0)", reg, frame.offset_of(val.id))?;
        }
        LoweredOperand::ImmI32(val) => {
            writeln!(asm, "    li {}, {}", reg, *val)?;
        }
        LoweredOperand::ImmU32(val) => {
            writeln!(asm, "    li {}, {}", reg, *val)?;
        }
        _ => {
            return Err(CodegenError::Generic(format!(
                "Unsupported operand for register loading: {:?}",
                operand
            )))
        }
    }
    Ok(())
}
