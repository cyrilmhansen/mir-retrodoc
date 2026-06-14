use crate::error::CompileError;
use mircap::{Instruction, ModuleImage, Opcode, Operand, TypeKind};

pub fn emit_type(kind: TypeKind) -> Result<&'static str, CompileError> {
    match kind {
        TypeKind::Void => Ok("void"),
        TypeKind::I32 => Ok("int32_t"),
        TypeKind::U32 => Ok("uint32_t"),
        TypeKind::Addr32 => Ok("uint32_t"),
        TypeKind::I64 => Ok("int64_t"),
        _ => Err(CompileError::UnsupportedType(kind)),
    }
}

pub fn emit_operand(op: &Operand) -> String {
    match op {
        Operand::Value(val) => format!("v{}", val.0),
        Operand::ImmI32(imm) => {
            if *imm == i32::MIN {
                "((int32_t)(-2147483647 - 1))".to_string()
            } else {
                format!("((int32_t){})", imm)
            }
        }
        Operand::ImmU32(imm) => format!("{}u", imm),
        Operand::ImmI64(imm) => {
            if *imm == i64::MIN {
                "((int64_t)(-9223372036854775807LL - 1LL))".to_string()
            } else {
                format!("((int64_t){}LL)", imm)
            }
        }
        Operand::ImmF32(bits) => format!("/* f32 bits 0x{bits:08x} */"),
        Operand::ImmF64(bits) => format!("/* f64 bits 0x{bits:016x} */"),
        Operand::Block(block) => format!("block_{}", block.0),
        Operand::Function(func) => format!("mir_fn_{}", func.0),
        Operand::Symbol(sym) => format!("sym_{}", sym.0),
        Operand::Type(ty) => format!("/* type {} */", ty.0),
    }
}

pub fn emit_instruction(insn: &Instruction, image: &ModuleImage) -> Result<String, CompileError> {
    match insn.opcode {
        Opcode::ConstI32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let val = emit_operand(&insn.operands[0]);
            Ok(format!("v{} = {};", dest.0, val))
        }
        Opcode::ConstU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let val = emit_operand(&insn.operands[0]);
            Ok(format!("v{} = {};", dest.0, val))
        }
        Opcode::Copy => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let src = emit_operand(&insn.operands[0]);
            Ok(format!("v{} = {};", dest.0, src))
        }
        Opcode::AddI32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!(
                "v{} = (int32_t)((uint32_t){} + (uint32_t){});",
                dest.0, lhs, rhs
            ))
        }
        Opcode::SubI32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!(
                "v{} = (int32_t)((uint32_t){} - (uint32_t){});",
                dest.0, lhs, rhs
            ))
        }
        Opcode::MulI32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!(
                "v{} = (int32_t)((uint32_t){} * (uint32_t){});",
                dest.0, lhs, rhs
            ))
        }
        Opcode::EqI32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} == {}) ? 1u : 0u;", dest.0, lhs, rhs))
        }
        Opcode::NeI32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} != {}) ? 1u : 0u;", dest.0, lhs, rhs))
        }
        Opcode::LtI32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} < {}) ? 1u : 0u;", dest.0, lhs, rhs))
        }
        Opcode::AddU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = {} + {};", dest.0, lhs, rhs))
        }
        Opcode::SubU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = {} - {};", dest.0, lhs, rhs))
        }
        Opcode::MulU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = {} * {};", dest.0, lhs, rhs))
        }
        Opcode::EqU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} == {}) ? 1u : 0u;", dest.0, lhs, rhs))
        }
        Opcode::NeU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} != {}) ? 1u : 0u;", dest.0, lhs, rhs))
        }
        Opcode::LtU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} < {}) ? 1u : 0u;", dest.0, lhs, rhs))
        }
        Opcode::LeU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} <= {}) ? 1u : 0u;", dest.0, lhs, rhs))
        }
        Opcode::GtU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} > {}) ? 1u : 0u;", dest.0, lhs, rhs))
        }
        Opcode::GeU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} >= {}) ? 1u : 0u;", dest.0, lhs, rhs))
        }
        Opcode::Branch => {
            let target = emit_operand(&insn.operands[0]);
            Ok(format!("goto {};", target))
        }
        Opcode::BranchIf => {
            let cond = emit_operand(&insn.operands[0]);
            let t = emit_operand(&insn.operands[1]);
            let f = emit_operand(&insn.operands[2]);
            Ok(format!("if ({} != 0) goto {}; else goto {};", cond, t, f))
        }
        Opcode::Call => {
            let callee = emit_operand(&insn.operands[0]);
            let args: Vec<String> = insn.operands[1..].iter().map(emit_operand).collect();
            let args_str = args.join(", ");
            if insn.results.is_empty() {
                Ok(format!("{}({});", callee, args_str))
            } else if insn.results.len() == 1 {
                let dest = insn.results[0];
                Ok(format!("v{} = {}({});", dest.0, callee, args_str))
            } else {
                Err(CompileError::MultipleResultsNotSupported)
            }
        }
        Opcode::Ret => {
            if insn.operands.is_empty() {
                Ok("return;".to_string())
            } else if insn.operands.len() == 1 {
                let val = emit_operand(&insn.operands[0]);
                Ok(format!("return {};", val))
            } else {
                Err(CompileError::MultipleResultsNotSupported)
            }
        }
        Opcode::Trap => Ok("mir_trap(3);".to_string()),
        Opcode::Alloc => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let size = emit_operand(&insn.operands[0]);
            let align = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = mir_alloc({}, {});", dest.0, size, align))
        }
        Opcode::LoadI32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let addr = emit_operand(&insn.operands[0]);
            Ok(format!("v{} = mir_load_i32({});", dest.0, addr))
        }
        Opcode::LoadU32 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let addr = emit_operand(&insn.operands[0]);
            Ok(format!("v{} = mir_load_u32({});", dest.0, addr))
        }
        Opcode::StoreI32 => {
            let addr = emit_operand(&insn.operands[0]);
            let val = emit_operand(&insn.operands[1]);
            Ok(format!("mir_store_i32({}, {});", addr, val))
        }
        Opcode::StoreU32 => {
            let addr = emit_operand(&insn.operands[0]);
            let val = emit_operand(&insn.operands[1]);
            Ok(format!("mir_store_u32({}, {});", addr, val))
        }
        Opcode::LoadU8 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let addr = emit_operand(&insn.operands[0]);
            Ok(format!("v{} = mir_load_u8({});", dest.0, addr))
        }
        Opcode::StoreU8 => {
            let addr = emit_operand(&insn.operands[0]);
            let val = emit_operand(&insn.operands[1]);
            Ok(format!("mir_store_u8({}, {});", addr, val))
        }
        Opcode::AddrAdd => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let base = emit_operand(&insn.operands[0]);
            let offset = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = mir_addr_add({}, {});", dest.0, base, offset))
        }
        Opcode::DataAddr => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let sym_id = match insn.operands[0] {
                Operand::Symbol(sym_id) => sym_id,
                _ => return Err(CompileError::MultipleResultsNotSupported),
            };
            let offset = emit_operand(&insn.operands[1]);
            let ds = image
                .data_segments
                .iter()
                .find(|ds| ds.symbol == sym_id)
                .ok_or_else(|| CompileError::MultipleResultsNotSupported)?;
            let ds_len = ds.bytes.len() as u32 + ds.zero_fill;
            Ok(format!(
                "v{} = mir_data_addr({}u, {}, {}u);",
                dest.0, ds.offset, offset, ds_len
            ))
        }
        Opcode::ConstI64 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let imm = emit_operand(&insn.operands[0]);
            Ok(format!("v{} = {};", dest.0, imm))
        }
        Opcode::AddI64 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!(
                "v{} = (int64_t)((uint64_t){} + (uint64_t){});",
                dest.0, lhs, rhs
            ))
        }
        Opcode::SubI64 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!(
                "v{} = (int64_t)((uint64_t){} - (uint64_t){});",
                dest.0, lhs, rhs
            ))
        }
        Opcode::MulI64 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!(
                "v{} = (int64_t)((uint64_t){} * (uint64_t){});",
                dest.0, lhs, rhs
            ))
        }
        Opcode::EqI64 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} == {}) ? 1 : 0;", dest.0, lhs, rhs))
        }
        Opcode::NeI64 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} != {}) ? 1 : 0;", dest.0, lhs, rhs))
        }
        Opcode::LtI64 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let lhs = emit_operand(&insn.operands[0]);
            let rhs = emit_operand(&insn.operands[1]);
            Ok(format!("v{} = ({} < {}) ? 1 : 0;", dest.0, lhs, rhs))
        }
        Opcode::LoadI64 => {
            let dest = insn
                .results
                .first()
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            let addr = emit_operand(&insn.operands[0]);
            Ok(format!("v{} = mir_load_i64({});", dest.0, addr))
        }
        Opcode::StoreI64 => {
            let addr = emit_operand(&insn.operands[0]);
            let val = emit_operand(&insn.operands[1]);
            Ok(format!("mir_store_i64({}, {});", addr, val))
        }
        Opcode::ConstF32
        | Opcode::ConstF64
        | Opcode::AddF32
        | Opcode::SubF32
        | Opcode::MulF32
        | Opcode::DivF32
        | Opcode::NegF32
        | Opcode::EqF32
        | Opcode::NeF32
        | Opcode::LtF32
        | Opcode::LeF32
        | Opcode::GtF32
        | Opcode::GeF32
        | Opcode::AddF64
        | Opcode::SubF64
        | Opcode::MulF64
        | Opcode::DivF64
        | Opcode::NegF64
        | Opcode::EqF64
        | Opcode::NeF64
        | Opcode::LtF64
        | Opcode::LeF64
        | Opcode::GtF64
        | Opcode::GeF64
        | Opcode::I32ToF32
        | Opcode::F32ToI32
        | Opcode::I32ToF64
        | Opcode::F64ToI32
        | Opcode::F32ToF64
        | Opcode::F64ToF32
        | Opcode::UnsupportedIndirectCall => Err(CompileError::UnsupportedOpcode(insn.opcode)),
    }
}
