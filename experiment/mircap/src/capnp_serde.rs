use crate::mircap_capnp;

use crate::ids::{BlockId, FunctionId, InstructionId, SourceSpanId, SymbolId, TypeId, ValueId};
use crate::image::{
    Block, DataSegment, Function, Header, Instruction, Module, ModuleImage, Opcode, Operand,
    Symbol, SymbolKind, TypeDef, TypeKind,
};
use capnp::message::Builder;
use capnp::serialize;

pub fn to_capnp_bytes(image: &ModuleImage) -> Vec<u8> {
    let mut message = Builder::new_default();
    {
        let mut root = message.init_root::<mircap_capnp::module_image::Builder>();

        // 1. Header
        {
            let mut header = root.reborrow().init_header();
            header.set_schema_name(&image.header.schema_name);
            header.set_format_version(image.header.format_version);
            header.set_producer_name(&image.header.producer_name);
            header.set_producer_version(&image.header.producer_version);
            if let Some(ref lang) = image.header.source_language {
                header.set_source_language(lang);
            }
            if let Some(ref assumptions) = image.header.target_assumptions {
                header.set_target_assumptions(assumptions);
            }
            let mut flags = header.init_feature_flags(image.header.feature_flags.len() as u32);
            for (i, flag) in image.header.feature_flags.iter().enumerate() {
                flags.set(i as u32, flag);
            }
        }

        // 2. Module
        {
            let mut module = root.reborrow().init_module();
            module.set_id(image.module.id);
            module.set_name(&image.module.name);
        }

        // 3. Types
        {
            let mut types = root.reborrow().init_types(image.types.len() as u32);
            for (i, ty) in image.types.iter().enumerate() {
                let mut capnp_ty = types.reborrow().get(i as u32);
                capnp_ty.set_id(ty.id.0);
                let capnp_kind = match ty.kind {
                    TypeKind::Void => mircap_capnp::TypeKind::Void,
                    TypeKind::I32 => mircap_capnp::TypeKind::I32,
                    TypeKind::U32 => mircap_capnp::TypeKind::U32,
                    TypeKind::Addr32 => mircap_capnp::TypeKind::Addr32,
                    TypeKind::I64 => mircap_capnp::TypeKind::I64,
                    TypeKind::UnsupportedFloat => mircap_capnp::TypeKind::UnsupportedFloat,
                    TypeKind::UnsupportedLongDouble => {
                        mircap_capnp::TypeKind::UnsupportedLongDouble
                    }
                    TypeKind::UnsupportedAggregate => mircap_capnp::TypeKind::UnsupportedAggregate,
                    TypeKind::UnsupportedVarargs => mircap_capnp::TypeKind::UnsupportedVarargs,
                    TypeKind::UnsupportedHostCAbi => mircap_capnp::TypeKind::UnsupportedHostCAbi,
                    TypeKind::F32 => mircap_capnp::TypeKind::F32,
                    TypeKind::F64 => mircap_capnp::TypeKind::F64,
                };
                capnp_ty.set_kind(capnp_kind);
            }
        }

        // 4. Symbols
        {
            let mut symbols = root.reborrow().init_symbols(image.symbols.len() as u32);
            for (i, sym) in image.symbols.iter().enumerate() {
                let mut capnp_sym = symbols.reborrow().get(i as u32);
                capnp_sym.set_id(sym.id.0);
                capnp_sym.set_name(&sym.name);
                let capnp_kind = match sym.kind {
                    SymbolKind::Function => mircap_capnp::SymbolKind::Function,
                    SymbolKind::Data => mircap_capnp::SymbolKind::Data,
                    SymbolKind::RuntimeHelper => mircap_capnp::SymbolKind::RuntimeHelper,
                };
                capnp_sym.set_kind(capnp_kind);
            }
        }

        // Flat lists preparation
        let mut flat_blocks = Vec::new();
        let mut func_block_ranges = Vec::new();
        for func in &image.functions {
            let first_block = flat_blocks.len() as u32;
            let count = func.blocks.len() as u32;
            for &block_id in &func.blocks {
                if let Some(block) = image.blocks.iter().find(|b| b.id == block_id) {
                    flat_blocks.push(block);
                }
            }
            func_block_ranges.push((first_block, count));
        }

        let mut flat_insns = Vec::new();
        let mut block_insn_ranges = Vec::new();
        for block in &flat_blocks {
            let first_insn = flat_insns.len() as u32;
            let count = block.instructions.len() as u32;
            for &insn_id in &block.instructions {
                if let Some(insn) = image.instructions.iter().find(|i| i.id == insn_id) {
                    flat_insns.push(insn);
                }
            }
            block_insn_ranges.push((first_insn, count));
        }

        let mut flat_operands = Vec::new();
        let mut insn_operand_ranges = Vec::new();
        let mut flat_results = Vec::new();
        let mut insn_result_ranges = Vec::new();
        for insn in &flat_insns {
            let first_op = flat_operands.len() as u32;
            let op_count = insn.operands.len() as u32;
            for op in &insn.operands {
                flat_operands.push(op);
            }
            insn_operand_ranges.push((first_op, op_count));

            let first_res = flat_results.len() as u32;
            let res_count = insn.results.len() as u32;
            for &res in &insn.results {
                flat_results.push(res);
            }
            insn_result_ranges.push((first_res, res_count));
        }

        // 5. Functions
        {
            let mut functions = root.reborrow().init_functions(image.functions.len() as u32);
            for (i, func) in image.functions.iter().enumerate() {
                let mut capnp_func = functions.reborrow().get(i as u32);
                capnp_func.set_id(func.id.0);
                capnp_func.set_symbol(func.symbol.0);

                let mut params = capnp_func.reborrow().init_params(func.params.len() as u32);
                for (j, &p) in func.params.iter().enumerate() {
                    params.set(j as u32, p.0);
                }

                let mut results = capnp_func
                    .reborrow()
                    .init_results(func.results.len() as u32);
                for (j, &r) in func.results.iter().enumerate() {
                    results.set(j as u32, r.0);
                }

                capnp_func.set_value_count(func.value_count);

                let mut val_types = capnp_func
                    .reborrow()
                    .init_value_types(func.value_types.len() as u32);
                for (j, &t) in func.value_types.iter().enumerate() {
                    val_types.set(j as u32, t.0);
                }

                let (first_block, block_count) = func_block_ranges[i];
                capnp_func.set_first_block(first_block);
                capnp_func.set_block_count(block_count);
                capnp_func.set_flags(func.flags);
                capnp_func.set_source_span(func.source_span.map(|id| id.0).unwrap_or(0));
            }
        }

        // 6. Data Segments
        {
            let mut data_segs = root
                .reborrow()
                .init_data_segments(image.data_segments.len() as u32);
            for (i, ds) in image.data_segments.iter().enumerate() {
                let mut capnp_ds = data_segs.reborrow().get(i as u32);
                capnp_ds.set_symbol(ds.symbol.0);
                capnp_ds.set_offset(ds.offset);
                capnp_ds.set_bytes(&ds.bytes);
                capnp_ds.set_zero_fill(ds.zero_fill);
            }
        }

        // 7. Blocks
        {
            let mut blocks = root.reborrow().init_blocks(flat_blocks.len() as u32);
            for (i, block) in flat_blocks.iter().enumerate() {
                let mut capnp_block = blocks.reborrow().get(i as u32);
                capnp_block.set_id(block.id.0);
                capnp_block.set_parent_function(block.parent.0);

                let (first_insn, insn_count) = block_insn_ranges[i];
                capnp_block.set_first_instruction(first_insn);
                capnp_block.set_instruction_count(insn_count);
                capnp_block.set_terminator(block.terminator.0);
                capnp_block.set_source_span(block.source_span.map(|id| id.0).unwrap_or(0));
            }
        }

        // 8. Instructions
        {
            let mut instructions = root.reborrow().init_instructions(flat_insns.len() as u32);
            for (i, insn) in flat_insns.iter().enumerate() {
                let mut capnp_insn = instructions.reborrow().get(i as u32);
                capnp_insn.set_id(insn.id.0);

                let capnp_op = match insn.opcode {
                    Opcode::ConstI32 => mircap_capnp::Opcode::ConstI32,
                    Opcode::ConstU32 => mircap_capnp::Opcode::ConstU32,
                    Opcode::Copy => mircap_capnp::Opcode::Copy,
                    Opcode::AddI32 => mircap_capnp::Opcode::AddI32,
                    Opcode::SubI32 => mircap_capnp::Opcode::SubI32,
                    Opcode::MulI32 => mircap_capnp::Opcode::MulI32,
                    Opcode::EqI32 => mircap_capnp::Opcode::EqI32,
                    Opcode::NeI32 => mircap_capnp::Opcode::NeI32,
                    Opcode::LtI32 => mircap_capnp::Opcode::LtI32,
                    Opcode::AddU32 => mircap_capnp::Opcode::AddU32,
                    Opcode::SubU32 => mircap_capnp::Opcode::SubU32,
                    Opcode::MulU32 => mircap_capnp::Opcode::MulU32,
                    Opcode::EqU32 => mircap_capnp::Opcode::EqU32,
                    Opcode::NeU32 => mircap_capnp::Opcode::NeU32,
                    Opcode::LtU32 => mircap_capnp::Opcode::LtU32,
                    Opcode::LeU32 => mircap_capnp::Opcode::LeU32,
                    Opcode::GtU32 => mircap_capnp::Opcode::GtU32,
                    Opcode::GeU32 => mircap_capnp::Opcode::GeU32,
                    Opcode::Branch => mircap_capnp::Opcode::Branch,
                    Opcode::BranchIf => mircap_capnp::Opcode::BranchIf,
                    Opcode::Call => mircap_capnp::Opcode::Call,
                    Opcode::Ret => mircap_capnp::Opcode::Ret,
                    Opcode::Trap => mircap_capnp::Opcode::Trap,
                    Opcode::Alloc => mircap_capnp::Opcode::Alloc,
                    Opcode::LoadI32 => mircap_capnp::Opcode::LoadI32,
                    Opcode::LoadU32 => mircap_capnp::Opcode::LoadU32,
                    Opcode::StoreI32 => mircap_capnp::Opcode::StoreI32,
                    Opcode::StoreU32 => mircap_capnp::Opcode::StoreU32,
                    Opcode::LoadU8 => mircap_capnp::Opcode::LoadU8,
                    Opcode::StoreU8 => mircap_capnp::Opcode::StoreU8,
                    Opcode::AddrAdd => mircap_capnp::Opcode::AddrAdd,
                    Opcode::DataAddr => mircap_capnp::Opcode::DataAddr,
                    Opcode::ConstI64 => mircap_capnp::Opcode::ConstI64,
                    Opcode::UnsupportedIndirectCall => {
                        mircap_capnp::Opcode::UnsupportedIndirectCall
                    }
                    Opcode::AddI64 => mircap_capnp::Opcode::AddI64,
                    Opcode::SubI64 => mircap_capnp::Opcode::SubI64,
                    Opcode::MulI64 => mircap_capnp::Opcode::MulI64,
                    Opcode::EqI64 => mircap_capnp::Opcode::EqI64,
                    Opcode::NeI64 => mircap_capnp::Opcode::NeI64,
                    Opcode::LtI64 => mircap_capnp::Opcode::LtI64,
                    Opcode::LoadI64 => mircap_capnp::Opcode::LoadI64,
                    Opcode::StoreI64 => mircap_capnp::Opcode::StoreI64,
                    Opcode::ConstF32 => mircap_capnp::Opcode::ConstF32,
                    Opcode::ConstF64 => mircap_capnp::Opcode::ConstF64,
                    Opcode::AddF32 => mircap_capnp::Opcode::AddF32,
                    Opcode::SubF32 => mircap_capnp::Opcode::SubF32,
                    Opcode::MulF32 => mircap_capnp::Opcode::MulF32,
                    Opcode::DivF32 => mircap_capnp::Opcode::DivF32,
                    Opcode::NegF32 => mircap_capnp::Opcode::NegF32,
                    Opcode::EqF32 => mircap_capnp::Opcode::EqF32,
                    Opcode::NeF32 => mircap_capnp::Opcode::NeF32,
                    Opcode::LtF32 => mircap_capnp::Opcode::LtF32,
                    Opcode::LeF32 => mircap_capnp::Opcode::LeF32,
                    Opcode::GtF32 => mircap_capnp::Opcode::GtF32,
                    Opcode::GeF32 => mircap_capnp::Opcode::GeF32,
                    Opcode::AddF64 => mircap_capnp::Opcode::AddF64,
                    Opcode::SubF64 => mircap_capnp::Opcode::SubF64,
                    Opcode::MulF64 => mircap_capnp::Opcode::MulF64,
                    Opcode::DivF64 => mircap_capnp::Opcode::DivF64,
                    Opcode::NegF64 => mircap_capnp::Opcode::NegF64,
                    Opcode::EqF64 => mircap_capnp::Opcode::EqF64,
                    Opcode::NeF64 => mircap_capnp::Opcode::NeF64,
                    Opcode::LtF64 => mircap_capnp::Opcode::LtF64,
                    Opcode::LeF64 => mircap_capnp::Opcode::LeF64,
                    Opcode::GtF64 => mircap_capnp::Opcode::GtF64,
                    Opcode::GeF64 => mircap_capnp::Opcode::GeF64,
                    Opcode::I32ToF32 => mircap_capnp::Opcode::I32ToF32,
                    Opcode::F32ToI32 => mircap_capnp::Opcode::F32ToI32,
                    Opcode::I32ToF64 => mircap_capnp::Opcode::I32ToF64,
                    Opcode::F64ToI32 => mircap_capnp::Opcode::F64ToI32,
                    Opcode::F32ToF64 => mircap_capnp::Opcode::F32ToF64,
                    Opcode::F64ToF32 => mircap_capnp::Opcode::F64ToF32,
                };
                capnp_insn.set_opcode(capnp_op);

                let (first_res, res_count) = insn_result_ranges[i];
                capnp_insn.set_first_result(first_res);
                capnp_insn.set_result_count(res_count);

                let (first_op, op_count) = insn_operand_ranges[i];
                capnp_insn.set_first_operand(first_op);
                capnp_insn.set_operand_count(op_count);

                capnp_insn.set_source_span(insn.source_span.map(|id| id.0).unwrap_or(0));
            }
        }

        // 9. Operands
        {
            let mut operands = root.reborrow().init_operands(flat_operands.len() as u32);
            for (i, op) in flat_operands.iter().enumerate() {
                let mut capnp_op = operands.reborrow().get(i as u32);
                match **op {
                    Operand::Value(val) => capnp_op.set_value(val.0),
                    Operand::ImmI32(val) => capnp_op.set_imm_i32(val),
                    Operand::ImmU32(val) => capnp_op.set_imm_u32(val),
                    Operand::ImmI64(val) => capnp_op.set_imm_i64(val),
                    Operand::ImmF32(val) => capnp_op.set_imm_f32(f32::from_bits(val)),
                    Operand::ImmF64(val) => capnp_op.set_imm_f64(f64::from_bits(val)),
                    Operand::Block(val) => capnp_op.set_block(val.0),
                    Operand::Function(val) => capnp_op.set_function(val.0),
                    Operand::Symbol(val) => capnp_op.set_symbol(val.0),
                    Operand::Type(val) => capnp_op.set_type(val.0),
                }
            }
        }

        // 10. Results
        {
            let mut results = root.reborrow().init_results(flat_results.len() as u32);
            for (i, &res) in flat_results.iter().enumerate() {
                results.set(i as u32, res.0);
            }
        }

        // 11. Source Spans
        root.reborrow().init_source_spans(0);

        // 12. Metadata
        root.reborrow().init_metadata(0);
    }

    let mut bytes = Vec::new();
    serialize::write_message(&mut bytes, &message).expect("write message failed");
    bytes
}

pub fn from_capnp_bytes(bytes: &[u8]) -> Result<ModuleImage, capnp::Error> {
    let reader = serialize::read_message(bytes, capnp::message::ReaderOptions::new())?;
    let root = reader.get_root::<mircap_capnp::module_image::Reader>()?;

    // 1. Header
    let capnp_header = root.get_header()?;
    let schema_name = capnp_header.get_schema_name()?.to_string()?;
    let format_version = capnp_header.get_format_version();
    let producer_name = capnp_header.get_producer_name()?.to_string()?;
    let producer_version = capnp_header.get_producer_version()?.to_string()?;

    let source_lang_str = capnp_header.get_source_language()?.to_string()?;
    let source_language = if source_lang_str.is_empty() {
        None
    } else {
        Some(source_lang_str)
    };

    let target_ass_str = capnp_header.get_target_assumptions()?.to_string()?;
    let target_assumptions = if target_ass_str.is_empty() {
        None
    } else {
        Some(target_ass_str)
    };

    let capnp_flags = capnp_header.get_feature_flags()?;
    let mut feature_flags = Vec::new();
    for i in 0..capnp_flags.len() {
        feature_flags.push(capnp_flags.get(i)?.to_string()?);
    }

    let header = Header {
        schema_name,
        format_version,
        producer_name,
        producer_version,
        source_language,
        target_assumptions,
        feature_flags,
    };

    // 2. Module
    let capnp_module = root.get_module()?;
    let module = Module {
        id: capnp_module.get_id(),
        name: capnp_module.get_name()?.to_string()?,
    };

    // 3. Types
    let capnp_types = root.get_types()?;
    let mut types = Vec::new();
    for i in 0..capnp_types.len() {
        let ty = capnp_types.get(i);
        let id = TypeId(ty.get_id());
        let kind = match ty.get_kind()? {
            mircap_capnp::TypeKind::Void => TypeKind::Void,
            mircap_capnp::TypeKind::I32 => TypeKind::I32,
            mircap_capnp::TypeKind::U32 => TypeKind::U32,
            mircap_capnp::TypeKind::Addr32 => TypeKind::Addr32,
            mircap_capnp::TypeKind::I64 => TypeKind::I64,
            mircap_capnp::TypeKind::UnsupportedFloat => TypeKind::UnsupportedFloat,
            mircap_capnp::TypeKind::UnsupportedLongDouble => TypeKind::UnsupportedLongDouble,
            mircap_capnp::TypeKind::UnsupportedAggregate => TypeKind::UnsupportedAggregate,
            mircap_capnp::TypeKind::UnsupportedVarargs => TypeKind::UnsupportedVarargs,
            mircap_capnp::TypeKind::UnsupportedHostCAbi => TypeKind::UnsupportedHostCAbi,
            mircap_capnp::TypeKind::F32 => TypeKind::F32,
            mircap_capnp::TypeKind::F64 => TypeKind::F64,
        };
        types.push(TypeDef { id, kind });
    }

    // 4. Symbols
    let capnp_symbols = root.get_symbols()?;
    let mut symbols = Vec::new();
    for i in 0..capnp_symbols.len() {
        let sym = capnp_symbols.get(i);
        let id = SymbolId(sym.get_id());
        let name = sym.get_name()?.to_string()?;
        let kind = match sym.get_kind()? {
            mircap_capnp::SymbolKind::Function => SymbolKind::Function,
            mircap_capnp::SymbolKind::Data => SymbolKind::Data,
            mircap_capnp::SymbolKind::RuntimeHelper => SymbolKind::RuntimeHelper,
        };
        symbols.push(Symbol { id, name, kind });
    }

    // 5. Data Segments
    let capnp_data_segs = root.get_data_segments()?;
    let mut data_segments = Vec::new();
    for i in 0..capnp_data_segs.len() {
        let ds = capnp_data_segs.get(i);
        let symbol = SymbolId(ds.get_symbol());
        let offset = ds.get_offset();
        let bytes = ds.get_bytes()?.to_vec();
        let zero_fill = ds.get_zero_fill();
        data_segments.push(DataSegment {
            symbol,
            offset,
            bytes,
            zero_fill,
        });
    }

    // Flat tables
    let capnp_blocks = root.get_blocks()?;
    let capnp_instructions = root.get_instructions()?;
    let capnp_operands = root.get_operands()?;
    let capnp_results = root.get_results()?;

    // Read flat results list
    let mut results_list = Vec::new();
    for i in 0..capnp_results.len() {
        results_list.push(ValueId(capnp_results.get(i)));
    }

    // Read flat operands list
    let mut operands_list = Vec::new();
    for i in 0..capnp_operands.len() {
        let capnp_op = capnp_operands.get(i);
        let op = match capnp_op.which()? {
            mircap_capnp::operand::Which::Value(val) => Operand::Value(ValueId(val)),
            mircap_capnp::operand::Which::ImmI32(val) => Operand::ImmI32(val),
            mircap_capnp::operand::Which::ImmU32(val) => Operand::ImmU32(val),
            mircap_capnp::operand::Which::Block(val) => Operand::Block(BlockId(val)),
            mircap_capnp::operand::Which::Function(val) => Operand::Function(FunctionId(val)),
            mircap_capnp::operand::Which::Symbol(val) => Operand::Symbol(SymbolId(val)),
            mircap_capnp::operand::Which::Type(val) => Operand::Type(TypeId(val)),
            mircap_capnp::operand::Which::ImmI64(val) => Operand::ImmI64(val),
            mircap_capnp::operand::Which::ImmF32(val) => Operand::ImmF32(val.to_bits()),
            mircap_capnp::operand::Which::ImmF64(val) => Operand::ImmF64(val.to_bits()),
        };
        operands_list.push(op);
    }

    // Read flat instructions list
    let mut instructions = Vec::new();
    for i in 0..capnp_instructions.len() {
        let insn = capnp_instructions.get(i);
        let id = InstructionId(insn.get_id());

        let opcode = match insn.get_opcode()? {
            mircap_capnp::Opcode::ConstI32 => Opcode::ConstI32,
            mircap_capnp::Opcode::ConstU32 => Opcode::ConstU32,
            mircap_capnp::Opcode::Copy => Opcode::Copy,
            mircap_capnp::Opcode::AddI32 => Opcode::AddI32,
            mircap_capnp::Opcode::SubI32 => Opcode::SubI32,
            mircap_capnp::Opcode::MulI32 => Opcode::MulI32,
            mircap_capnp::Opcode::EqI32 => Opcode::EqI32,
            mircap_capnp::Opcode::NeI32 => Opcode::NeI32,
            mircap_capnp::Opcode::LtI32 => Opcode::LtI32,
            mircap_capnp::Opcode::AddU32 => Opcode::AddU32,
            mircap_capnp::Opcode::SubU32 => Opcode::SubU32,
            mircap_capnp::Opcode::MulU32 => Opcode::MulU32,
            mircap_capnp::Opcode::EqU32 => Opcode::EqU32,
            mircap_capnp::Opcode::NeU32 => Opcode::NeU32,
            mircap_capnp::Opcode::LtU32 => Opcode::LtU32,
            mircap_capnp::Opcode::LeU32 => Opcode::LeU32,
            mircap_capnp::Opcode::GtU32 => Opcode::GtU32,
            mircap_capnp::Opcode::GeU32 => Opcode::GeU32,
            mircap_capnp::Opcode::Branch => Opcode::Branch,
            mircap_capnp::Opcode::BranchIf => Opcode::BranchIf,
            mircap_capnp::Opcode::Call => Opcode::Call,
            mircap_capnp::Opcode::Ret => Opcode::Ret,
            mircap_capnp::Opcode::Trap => Opcode::Trap,
            mircap_capnp::Opcode::Alloc => Opcode::Alloc,
            mircap_capnp::Opcode::LoadI32 => Opcode::LoadI32,
            mircap_capnp::Opcode::LoadU32 => Opcode::LoadU32,
            mircap_capnp::Opcode::StoreI32 => Opcode::StoreI32,
            mircap_capnp::Opcode::StoreU32 => Opcode::StoreU32,
            mircap_capnp::Opcode::LoadU8 => Opcode::LoadU8,
            mircap_capnp::Opcode::StoreU8 => Opcode::StoreU8,
            mircap_capnp::Opcode::AddrAdd => Opcode::AddrAdd,
            mircap_capnp::Opcode::DataAddr => Opcode::DataAddr,
            mircap_capnp::Opcode::ConstI64 => Opcode::ConstI64,
            mircap_capnp::Opcode::UnsupportedIndirectCall => Opcode::UnsupportedIndirectCall,
            mircap_capnp::Opcode::AddI64 => Opcode::AddI64,
            mircap_capnp::Opcode::SubI64 => Opcode::SubI64,
            mircap_capnp::Opcode::MulI64 => Opcode::MulI64,
            mircap_capnp::Opcode::EqI64 => Opcode::EqI64,
            mircap_capnp::Opcode::NeI64 => Opcode::NeI64,
            mircap_capnp::Opcode::LtI64 => Opcode::LtI64,
            mircap_capnp::Opcode::LoadI64 => Opcode::LoadI64,
            mircap_capnp::Opcode::StoreI64 => Opcode::StoreI64,
            mircap_capnp::Opcode::ConstF32 => Opcode::ConstF32,
            mircap_capnp::Opcode::ConstF64 => Opcode::ConstF64,
            mircap_capnp::Opcode::AddF32 => Opcode::AddF32,
            mircap_capnp::Opcode::SubF32 => Opcode::SubF32,
            mircap_capnp::Opcode::MulF32 => Opcode::MulF32,
            mircap_capnp::Opcode::DivF32 => Opcode::DivF32,
            mircap_capnp::Opcode::NegF32 => Opcode::NegF32,
            mircap_capnp::Opcode::EqF32 => Opcode::EqF32,
            mircap_capnp::Opcode::NeF32 => Opcode::NeF32,
            mircap_capnp::Opcode::LtF32 => Opcode::LtF32,
            mircap_capnp::Opcode::LeF32 => Opcode::LeF32,
            mircap_capnp::Opcode::GtF32 => Opcode::GtF32,
            mircap_capnp::Opcode::GeF32 => Opcode::GeF32,
            mircap_capnp::Opcode::AddF64 => Opcode::AddF64,
            mircap_capnp::Opcode::SubF64 => Opcode::SubF64,
            mircap_capnp::Opcode::MulF64 => Opcode::MulF64,
            mircap_capnp::Opcode::DivF64 => Opcode::DivF64,
            mircap_capnp::Opcode::NegF64 => Opcode::NegF64,
            mircap_capnp::Opcode::EqF64 => Opcode::EqF64,
            mircap_capnp::Opcode::NeF64 => Opcode::NeF64,
            mircap_capnp::Opcode::LtF64 => Opcode::LtF64,
            mircap_capnp::Opcode::LeF64 => Opcode::LeF64,
            mircap_capnp::Opcode::GtF64 => Opcode::GtF64,
            mircap_capnp::Opcode::GeF64 => Opcode::GeF64,
            mircap_capnp::Opcode::I32ToF32 => Opcode::I32ToF32,
            mircap_capnp::Opcode::F32ToI32 => Opcode::F32ToI32,
            mircap_capnp::Opcode::I32ToF64 => Opcode::I32ToF64,
            mircap_capnp::Opcode::F64ToI32 => Opcode::F64ToI32,
            mircap_capnp::Opcode::F32ToF64 => Opcode::F32ToF64,
            mircap_capnp::Opcode::F64ToF32 => Opcode::F64ToF32,
        };

        let first_res = insn.get_first_result() as usize;
        let res_count = insn.get_result_count() as usize;
        let mut results = Vec::new();
        if first_res + res_count <= results_list.len() {
            results.extend_from_slice(&results_list[first_res..first_res + res_count]);
        }

        let first_op = insn.get_first_operand() as usize;
        let op_count = insn.get_operand_count() as usize;
        let mut operands = Vec::new();
        if first_op + op_count <= operands_list.len() {
            operands.extend_from_slice(&operands_list[first_op..first_op + op_count]);
        }

        let span_val = insn.get_source_span();
        let source_span = if span_val == 0 {
            None
        } else {
            Some(SourceSpanId(span_val))
        };

        instructions.push(Instruction {
            id,
            opcode,
            results,
            operands,
            source_span,
        });
    }

    // Read flat blocks list
    let mut blocks = Vec::new();
    for i in 0..capnp_blocks.len() {
        let block = capnp_blocks.get(i);
        let id = BlockId(block.get_id());
        let parent = FunctionId(block.get_parent_function());

        let first_insn = block.get_first_instruction() as usize;
        let insn_count = block.get_instruction_count() as usize;
        let mut block_insns = Vec::new();
        for idx in first_insn..first_insn + insn_count {
            if idx < instructions.len() {
                block_insns.push(instructions[idx].id);
            }
        }

        let terminator = InstructionId(block.get_terminator());
        let span_val = block.get_source_span();
        let source_span = if span_val == 0 {
            None
        } else {
            Some(SourceSpanId(span_val))
        };

        blocks.push(Block {
            id,
            parent,
            instructions: block_insns,
            terminator,
            source_span,
        });
    }

    // Read functions
    let capnp_functions = root.get_functions()?;
    let mut functions = Vec::new();
    for i in 0..capnp_functions.len() {
        let func = capnp_functions.get(i);
        let id = FunctionId(func.get_id());
        let symbol = SymbolId(func.get_symbol());

        let capnp_params = func.get_params()?;
        let mut params = Vec::new();
        for j in 0..capnp_params.len() {
            params.push(TypeId(capnp_params.get(j)));
        }

        let capnp_results_list = func.get_results()?;
        let mut results = Vec::new();
        for j in 0..capnp_results_list.len() {
            results.push(TypeId(capnp_results_list.get(j)));
        }

        let value_count = func.get_value_count();

        let capnp_val_types = func.get_value_types()?;
        let mut value_types = Vec::new();
        for j in 0..capnp_val_types.len() {
            value_types.push(TypeId(capnp_val_types.get(j)));
        }

        let first_block = func.get_first_block() as usize;
        let block_count = func.get_block_count() as usize;
        let mut func_blocks = Vec::new();
        for idx in first_block..first_block + block_count {
            if idx < blocks.len() {
                func_blocks.push(blocks[idx].id);
            }
        }

        let flags = func.get_flags();
        let span_val = func.get_source_span();
        let source_span = if span_val == 0 {
            None
        } else {
            Some(SourceSpanId(span_val))
        };

        functions.push(Function {
            id,
            symbol,
            params,
            results,
            value_count,
            value_types,
            blocks: func_blocks,
            flags,
            source_span,
        });
    }

    Ok(ModuleImage {
        header,
        module,
        types,
        symbols,
        functions,
        data_segments,
        blocks,
        instructions,
        source_spans: Vec::new(),
        metadata: Vec::new(),
    })
}
