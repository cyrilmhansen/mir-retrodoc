use crate::c_emit::emit_type;
use crate::error::CompileError;
use crate::pretty::pretty_print_c;
use crate::runtime_c::RUNTIME_HEADER;
use mircap::{Opcode, SymbolId, TypeKind, ValueId};
use mirplan::{
    DataSegmentPlan, LoweredFunction, LoweredInstruction, LoweredOperand, LoweredProgram,
    LoweredValue,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn compile_lowered(lowered: &LoweredProgram, entry_name: &str) -> Result<String, CompileError> {
    let entry_func = find_entry_function(lowered, entry_name)?;
    validate_entry_signature(entry_func)?;

    let mut out = String::new();
    out.push_str(RUNTIME_HEADER);
    out.push('\n');

    out.push_str("/* Forward Declarations */\n");
    for function in &lowered.functions {
        out.push_str(&emit_function_declaration(function)?);
        out.push_str(";\n");
    }
    out.push('\n');

    emit_data_segments(&mut out, &lowered.data_segments);

    for function in &lowered.functions {
        out.push_str(&emit_function_implementation(
            function,
            &lowered.data_segments,
        )?);
        out.push('\n');
    }

    emit_main(&mut out, entry_func)?;
    Ok(pretty_print_c(&out))
}

fn find_entry_function<'a>(
    lowered: &'a LoweredProgram,
    name: &str,
) -> Result<&'a LoweredFunction, CompileError> {
    lowered
        .functions
        .iter()
        .find(|function| function.name == name)
        .ok_or_else(|| CompileError::EntryFunctionNotFound(name.to_string()))
}

fn validate_entry_signature(function: &LoweredFunction) -> Result<(), CompileError> {
    if !function.params.is_empty() {
        return Err(CompileError::InvalidEntrySignature(
            "Entry function must have 0 parameters".to_string(),
        ));
    }
    if function.results.len() > 1 {
        for ret_ty in &function.results {
            if !is_printable_entry_type(*ret_ty) {
                return Err(CompileError::InvalidEntrySignature(format!(
                    "Unsupported entry function result type: {:?}",
                    ret_ty
                )));
            }
        }
    } else if let Some(ret_ty) = function.results.first() {
        if !is_printable_entry_type(*ret_ty) {
            return Err(CompileError::InvalidEntrySignature(format!(
                "Unsupported entry function result type: {:?}",
                ret_ty
            )));
        }
    }
    Ok(())
}

fn is_printable_entry_type(kind: TypeKind) -> bool {
    matches!(
        kind,
        TypeKind::I32
            | TypeKind::U32
            | TypeKind::Addr32
            | TypeKind::I64
            | TypeKind::F32
            | TypeKind::F64
    )
}

fn emit_data_segments(out: &mut String, segments: &[DataSegmentPlan]) {
    out.push_str("/* Data Segments */\n");
    for (idx, segment) in segments.iter().enumerate() {
        out.push_str(&format!("static const uint8_t data_seg_{}[] = {{", idx));
        if segment.bytes.is_empty() {
            out.push_str("0");
        } else {
            let byte_strs = segment
                .bytes
                .iter()
                .map(|byte| format!("0x{byte:02x}"))
                .collect::<Vec<_>>();
            out.push_str(&byte_strs.join(", "));
        }
        out.push_str("};\n");
    }
    out.push('\n');

    out.push_str("void init_data_segments(void) {\n");
    for (idx, segment) in segments.iter().enumerate() {
        let len = segment.bytes.len() as u32;
        out.push_str(&format!("    /* Segment {} */\n", idx));
        if len > 0 {
            out.push_str(&format!(
                "    if ({0}u > MEMORY_SIZE || {1}u > MEMORY_SIZE - {0}u) {{ mir_trap(14); }}\n",
                segment.offset, len
            ));
            out.push_str(&format!(
                "    for (uint32_t i = 0; i < {1}u; i++) {{ g_memory[{0}u + i] = data_seg_{2}[i]; }}\n",
                segment.offset, len, idx
            ));
        }

        let zero_start = segment.offset + len;
        if segment.zero_fill > 0 {
            out.push_str(&format!(
                "    if ({0}u > MEMORY_SIZE || {1}u > MEMORY_SIZE - {0}u) {{ mir_trap(14); }}\n",
                zero_start, segment.zero_fill
            ));
            out.push_str(&format!(
                "    for (uint32_t i = 0; i < {1}u; i++) {{ g_memory[{0}u + i] = 0; }}\n",
                zero_start, segment.zero_fill
            ));
        }

        let end = zero_start + segment.zero_fill;
        out.push_str(&format!(
            "    if ({0}u > g_heap_ptr) {{ g_heap_ptr = {0}u; }}\n",
            end
        ));
    }
    out.push_str("}\n\n");
}

fn emit_function_declaration(function: &LoweredFunction) -> Result<String, CompileError> {
    let ret_str = return_type(function)?;
    let params = emit_function_params(function)?;

    Ok(format!("{} mir_fn_{}({})", ret_str, function.id.0, params))
}

fn return_type(function: &LoweredFunction) -> Result<&'static str, CompileError> {
    if function.results.is_empty() || function.results.len() > 1 {
        Ok("void")
    } else if function.results.len() == 1 {
        emit_type(function.results[0])
    } else {
        Err(CompileError::MultipleResultsNotSupported)
    }
}

fn emit_function_params(function: &LoweredFunction) -> Result<String, CompileError> {
    let mut params = Vec::new();
    for param in &function.params {
        params.push(format!("{} v{}", emit_type(param.type_kind)?, param.id.0));
    }
    if function.results.len() > 1 {
        for (idx, result_type) in function.results.iter().enumerate() {
            params.push(format!("{} *out_{}", emit_type(*result_type)?, idx));
        }
    }
    if params.is_empty() {
        Ok("void".to_string())
    } else {
        Ok(params.join(", "))
    }
}

fn emit_function_implementation(
    function: &LoweredFunction,
    data_segments: &[DataSegmentPlan],
) -> Result<String, CompileError> {
    let mut out = String::new();
    out.push_str(&emit_function_declaration(function)?);
    out.push_str(" {\n");

    let param_ids = function
        .params
        .iter()
        .map(|param| param.id)
        .collect::<BTreeSet<_>>();
    let locals = collect_locals(function, &param_ids);
    for value in locals.values() {
        let type_str = emit_type(value.type_kind)?;
        let init_val = match value.type_kind {
            TypeKind::I32 => "0",
            TypeKind::I64 => "0LL",
            TypeKind::F32 => "0.0f",
            TypeKind::F64 => "0.0",
            _ => "0u",
        };
        out.push_str(&format!(
            "    {} v{} = {};\n",
            type_str, value.id.0, init_val
        ));
    }

    for param in &function.params {
        out.push_str(&format!("    (void)v{};\n", param.id.0));
    }
    if function.results.len() > 1 {
        for idx in 0..function.results.len() {
            out.push_str(&format!("    (void)out_{};\n", idx));
        }
    }
    for value in locals.values() {
        out.push_str(&format!("    (void)v{};\n", value.id.0));
    }

    let used_blocks = used_block_labels(function);
    for block in &function.blocks {
        if used_blocks.contains(&block.label.id.0) {
            out.push_str(&format!("block_{}:\n", block.label.id.0));
        }
        for instruction in &block.instructions {
            out.push_str(&format!(
                "    {}\n",
                emit_lowered_instruction(instruction, function, data_segments)?
            ));
        }
    }

    out.push_str("}\n");
    Ok(out)
}

fn collect_locals(
    function: &LoweredFunction,
    param_ids: &BTreeSet<ValueId>,
) -> BTreeMap<u32, LoweredValue> {
    let mut locals = BTreeMap::new();
    for instruction in function
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
    {
        for value in &instruction.writes {
            if value.type_kind != TypeKind::Void && !param_ids.contains(&value.id) {
                locals.entry(value.id.0).or_insert_with(|| value.clone());
            }
        }
    }
    locals
}

fn used_block_labels(function: &LoweredFunction) -> BTreeSet<u32> {
    function
        .blocks
        .iter()
        .flat_map(|block| block.successors.iter())
        .map(|target| target.block.id.0)
        .collect()
}

fn emit_lowered_instruction(
    instruction: &LoweredInstruction,
    function: &LoweredFunction,
    data_segments: &[DataSegmentPlan],
) -> Result<String, CompileError> {
    match instruction.opcode {
        Opcode::ConstI32
        | Opcode::ConstU32
        | Opcode::ConstI64
        | Opcode::ConstF32
        | Opcode::ConstF64
        | Opcode::Copy => {
            let dest = one_write(instruction)?;
            let val = emit_operand(one_operand(instruction, 0)?);
            Ok(format!("v{} = {};", dest.id.0, val))
        }
        Opcode::AddI32 | Opcode::SubI32 | Opcode::MulI32 => {
            let dest = one_write(instruction)?;
            let lhs = emit_operand(one_operand(instruction, 0)?);
            let rhs = emit_operand(one_operand(instruction, 1)?);
            let op = arithmetic_symbol(instruction.opcode)?;
            Ok(format!(
                "v{} = (int32_t)((uint32_t){} {} (uint32_t){});",
                dest.id.0, lhs, op, rhs
            ))
        }
        Opcode::AddI64 | Opcode::SubI64 | Opcode::MulI64 => {
            let dest = one_write(instruction)?;
            let lhs = emit_operand(one_operand(instruction, 0)?);
            let rhs = emit_operand(one_operand(instruction, 1)?);
            let op = arithmetic_symbol(instruction.opcode)?;
            Ok(format!(
                "v{} = (int64_t)((uint64_t){} {} (uint64_t){});",
                dest.id.0, lhs, op, rhs
            ))
        }
        Opcode::EqI32
        | Opcode::NeI32
        | Opcode::LtI32
        | Opcode::EqI64
        | Opcode::NeI64
        | Opcode::LtI64 => emit_compare(instruction, true),
        Opcode::AddU32 | Opcode::SubU32 | Opcode::MulU32 => {
            let dest = one_write(instruction)?;
            let lhs = emit_operand(one_operand(instruction, 0)?);
            let rhs = emit_operand(one_operand(instruction, 1)?);
            let op = arithmetic_symbol(instruction.opcode)?;
            Ok(format!("v{} = {} {} {};", dest.id.0, lhs, op, rhs))
        }
        Opcode::AddF32
        | Opcode::SubF32
        | Opcode::MulF32
        | Opcode::DivF32
        | Opcode::AddF64
        | Opcode::SubF64
        | Opcode::MulF64
        | Opcode::DivF64 => {
            let dest = one_write(instruction)?;
            let lhs = emit_operand(one_operand(instruction, 0)?);
            let rhs = emit_operand(one_operand(instruction, 1)?);
            let op = arithmetic_symbol(instruction.opcode)?;
            Ok(format!("v{} = {} {} {};", dest.id.0, lhs, op, rhs))
        }
        Opcode::NegF32 | Opcode::NegF64 => {
            let dest = one_write(instruction)?;
            let operand = emit_operand(one_operand(instruction, 0)?);
            Ok(format!("v{} = -{};", dest.id.0, operand))
        }
        Opcode::EqU32
        | Opcode::NeU32
        | Opcode::LtU32
        | Opcode::LeU32
        | Opcode::GtU32
        | Opcode::GeU32 => emit_compare(instruction, false),
        Opcode::Branch => {
            let target = emit_operand(one_operand(instruction, 0)?);
            Ok(format!("goto {};", target))
        }
        Opcode::BranchIf => {
            let cond = emit_operand(one_operand(instruction, 0)?);
            let true_target = emit_operand(one_operand(instruction, 1)?);
            let false_target = emit_operand(one_operand(instruction, 2)?);
            Ok(format!(
                "if ({} != 0) goto {}; else goto {};",
                cond, true_target, false_target
            ))
        }
        Opcode::Call => {
            let callee = emit_operand(one_operand(instruction, 0)?);
            let args = instruction.operands[1..]
                .iter()
                .map(emit_operand)
                .collect::<Vec<_>>()
                .join(", ");
            if instruction.writes.len() > 1 {
                let mut call_args = Vec::new();
                if !args.is_empty() {
                    call_args.push(args);
                }
                for write in &instruction.writes {
                    call_args.push(format!("&v{}", write.id.0));
                }
                Ok(format!("{}({});", callee, call_args.join(", ")))
            } else if instruction.writes.is_empty() {
                Ok(format!("{}({});", callee, args))
            } else if instruction.writes.len() == 1 {
                Ok(format!(
                    "v{} = {}({});",
                    instruction.writes[0].id.0, callee, args
                ))
            } else {
                Err(CompileError::MultipleResultsNotSupported)
            }
        }
        Opcode::Ret => {
            if instruction.operands.is_empty() {
                Ok("return;".to_string())
            } else if function.results.len() > 1 {
                if instruction.operands.len() != function.results.len() {
                    return Err(CompileError::MultipleResultsNotSupported);
                }
                let mut lines = Vec::new();
                for (idx, operand) in instruction.operands.iter().enumerate() {
                    lines.push(format!("*out_{} = {};", idx, emit_operand(operand)));
                }
                lines.push("return;".to_string());
                Ok(lines.join(" "))
            } else if instruction.operands.len() == 1 {
                Ok(format!(
                    "return {};",
                    emit_operand(one_operand(instruction, 0)?)
                ))
            } else {
                Err(CompileError::MultipleResultsNotSupported)
            }
        }
        Opcode::Trap => Ok("mir_trap(3);".to_string()),
        Opcode::Alloc => {
            let dest = one_write(instruction)?;
            let size = emit_operand(one_operand(instruction, 0)?);
            let align = emit_operand(one_operand(instruction, 1)?);
            Ok(format!("v{} = mir_alloc({}, {});", dest.id.0, size, align))
        }
        Opcode::LoadI32 | Opcode::LoadU32 | Opcode::LoadU8 | Opcode::LoadI64 => {
            let dest = one_write(instruction)?;
            let addr = emit_operand(one_operand(instruction, 0)?);
            let helper = match instruction.opcode {
                Opcode::LoadI32 => "mir_load_i32",
                Opcode::LoadU32 => "mir_load_u32",
                Opcode::LoadU8 => "mir_load_u8",
                Opcode::LoadI64 => "mir_load_i64",
                _ => unreachable!(),
            };
            Ok(format!("v{} = {}({});", dest.id.0, helper, addr))
        }
        Opcode::StoreI32 | Opcode::StoreU32 | Opcode::StoreU8 | Opcode::StoreI64 => {
            let addr = emit_operand(one_operand(instruction, 0)?);
            let val = emit_operand(one_operand(instruction, 1)?);
            let helper = match instruction.opcode {
                Opcode::StoreI32 => "mir_store_i32",
                Opcode::StoreU32 => "mir_store_u32",
                Opcode::StoreU8 => "mir_store_u8",
                Opcode::StoreI64 => "mir_store_i64",
                _ => unreachable!(),
            };
            Ok(format!("{}({}, {});", helper, addr, val))
        }
        Opcode::AddrAdd => {
            let dest = one_write(instruction)?;
            let base = emit_operand(one_operand(instruction, 0)?);
            let offset = emit_operand(one_operand(instruction, 1)?);
            Ok(format!(
                "v{} = mir_addr_add({}, {});",
                dest.id.0, base, offset
            ))
        }
        Opcode::DataAddr => {
            let dest = one_write(instruction)?;
            let symbol = symbol_operand(one_operand(instruction, 0)?)?;
            let offset = emit_operand(one_operand(instruction, 1)?);
            let segment = data_segments
                .iter()
                .find(|segment| segment.symbol == symbol)
                .ok_or(CompileError::MultipleResultsNotSupported)?;
            Ok(format!(
                "v{} = mir_data_addr({}u, {}, {}u);",
                dest.id.0, segment.offset, offset, segment.length
            ))
        }
        Opcode::EqF32
        | Opcode::NeF32
        | Opcode::LtF32
        | Opcode::LeF32
        | Opcode::GtF32
        | Opcode::GeF32
        | Opcode::EqF64
        | Opcode::NeF64
        | Opcode::LtF64
        | Opcode::LeF64
        | Opcode::GtF64
        | Opcode::GeF64 => emit_compare(instruction, false),
        Opcode::I32ToF32 => emit_convert(instruction, "float"),
        Opcode::F32ToI32 => emit_convert(instruction, "int32_t"),
        Opcode::I32ToF64 => emit_convert(instruction, "double"),
        Opcode::F64ToI32 => emit_convert(instruction, "int32_t"),
        Opcode::F32ToF64 => emit_convert(instruction, "double"),
        Opcode::F64ToF32 => emit_convert(instruction, "float"),
        Opcode::ExtractValue
        | Opcode::InsertValue
        | Opcode::VaStart
        | Opcode::VaArg
        | Opcode::VaEnd
        | Opcode::UnsupportedIndirectCall => {
            Err(CompileError::UnsupportedOpcode(instruction.opcode))
        }
    }
}

fn emit_convert(
    instruction: &LoweredInstruction,
    c_type: &str,
) -> Result<String, CompileError> {
    let dest = one_write(instruction)?;
    let src = emit_operand(one_operand(instruction, 0)?);
    Ok(format!("v{} = ({}){};", dest.id.0, c_type, src))
}

fn emit_compare(instruction: &LoweredInstruction, _signed: bool) -> Result<String, CompileError> {
    let dest = one_write(instruction)?;
    let lhs = emit_operand(one_operand(instruction, 0)?);
    let rhs = emit_operand(one_operand(instruction, 1)?);
    let op = compare_symbol(instruction.opcode)?;
    Ok(format!(
        "v{} = ({} {} {}) ? 1u : 0u;",
        dest.id.0, lhs, op, rhs
    ))
}

fn one_write(instruction: &LoweredInstruction) -> Result<&LoweredValue, CompileError> {
    if instruction.writes.len() == 1 {
        Ok(&instruction.writes[0])
    } else {
        Err(CompileError::MultipleResultsNotSupported)
    }
}

fn one_operand(
    instruction: &LoweredInstruction,
    index: usize,
) -> Result<&LoweredOperand, CompileError> {
    instruction
        .operands
        .get(index)
        .ok_or(CompileError::MultipleResultsNotSupported)
}

fn emit_operand(operand: &LoweredOperand) -> String {
    match operand {
        LoweredOperand::Value(value) => format!("v{}", value.id.0),
        LoweredOperand::ImmI32(value) => {
            if *value == i32::MIN {
                "((int32_t)(-2147483647 - 1))".to_string()
            } else {
                format!("((int32_t){})", value)
            }
        }
        LoweredOperand::ImmU32(value) => format!("{}u", value),
        LoweredOperand::ImmI64(value) => {
            if *value == i64::MIN {
                "((int64_t)(-9223372036854775807LL - 1LL))".to_string()
            } else {
                format!("((int64_t){}LL)", value)
            }
        }
        LoweredOperand::ImmF32(bits) => format!("mir_f32_from_bits(0x{bits:08x}u)"),
        LoweredOperand::ImmF64(bits) => format!("mir_f64_from_bits(UINT64_C(0x{bits:016x}))"),
        LoweredOperand::Block(block) => format!("block_{}", block.id.0),
        LoweredOperand::Function(function) => format!("mir_fn_{}", function.id.0),
        LoweredOperand::Symbol { id, .. } => format!("sym_{}", id.0),
        LoweredOperand::Type(type_id) => format!("/* type {} */", type_id.0),
    }
}

fn symbol_operand(operand: &LoweredOperand) -> Result<SymbolId, CompileError> {
    match operand {
        LoweredOperand::Symbol { id, .. } => Ok(*id),
        _ => Err(CompileError::MultipleResultsNotSupported),
    }
}

fn arithmetic_symbol(opcode: Opcode) -> Result<&'static str, CompileError> {
    match opcode {
        Opcode::AddI32 | Opcode::AddU32 | Opcode::AddI64 | Opcode::AddF32 | Opcode::AddF64 => {
            Ok("+")
        }
        Opcode::SubI32 | Opcode::SubU32 | Opcode::SubI64 | Opcode::SubF32 | Opcode::SubF64 => {
            Ok("-")
        }
        Opcode::MulI32 | Opcode::MulU32 | Opcode::MulI64 | Opcode::MulF32 | Opcode::MulF64 => {
            Ok("*")
        }
        Opcode::DivF32 | Opcode::DivF64 => Ok("/"),
        _ => Err(CompileError::UnsupportedOpcode(opcode)),
    }
}

fn compare_symbol(opcode: Opcode) -> Result<&'static str, CompileError> {
    match opcode {
        Opcode::EqI32 | Opcode::EqU32 | Opcode::EqI64 | Opcode::EqF32 | Opcode::EqF64 => Ok("=="),
        Opcode::NeI32 | Opcode::NeU32 | Opcode::NeI64 | Opcode::NeF32 | Opcode::NeF64 => Ok("!="),
        Opcode::LtI32 | Opcode::LtU32 | Opcode::LtI64 | Opcode::LtF32 | Opcode::LtF64 => Ok("<"),
        Opcode::LeU32 | Opcode::LeF32 | Opcode::LeF64 => Ok("<="),
        Opcode::GtU32 | Opcode::GtF32 | Opcode::GtF64 => Ok(">"),
        Opcode::GeU32 | Opcode::GeF32 | Opcode::GeF64 => Ok(">="),
        _ => Err(CompileError::UnsupportedOpcode(opcode)),
    }
}

fn emit_main(out: &mut String, entry_func: &LoweredFunction) -> Result<(), CompileError> {
    let entry_ret_type = entry_func
        .results
        .first()
        .copied()
        .unwrap_or(TypeKind::Void);

    out.push_str("int main(void) {\n");
    out.push_str("    init_data_segments();\n");
    if entry_func.results.len() > 1 {
        let mut call_args = Vec::new();
        for (idx, result_type) in entry_func.results.iter().enumerate() {
            out.push_str(&format!(
                "    {} res_{} = {};\n",
                emit_type(*result_type)?,
                idx,
                zero_value(*result_type)?
            ));
            call_args.push(format!("&res_{}", idx));
        }
        out.push_str(&format!(
            "    mir_fn_{}({});\n",
            entry_func.id.0,
            call_args.join(", ")
        ));
        for (idx, result_type) in entry_func.results.iter().enumerate() {
            emit_print_result(out, *result_type, &format!("res_{}", idx))?;
        }
    } else {
        match entry_ret_type {
            TypeKind::Void => {
                out.push_str(&format!("    mir_fn_{}();\n", entry_func.id.0));
                out.push_str("    printf(\"Result: void\\n\");\n");
            }
            TypeKind::I32 => {
                out.push_str(&format!(
                    "    int32_t res = mir_fn_{}();\n",
                    entry_func.id.0
                ));
                out.push_str("    printf(\"Result: i32 %\" PRId32 \"\\n\", res);\n");
            }
            TypeKind::U32 => {
                out.push_str(&format!(
                    "    uint32_t res = mir_fn_{}();\n",
                    entry_func.id.0
                ));
                out.push_str("    printf(\"Result: u32 %\" PRIu32 \"\\n\", res);\n");
            }
            TypeKind::Addr32 => {
                out.push_str(&format!(
                    "    uint32_t res = mir_fn_{}();\n",
                    entry_func.id.0
                ));
                out.push_str("    printf(\"Result: addr32 %\" PRIu32 \"\\n\", res);\n");
            }
            TypeKind::I64 => {
                out.push_str(&format!(
                    "    int64_t res = mir_fn_{}();\n",
                    entry_func.id.0
                ));
                out.push_str("    printf(\"Result: i64 %\" PRId64 \"\\n\", res);\n");
            }
            TypeKind::F32 => {
                out.push_str(&format!("    float res = mir_fn_{}();\n", entry_func.id.0));
                emit_print_result(out, TypeKind::F32, "res")?;
            }
            TypeKind::F64 => {
                out.push_str(&format!("    double res = mir_fn_{}();\n", entry_func.id.0));
                emit_print_result(out, TypeKind::F64, "res")?;
            }
            _ => return Err(CompileError::UnsupportedType(entry_ret_type)),
        }
    }
    out.push_str("    return 0;\n");
    out.push_str("}\n");
    Ok(())
}

fn zero_value(kind: TypeKind) -> Result<&'static str, CompileError> {
    match kind {
        TypeKind::I32 => Ok("0"),
        TypeKind::I64 => Ok("0LL"),
        TypeKind::F32 => Ok("0.0f"),
        TypeKind::F64 => Ok("0.0"),
        TypeKind::U32 | TypeKind::Addr32 => Ok("0u"),
        TypeKind::Void => Ok("0"),
        _ => Err(CompileError::UnsupportedType(kind)),
    }
}

fn emit_print_result(out: &mut String, kind: TypeKind, expr: &str) -> Result<(), CompileError> {
    match kind {
        TypeKind::I32 => out.push_str(&format!(
            "    printf(\"Result: i32 %\" PRId32 \"\\n\", {});\n",
            expr
        )),
        TypeKind::U32 => out.push_str(&format!(
            "    printf(\"Result: u32 %\" PRIu32 \"\\n\", {});\n",
            expr
        )),
        TypeKind::Addr32 => out.push_str(&format!(
            "    printf(\"Result: addr32 %\" PRIu32 \"\\n\", {});\n",
            expr
        )),
        TypeKind::I64 => out.push_str(&format!(
            "    printf(\"Result: i64 %\" PRId64 \"\\n\", {});\n",
            expr
        )),
        TypeKind::F32 => out.push_str(&format!(
            "    printf(\"Result: f32 %.9g bits=0x%08\" PRIx32 \"\\n\", (double){0}, mir_f32_to_bits({0}));\n",
            expr
        )),
        TypeKind::F64 => out.push_str(&format!(
            "    printf(\"Result: f64 %.17g bits=0x%016\" PRIx64 \"\\n\", {0}, mir_f64_to_bits({0}));\n",
            expr
        )),
        TypeKind::Void => out.push_str("    printf(\"Result: void\\n\");\n"),
        _ => return Err(CompileError::UnsupportedType(kind)),
    }
    Ok(())
}

pub struct C11Backend {
    pub entry_name: String,
}

impl C11Backend {
    pub fn new(entry_name: impl Into<String>) -> Self {
        Self {
            entry_name: entry_name.into(),
        }
    }
}

impl mirplan::Backend for C11Backend {
    type Output = String;
    type Error = CompileError;

    fn compile(&self, program: &LoweredProgram) -> Result<Self::Output, Self::Error> {
        compile_lowered(program, &self.entry_name)
    }
}
