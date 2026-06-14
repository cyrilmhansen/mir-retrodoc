use crate::{
    CompilePlan, FunctionPlan, LoweredBranchTarget, LoweredInstructionKind, LoweredMemoryOp,
    LoweredProgram, LoweredValue, OperandPlan, ValuePlan,
};
use mircap::{Opcode, TypeKind};
use mirspace::EdgeKind;

pub fn format_plan(plan: &CompilePlan) -> String {
    let mut out = String::new();
    out.push_str(&format!("module {}\n", plan.module_name));

    if !plan.data_segments.is_empty() {
        out.push_str("data:\n");
        for segment in &plan.data_segments {
            out.push_str(&format!(
                "  symbol#{} {} offset={} length={}\n",
                segment.symbol.0, segment.name, segment.offset, segment.length
            ));
        }
    }

    out.push_str("functions:\n");
    for function in &plan.functions {
        format_function(&mut out, function);
    }

    out
}

pub fn format_lowered(program: &LoweredProgram) -> String {
    let mut out = String::new();
    out.push_str(&format!("lowered module {}\n", program.module_name));

    if !program.data_segments.is_empty() {
        out.push_str("data:\n");
        for segment in &program.data_segments {
            out.push_str(&format!(
                "  symbol#{} {} offset={} length={}\n",
                segment.symbol.0, segment.name, segment.offset, segment.length
            ));
        }
    }

    out.push_str("functions:\n");

    for function in &program.functions {
        out.push_str(&format!(
            "  fn f{}#{} {} entry=b{}#{}\n",
            function.ix.0, function.id.0, function.name, function.entry.ix.0, function.entry.id.0
        ));
        out.push_str(&format!(
            "    params: {}\n",
            format_lowered_values(&function.params)
        ));
        out.push_str(&format!(
            "    results: {}\n",
            format_types(&function.results)
        ));

        for block in &function.blocks {
            out.push_str(&format!(
                "    block b{}#{}\n",
                block.label.ix.0, block.label.id.0
            ));
            for instruction in &block.instructions {
                let writes = format_lowered_values(&instruction.writes);
                let reads = format_lowered_values(&instruction.reads);
                out.push_str(&format!(
                    "      i{}#{} {} {} writes=[{}] reads=[{}]{}\n",
                    instruction.ix.0,
                    instruction.id.0,
                    format_lowered_kind_name(&instruction.kind),
                    format_opcode(instruction.opcode),
                    writes,
                    reads,
                    format_lowered_kind_detail(&instruction.kind)
                ));
            }

            if !block.successors.is_empty() {
                out.push_str(&format!(
                    "      successors: {}\n",
                    format_lowered_targets(&block.successors)
                ));
            }
        }
    }

    out
}

fn format_function(out: &mut String, function: &FunctionPlan) {
    out.push_str(&format!(
        "  fn f{}#{} {} entry=b{}\n",
        function.ix.0, function.id.0, function.name, function.entry.0
    ));
    out.push_str(&format!(
        "    params: {}\n",
        format_values(&function.params)
    ));
    out.push_str(&format!(
        "    results: {}\n",
        format_types(&function.results)
    ));

    for block in &function.blocks {
        out.push_str(&format!("    block b{}#{}\n", block.ix.0, block.id.0));
        for instruction in &block.instructions {
            let results = format_values(&instruction.results);
            let operands = instruction
                .operands
                .iter()
                .map(format_operand)
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "      i{}#{} {} -> [{}] [{}]\n",
                instruction.ix.0,
                instruction.id.0,
                format_opcode(instruction.opcode),
                results,
                operands
            ));
        }

        if !block.successors.is_empty() {
            let successors = block
                .successors
                .iter()
                .map(|edge| {
                    format!(
                        "{}:b{}#{}",
                        format_edge_kind(edge.kind),
                        edge.target.0,
                        edge.target_id.0
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("      successors: {successors}\n"));
        }
    }

    if !function.call_sites.is_empty() {
        out.push_str("    call-sites:\n");
        for call in &function.call_sites {
            out.push_str(&format!(
                "      i{}#{} -> f{}#{} {}\n",
                call.instruction.0,
                call.instruction_id.0,
                call.callee.0,
                call.callee_id.0,
                call.callee_name
            ));
        }
    }

    if !function.memory_ops.is_empty() {
        out.push_str("    memory-ops:\n");
        for op in &function.memory_ops {
            out.push_str(&format!(
                "      i{}#{} {}\n",
                op.instruction.0,
                op.instruction_id.0,
                format_opcode(op.opcode)
            ));
        }
    }
}

fn format_values(values: &[ValuePlan]) -> String {
    if values.is_empty() {
        return "-".to_string();
    }

    values
        .iter()
        .map(|value| format_value(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_lowered_values(values: &[LoweredValue]) -> String {
    if values.is_empty() {
        return "-".to_string();
    }

    values
        .iter()
        .map(|value| {
            format!(
                "v{}#{}:{}",
                value.ix.0,
                value.id.0,
                format_type(value.type_kind)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_value(value: &ValuePlan) -> String {
    format!(
        "v{}#{}:{}",
        value.ix.0,
        value.id.0,
        format_type(value.type_kind)
    )
}

fn format_types(types: &[TypeKind]) -> String {
    if types.is_empty() {
        return "-".to_string();
    }

    types
        .iter()
        .map(|&ty| format_type(ty))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_operand(operand: &OperandPlan) -> String {
    match operand {
        OperandPlan::Value(value) => format_value(value),
        OperandPlan::ImmI32(value) => format!("i32:{value}"),
        OperandPlan::ImmU32(value) => format!("u32:{value}"),
        OperandPlan::Block { ix, id } => format!("b{}#{}", ix.0, id.0),
        OperandPlan::Function { ix, id, name } => format!("f{}#{}:{name}", ix.0, id.0),
        OperandPlan::Symbol { ix, id, name } => format!("s{}#{}:{name}", ix.0, id.0),
        OperandPlan::Type(type_id) => format!("type#{}", type_id.0),
    }
}

fn format_type(kind: TypeKind) -> &'static str {
    match kind {
        TypeKind::Void => "void",
        TypeKind::I32 => "i32",
        TypeKind::U32 => "u32",
        TypeKind::Addr32 => "addr32",
        TypeKind::UnsupportedI64 => "unsupported_i64",
        TypeKind::UnsupportedFloat => "unsupported_float",
        TypeKind::UnsupportedLongDouble => "unsupported_long_double",
        TypeKind::UnsupportedAggregate => "unsupported_aggregate",
        TypeKind::UnsupportedVarargs => "unsupported_varargs",
        TypeKind::UnsupportedHostCAbi => "unsupported_host_c_abi",
    }
}

fn format_opcode(opcode: Opcode) -> &'static str {
    match opcode {
        Opcode::ConstI32 => "const_i32",
        Opcode::ConstU32 => "const_u32",
        Opcode::Copy => "copy",
        Opcode::AddI32 => "add_i32",
        Opcode::SubI32 => "sub_i32",
        Opcode::MulI32 => "mul_i32",
        Opcode::EqI32 => "eq_i32",
        Opcode::NeI32 => "ne_i32",
        Opcode::LtI32 => "lt_i32",
        Opcode::AddU32 => "add_u32",
        Opcode::SubU32 => "sub_u32",
        Opcode::MulU32 => "mul_u32",
        Opcode::EqU32 => "eq_u32",
        Opcode::NeU32 => "ne_u32",
        Opcode::LtU32 => "lt_u32",
        Opcode::LeU32 => "le_u32",
        Opcode::GtU32 => "gt_u32",
        Opcode::GeU32 => "ge_u32",
        Opcode::Branch => "branch",
        Opcode::BranchIf => "branch_if",
        Opcode::Call => "call",
        Opcode::Ret => "ret",
        Opcode::Trap => "trap",
        Opcode::Alloc => "alloc",
        Opcode::LoadI32 => "load_i32",
        Opcode::LoadU32 => "load_u32",
        Opcode::StoreI32 => "store_i32",
        Opcode::StoreU32 => "store_u32",
        Opcode::LoadU8 => "load_u8",
        Opcode::StoreU8 => "store_u8",
        Opcode::AddrAdd => "addr_add",
        Opcode::DataAddr => "data_addr",
        Opcode::UnsupportedI64 => "unsupported_i64",
        Opcode::UnsupportedIndirectCall => "unsupported_indirect_call",
    }
}

fn format_edge_kind(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::Unconditional => "branch",
        EdgeKind::TrueBranch => "true",
        EdgeKind::FalseBranch => "false",
    }
}

fn format_lowered_kind_name(kind: &LoweredInstructionKind) -> &'static str {
    match kind {
        LoweredInstructionKind::Value => "value",
        LoweredInstructionKind::Branch { .. } => "branch",
        LoweredInstructionKind::Call { .. } => "call",
        LoweredInstructionKind::Return => "return",
        LoweredInstructionKind::Trap => "trap",
        LoweredInstructionKind::Memory { .. } => "memory",
    }
}

fn format_lowered_kind_detail(kind: &LoweredInstructionKind) -> String {
    match kind {
        LoweredInstructionKind::Branch { targets } => {
            format!(" targets=[{}]", format_lowered_targets(targets))
        }
        LoweredInstructionKind::Call { callee } => {
            format!(" callee=f{}#{} {}", callee.ix.0, callee.id.0, callee.name)
        }
        LoweredInstructionKind::Memory { op } => {
            format!(" op={}", format_lowered_memory_op(op))
        }
        _ => String::new(),
    }
}

fn format_lowered_targets(targets: &[LoweredBranchTarget]) -> String {
    targets
        .iter()
        .map(|target| {
            format!(
                "{}:b{}#{}",
                format_edge_kind(target.kind),
                target.block.ix.0,
                target.block.id.0
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_lowered_memory_op(op: &LoweredMemoryOp) -> &'static str {
    match op {
        LoweredMemoryOp::Alloc => "alloc",
        LoweredMemoryOp::Load => "load",
        LoweredMemoryOp::Store => "store",
        LoweredMemoryOp::Address => "address",
    }
}
