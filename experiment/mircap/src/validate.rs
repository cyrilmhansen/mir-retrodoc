use crate::error::{EntityRef, ErrorKind, ValidationError};
use crate::ids::{BlockId, FunctionId, TypeId, ValueId};
use crate::image::{
    ModuleImage, Opcode, Operand, SymbolKind, TypeKind, FORMAT_SCHEMA_NAME, FORMAT_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationReport {
    pub function_count: usize,
    pub block_count: usize,
    pub instruction_count: usize,
}

pub trait Validate {
    fn validate(&self) -> Result<ValidationReport, Vec<ValidationError>>;
}

impl Validate for ModuleImage {
    fn validate(&self) -> Result<ValidationReport, Vec<ValidationError>> {
        let mut v = Validator {
            image: self,
            errors: Vec::new(),
        };
        v.run();
        if v.errors.is_empty() {
            Ok(ValidationReport {
                function_count: self.functions.len(),
                block_count: self.blocks.len(),
                instruction_count: self.instructions.len(),
            })
        } else {
            Err(v.errors)
        }
    }
}

struct Validator<'a> {
    image: &'a ModuleImage,
    errors: Vec<ValidationError>,
}

impl Validator<'_> {
    fn run(&mut self) {
        if self.image.header.schema_name != FORMAT_SCHEMA_NAME {
            self.error(
                ErrorKind::InvalidFormat,
                EntityRef::Module,
                "unsupported schema name",
            );
        }
        if self.image.header.format_version != FORMAT_VERSION {
            self.error(
                ErrorKind::UnsupportedVersion,
                EntityRef::Module,
                "unsupported format version",
            );
        }

        self.check_unique_ids();
        self.check_unsupported_types();
        self.check_references_and_shapes();
    }

    fn check_unique_ids(&mut self) {
        self.unique(
            self.image.types.iter().map(|t| t.id.0),
            EntityRef::Module,
            "duplicate type ID",
        );
        self.unique(
            self.image.symbols.iter().map(|s| s.id.0),
            EntityRef::Module,
            "duplicate symbol ID",
        );
        self.unique(
            self.image.functions.iter().map(|f| f.id.0),
            EntityRef::Module,
            "duplicate function ID",
        );
        self.unique(
            self.image.blocks.iter().map(|b| b.id.0),
            EntityRef::Module,
            "duplicate block ID",
        );
        self.unique(
            self.image.instructions.iter().map(|i| i.id.0),
            EntityRef::Module,
            "duplicate instruction ID",
        );
        self.unique(
            self.image.data_segments.iter().map(|d| d.symbol.0),
            EntityRef::Module,
            "duplicate data segment symbol ID",
        );
    }

    fn unique<I>(&mut self, ids: I, entity: EntityRef, message: &str)
    where
        I: IntoIterator<Item = u32>,
    {
        let mut seen = BTreeSet::new();
        for id in ids {
            if !seen.insert(id) {
                self.error(
                    ErrorKind::DuplicateId,
                    entity.clone(),
                    format!("{message}: {id}"),
                );
            }
        }
    }

    fn check_unsupported_types(&mut self) {
        for ty in &self.image.types {
            match ty.kind {
                TypeKind::UnsupportedI64
                | TypeKind::UnsupportedFloat
                | TypeKind::UnsupportedLongDouble
                | TypeKind::UnsupportedAggregate
                | TypeKind::UnsupportedVarargs
                | TypeKind::UnsupportedHostCAbi => {
                    self.error(
                        ErrorKind::UnsupportedFeature,
                        EntityRef::Type(ty.id),
                        format!("unsupported MIR-F0 type: {:?}", ty.kind),
                    );
                }
                TypeKind::Void | TypeKind::I32 | TypeKind::U32 | TypeKind::Addr32 => {}
            }
        }
    }

    fn check_references_and_shapes(&mut self) {
        let types: BTreeSet<TypeId> = self.image.types.iter().map(|t| t.id).collect();
        let symbols: BTreeMap<_, _> = self.image.symbols.iter().map(|s| (s.id, s)).collect();
        let functions: BTreeMap<_, _> = self.image.functions.iter().map(|f| (f.id, f)).collect();
        let blocks: BTreeMap<_, _> = self.image.blocks.iter().map(|b| (b.id, b)).collect();
        let insns: BTreeMap<_, _> = self.image.instructions.iter().map(|i| (i.id, i)).collect();

        for function in &self.image.functions {
            match symbols.get(&function.symbol) {
                Some(symbol) if symbol.kind == SymbolKind::Function => {}
                Some(_) => self.error(
                    ErrorKind::MalformedFunctionSignature,
                    EntityRef::Function(function.id),
                    "function symbol is not a function symbol",
                ),
                None => self.error(
                    ErrorKind::MissingReference,
                    EntityRef::Function(function.id),
                    format!("missing function symbol {}", function.symbol),
                ),
            }
            for ty in function.params.iter().chain(function.results.iter()) {
                if !types.contains(ty) {
                    self.error(
                        ErrorKind::MissingReference,
                        EntityRef::Function(function.id),
                        format!("missing type {ty} in signature"),
                    );
                }
            }
            if function.value_types.len() != function.value_count as usize {
                self.error(
                    ErrorKind::MalformedFunctionSignature,
                    EntityRef::Function(function.id),
                    "value_types length must match value_count",
                );
            }
            for (idx, ty) in function.value_types.iter().enumerate() {
                if !types.contains(ty) {
                    self.error(
                        ErrorKind::MissingReference,
                        EntityRef::Function(function.id),
                        format!("missing type {ty} for value {idx}"),
                    );
                }
            }
            if function.params.len() > function.value_types.len() {
                self.error(
                    ErrorKind::MalformedFunctionSignature,
                    EntityRef::Function(function.id),
                    "parameter count exceeds value_count",
                );
            } else {
                for (idx, param_ty) in function.params.iter().enumerate() {
                    if function.value_types.get(idx) != Some(param_ty) {
                        self.error(
                            ErrorKind::MalformedFunctionSignature,
                            EntityRef::Function(function.id),
                            format!("parameter {idx} type does not match value table"),
                        );
                    }
                }
            }
            for block_id in &function.blocks {
                match blocks.get(block_id) {
                    Some(block) if block.parent == function.id => {}
                    Some(_) => self.error(
                        ErrorKind::WrongParent,
                        EntityRef::Block(*block_id),
                        "block parent does not match function",
                    ),
                    None => self.error(
                        ErrorKind::MissingReference,
                        EntityRef::Function(function.id),
                        format!("missing block {block_id}"),
                    ),
                }
            }
        }

        for data in &self.image.data_segments {
            match symbols.get(&data.symbol) {
                Some(symbol) if symbol.kind == SymbolKind::Data => {}
                Some(_) => self.error(
                    ErrorKind::MalformedOperand,
                    EntityRef::Symbol(data.symbol),
                    "data segment symbol is not a data symbol",
                ),
                None => self.error(
                    ErrorKind::MissingReference,
                    EntityRef::Symbol(data.symbol),
                    format!("missing data symbol {}", data.symbol),
                ),
            }
            let Some(bytes_end) = data.offset.checked_add(data.bytes.len() as u32) else {
                self.error(
                    ErrorKind::MalformedOperand,
                    EntityRef::Symbol(data.symbol),
                    "data segment byte range overflows u32",
                );
                continue;
            };
            if bytes_end.checked_add(data.zero_fill).is_none() {
                self.error(
                    ErrorKind::MalformedOperand,
                    EntityRef::Symbol(data.symbol),
                    "data segment zero-fill range overflows u32",
                );
            }
        }

        for block in &self.image.blocks {
            if !functions.contains_key(&block.parent) {
                self.error(
                    ErrorKind::MissingReference,
                    EntityRef::Block(block.id),
                    format!("missing parent function {}", block.parent),
                );
            }
            if block.instructions.is_empty() {
                self.error(
                    ErrorKind::InvalidTerminator,
                    EntityRef::Block(block.id),
                    "block has no terminator",
                );
                continue;
            }
            for (idx, insn_id) in block.instructions.iter().enumerate() {
                let Some(insn) = insns.get(insn_id) else {
                    self.error(
                        ErrorKind::MissingReference,
                        EntityRef::Block(block.id),
                        format!("missing instruction {insn_id}"),
                    );
                    continue;
                };
                if idx + 1 < block.instructions.len() && insn.opcode.is_terminator() {
                    self.error(
                        ErrorKind::InvalidTerminator,
                        EntityRef::Instruction(*insn_id),
                        "instruction appears after block terminator",
                    );
                }
            }
            match insns.get(&block.terminator) {
                Some(insn) if insn.opcode.is_terminator() => {}
                Some(_) => self.error(
                    ErrorKind::InvalidTerminator,
                    EntityRef::Block(block.id),
                    "block terminator reference is not a terminator opcode",
                ),
                None => self.error(
                    ErrorKind::MissingReference,
                    EntityRef::Block(block.id),
                    format!("missing terminator {}", block.terminator),
                ),
            }
            if block.instructions.last().copied() != Some(block.terminator) {
                self.error(
                    ErrorKind::InvalidTerminator,
                    EntityRef::Block(block.id),
                    "terminator must be the final instruction in the block",
                );
            }
        }

        for function in &self.image.functions {
            for block_id in &function.blocks {
                let Some(block) = blocks.get(block_id) else {
                    continue;
                };
                for insn_id in &block.instructions {
                    if let Some(insn) = insns.get(insn_id) {
                        self.check_instruction(function.id, insn, &functions, &blocks);
                    }
                }
            }
        }
    }

    fn check_instruction(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
        blocks: &BTreeMap<BlockId, &crate::image::Block>,
    ) {
        if let Some(function) = functions.get(&current_function) {
            for result in &insn.results {
                if result.0 >= function.value_count {
                    self.error(
                        ErrorKind::MissingReference,
                        EntityRef::Instruction(insn.id),
                        format!(
                            "result value {result} exceeds function value_count {}",
                            function.value_count
                        ),
                    );
                }
            }
            for operand in &insn.operands {
                if let Operand::Value(value) = operand {
                    if value.0 >= function.value_count {
                        self.error(
                            ErrorKind::MissingReference,
                            EntityRef::Instruction(insn.id),
                            format!(
                                "operand value {value} exceeds function value_count {}",
                                function.value_count
                            ),
                        );
                    }
                }
            }
        }

        if matches!(
            insn.opcode,
            Opcode::UnsupportedI64 | Opcode::UnsupportedIndirectCall
        ) {
            self.error(
                ErrorKind::UnsupportedFeature,
                EntityRef::Instruction(insn.id),
                format!("unsupported MIR-F0 opcode: {:?}", insn.opcode),
            );
        }

        match insn.opcode {
            Opcode::ConstI32 | Opcode::ConstU32 => {
                self.expect_results(insn, 1);
                self.expect_operands(insn, 1);
                let expected = if matches!(insn.opcode, Opcode::ConstI32) {
                    TypeKind::I32
                } else {
                    TypeKind::U32
                };
                self.expect_result_type(current_function, insn, functions, 0, expected);
                let ok = match insn.opcode {
                    Opcode::ConstI32 => matches!(insn.operands.first(), Some(Operand::ImmI32(_))),
                    Opcode::ConstU32 => matches!(insn.operands.first(), Some(Operand::ImmU32(_))),
                    _ => false,
                };
                if !ok {
                    self.error(
                        ErrorKind::MalformedOperand,
                        EntityRef::Instruction(insn.id),
                        "constant immediate kind does not match opcode",
                    );
                }
            }
            Opcode::Copy => {
                self.expect_results(insn, 1);
                self.expect_operands(insn, 1);
                self.expect_value_operand(insn, 0);
                if let (Some(result), Some(Operand::Value(src))) =
                    (insn.results.first(), insn.operands.first())
                {
                    let result_ty = self.value_type(current_function, *result, functions);
                    let src_ty = self.value_type(current_function, *src, functions);
                    if result_ty.is_some() && src_ty.is_some() && result_ty != src_ty {
                        self.error(
                            ErrorKind::TypeMismatch,
                            EntityRef::Instruction(insn.id),
                            "copy source/result type mismatch",
                        );
                    }
                }
            }
            Opcode::AddI32
            | Opcode::SubI32
            | Opcode::MulI32
            | Opcode::EqI32
            | Opcode::NeI32
            | Opcode::LtI32 => {
                self.expect_results(insn, 1);
                self.expect_operands(insn, 2);
                self.expect_value_operand(insn, 0);
                self.expect_value_operand(insn, 1);
                let result_type = if matches!(
                    insn.opcode,
                    Opcode::AddI32 | Opcode::SubI32 | Opcode::MulI32
                ) {
                    TypeKind::I32
                } else {
                    TypeKind::U32
                };
                self.expect_result_type(current_function, insn, functions, 0, result_type);
                self.expect_operand_type(current_function, insn, functions, 0, TypeKind::I32);
                self.expect_operand_type(current_function, insn, functions, 1, TypeKind::I32);
            }
            Opcode::AddU32
            | Opcode::SubU32
            | Opcode::MulU32
            | Opcode::EqU32
            | Opcode::NeU32
            | Opcode::LtU32
            | Opcode::LeU32
            | Opcode::GtU32
            | Opcode::GeU32 => {
                self.expect_results(insn, 1);
                self.expect_operands(insn, 2);
                self.expect_value_operand(insn, 0);
                self.expect_value_operand(insn, 1);
                self.expect_result_type(current_function, insn, functions, 0, TypeKind::U32);
                self.expect_operand_type(current_function, insn, functions, 0, TypeKind::U32);
                self.expect_operand_type(current_function, insn, functions, 1, TypeKind::U32);
            }
            Opcode::Branch => {
                self.expect_results(insn, 0);
                self.expect_operands(insn, 1);
                self.expect_same_function_block(insn, 0, current_function, blocks);
            }
            Opcode::BranchIf => {
                self.expect_results(insn, 0);
                self.expect_operands(insn, 3);
                self.expect_value_operand(insn, 0);
                self.expect_operand_type(current_function, insn, functions, 0, TypeKind::U32);
                self.expect_same_function_block(insn, 1, current_function, blocks);
                self.expect_same_function_block(insn, 2, current_function, blocks);
            }
            Opcode::Call => self.check_call(current_function, insn, functions),
            Opcode::Ret => self.check_return(current_function, insn, functions),
            Opcode::Trap => {
                self.expect_results(insn, 0);
            }
            Opcode::Alloc => self.check_alloc(current_function, insn, functions),
            Opcode::LoadI32 => self.check_load(current_function, insn, functions, TypeKind::I32),
            Opcode::LoadU32 => self.check_load(current_function, insn, functions, TypeKind::U32),
            Opcode::StoreI32 => self.check_store(current_function, insn, functions, TypeKind::I32),
            Opcode::StoreU32 => self.check_store(current_function, insn, functions, TypeKind::U32),
            Opcode::LoadU8 => self.check_load(current_function, insn, functions, TypeKind::U32),
            Opcode::StoreU8 => self.check_store(current_function, insn, functions, TypeKind::U32),
            Opcode::AddrAdd => self.check_addr_add(current_function, insn, functions),
            Opcode::DataAddr => self.check_data_addr(current_function, insn, functions),
            Opcode::UnsupportedI64 | Opcode::UnsupportedIndirectCall => {}
        }
    }

    fn check_alloc(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
    ) {
        self.expect_results(insn, 1);
        self.expect_operands(insn, 2);
        self.expect_result_type(current_function, insn, functions, 0, TypeKind::Addr32);
        self.expect_integer_or_uimm_operand(current_function, insn, functions, 0);
        self.expect_integer_or_uimm_operand(current_function, insn, functions, 1);
    }

    fn check_load(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
        result_type: TypeKind,
    ) {
        self.expect_results(insn, 1);
        self.expect_operands(insn, 1);
        self.expect_result_type(current_function, insn, functions, 0, result_type);
        self.expect_operand_type(current_function, insn, functions, 0, TypeKind::Addr32);
    }

    fn check_store(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
        value_type: TypeKind,
    ) {
        self.expect_results(insn, 0);
        self.expect_operands(insn, 2);
        self.expect_operand_type(current_function, insn, functions, 0, TypeKind::Addr32);
        self.expect_operand_type(current_function, insn, functions, 1, value_type);
    }

    fn check_addr_add(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
    ) {
        self.expect_results(insn, 1);
        self.expect_operands(insn, 2);
        self.expect_result_type(current_function, insn, functions, 0, TypeKind::Addr32);
        self.expect_operand_type(current_function, insn, functions, 0, TypeKind::Addr32);
        self.expect_operand_type(current_function, insn, functions, 1, TypeKind::U32);
    }

    fn check_data_addr(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
    ) {
        self.expect_results(insn, 1);
        self.expect_operands(insn, 2);
        self.expect_result_type(current_function, insn, functions, 0, TypeKind::Addr32);

        let mut valid_symbol = false;
        let mut segment_len = 0u32;

        match insn.operands.first() {
            Some(Operand::Symbol(sym_id)) => match self.image.symbol(*sym_id) {
                Some(sym) => {
                    if sym.kind != SymbolKind::Data {
                        self.error(
                            ErrorKind::TypeMismatch,
                            EntityRef::Instruction(insn.id),
                            "data_addr first operand must reference a Data symbol",
                        );
                    } else {
                        valid_symbol = true;
                        if let Some(ds) = self
                            .image
                            .data_segments
                            .iter()
                            .find(|ds| ds.symbol == *sym_id)
                        {
                            segment_len = ds.bytes.len() as u32 + ds.zero_fill;
                        } else {
                            self.error(ErrorKind::MissingReference, EntityRef::Instruction(insn.id), format!("data_addr references symbol {} but no corresponding data segment exists", sym.name));
                            valid_symbol = false;
                        }
                    }
                }
                None => {
                    self.error(
                        ErrorKind::MissingReference,
                        EntityRef::Instruction(insn.id),
                        format!("data_addr references missing symbol {sym_id}"),
                    );
                }
            },
            _ => self.error(
                ErrorKind::MalformedOperand,
                EntityRef::Instruction(insn.id),
                "data_addr first operand must be a symbol reference",
            ),
        }

        match insn.operands.get(1) {
            Some(Operand::ImmU32(val)) => {
                if valid_symbol && *val > segment_len {
                    self.error(
                        ErrorKind::MalformedOperand,
                        EntityRef::Instruction(insn.id),
                        format!(
                            "static offset {} is out of range for data segment (max {})",
                            val, segment_len
                        ),
                    );
                }
            }
            Some(Operand::Value(val_id)) => {
                let Some(actual) = self.value_type(current_function, *val_id, functions) else {
                    return;
                };
                if actual != TypeKind::U32 {
                    self.error(
                        ErrorKind::TypeMismatch,
                        EntityRef::Instruction(insn.id),
                        format!("data_addr offset operand must be u32, got {actual:?}"),
                    );
                }
            }
            _ => self.error(
                ErrorKind::MalformedOperand,
                EntityRef::Instruction(insn.id),
                "data_addr second operand must be a u32 immediate or value reference",
            ),
        }
    }

    fn check_call(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
    ) {
        let Some(Operand::Function(callee_id)) = insn.operands.first() else {
            self.error(
                ErrorKind::MalformedOperand,
                EntityRef::Instruction(insn.id),
                "call expects callee function as first operand",
            );
            return;
        };
        let Some(callee) = functions.get(callee_id) else {
            self.error(
                ErrorKind::MissingReference,
                EntityRef::Instruction(insn.id),
                format!("call references missing function {callee_id}"),
            );
            return;
        };
        let arg_count = insn.operands.len().saturating_sub(1);
        if arg_count != callee.params.len() {
            self.error(
                ErrorKind::MalformedFunctionSignature,
                EntityRef::Instruction(insn.id),
                format!(
                    "call argument count mismatch: expected {}, got {arg_count}",
                    callee.params.len()
                ),
            );
        }
        if insn.results.len() != callee.results.len() {
            self.error(
                ErrorKind::MalformedFunctionSignature,
                EntityRef::Instruction(insn.id),
                format!(
                    "call result count mismatch: expected {}, got {}",
                    callee.results.len(),
                    insn.results.len()
                ),
            );
        }
        for (idx, expected_ty) in callee.results.iter().enumerate() {
            let Some(result) = insn.results.get(idx).copied() else {
                continue;
            };
            let Some(actual) = self.value_type(current_function, result, functions) else {
                continue;
            };
            let expected = self.image.type_kind(*expected_ty);
            if expected.is_some() && expected != Some(actual) {
                self.error(
                    ErrorKind::TypeMismatch,
                    EntityRef::Instruction(insn.id),
                    format!("call result {idx} type mismatch"),
                );
            }
        }
        for idx in 1..insn.operands.len() {
            self.expect_value_operand(insn, idx);
            let Some(Operand::Value(arg)) = insn.operands.get(idx) else {
                continue;
            };
            let Some(actual) = self.value_type(current_function, *arg, functions) else {
                continue;
            };
            let expected = callee
                .params
                .get(idx - 1)
                .and_then(|ty| self.image.type_kind(*ty));
            if expected.is_some() && expected != Some(actual) {
                self.error(
                    ErrorKind::TypeMismatch,
                    EntityRef::Instruction(insn.id),
                    format!("call argument {} type mismatch", idx - 1),
                );
            }
        }
    }

    fn check_return(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
    ) {
        self.expect_results(insn, 0);
        let expected = functions
            .get(&current_function)
            .map(|f| f.results.len())
            .unwrap_or(0);
        if insn.operands.len() != expected {
            self.error(
                ErrorKind::TypeMismatch,
                EntityRef::Instruction(insn.id),
                format!(
                    "return value count mismatch: expected {expected}, got {}",
                    insn.operands.len()
                ),
            );
        }
        for idx in 0..insn.operands.len() {
            self.expect_value_operand(insn, idx);
            let Some(Operand::Value(value)) = insn.operands.get(idx) else {
                continue;
            };
            let Some(actual) = self.value_type(current_function, *value, functions) else {
                continue;
            };
            let expected = functions
                .get(&current_function)
                .and_then(|function| function.results.get(idx))
                .and_then(|ty| self.image.type_kind(*ty));
            if expected.is_some() && expected != Some(actual) {
                self.error(
                    ErrorKind::TypeMismatch,
                    EntityRef::Instruction(insn.id),
                    format!("return value {idx} type mismatch"),
                );
            }
        }
    }

    fn expect_results(&mut self, insn: &crate::image::Instruction, expected: usize) {
        if insn.results.len() != expected {
            self.error(
                ErrorKind::MalformedOperand,
                EntityRef::Instruction(insn.id),
                format!("expected {expected} results, got {}", insn.results.len()),
            );
        }
    }

    fn expect_operands(&mut self, insn: &crate::image::Instruction, expected: usize) {
        if insn.operands.len() != expected {
            self.error(
                ErrorKind::MalformedOperand,
                EntityRef::Instruction(insn.id),
                format!("expected {expected} operands, got {}", insn.operands.len()),
            );
        }
    }

    fn expect_value_operand(&mut self, insn: &crate::image::Instruction, idx: usize) {
        if !matches!(insn.operands.get(idx), Some(Operand::Value(ValueId(_)))) {
            self.error(
                ErrorKind::MalformedOperand,
                EntityRef::Instruction(insn.id),
                format!("operand {idx} must be a value reference"),
            );
        }
    }

    fn expect_result_type(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
        idx: usize,
        expected: TypeKind,
    ) {
        let Some(value) = insn.results.get(idx).copied() else {
            return;
        };
        let Some(actual) = self.value_type(current_function, value, functions) else {
            return;
        };
        if actual != expected {
            self.error(
                ErrorKind::TypeMismatch,
                EntityRef::Instruction(insn.id),
                format!("result {idx} type mismatch: expected {expected:?}, got {actual:?}"),
            );
        }
    }

    fn expect_operand_type(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
        idx: usize,
        expected: TypeKind,
    ) {
        match insn.operands.get(idx) {
            Some(Operand::Value(value)) => {
                let Some(actual) = self.value_type(current_function, *value, functions) else {
                    return;
                };
                if actual != expected {
                    self.error(
                        ErrorKind::TypeMismatch,
                        EntityRef::Instruction(insn.id),
                        format!(
                            "operand {idx} type mismatch: expected {expected:?}, got {actual:?}"
                        ),
                    );
                }
            }
            _ => self.error(
                ErrorKind::MalformedOperand,
                EntityRef::Instruction(insn.id),
                format!("operand {idx} must be a value reference"),
            ),
        }
    }

    fn expect_integer_or_uimm_operand(
        &mut self,
        current_function: FunctionId,
        insn: &crate::image::Instruction,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
        idx: usize,
    ) {
        match insn.operands.get(idx) {
            Some(Operand::ImmU32(_)) => {}
            Some(Operand::Value(value)) => {
                let Some(actual) = self.value_type(current_function, *value, functions) else {
                    return;
                };
                if !matches!(actual, TypeKind::I32 | TypeKind::U32) {
                    self.error(
                        ErrorKind::TypeMismatch,
                        EntityRef::Instruction(insn.id),
                        format!("operand {idx} must be i32/u32 for alloc, got {actual:?}"),
                    );
                }
            }
            _ => self.error(
                ErrorKind::MalformedOperand,
                EntityRef::Instruction(insn.id),
                format!("operand {idx} must be u32 immediate or integer value"),
            ),
        }
    }

    fn value_type(
        &mut self,
        current_function: FunctionId,
        value: ValueId,
        functions: &BTreeMap<FunctionId, &crate::image::Function>,
    ) -> Option<TypeKind> {
        let function = functions.get(&current_function)?;
        let type_id = *function.value_types.get(value.0 as usize)?;
        match self.image.type_kind(type_id) {
            Some(kind) => Some(kind),
            None => {
                self.error(
                    ErrorKind::MissingReference,
                    EntityRef::Function(current_function),
                    format!("missing type {type_id} for value {value}"),
                );
                None
            }
        }
    }

    fn expect_same_function_block(
        &mut self,
        insn: &crate::image::Instruction,
        idx: usize,
        current_function: FunctionId,
        blocks: &BTreeMap<BlockId, &crate::image::Block>,
    ) {
        let Some(Operand::Block(block_id)) = insn.operands.get(idx) else {
            self.error(
                ErrorKind::MalformedOperand,
                EntityRef::Instruction(insn.id),
                format!("operand {idx} must be a block reference"),
            );
            return;
        };
        match blocks.get(block_id) {
            Some(block) if block.parent == current_function => {}
            Some(_) => self.error(
                ErrorKind::WrongParent,
                EntityRef::Instruction(insn.id),
                "branch target belongs to a different function",
            ),
            None => self.error(
                ErrorKind::MissingReference,
                EntityRef::Instruction(insn.id),
                format!("missing branch target block {block_id}"),
            ),
        }
    }

    fn error(&mut self, kind: ErrorKind, entity: EntityRef, message: impl Into<String>) {
        self.errors
            .push(ValidationError::new(kind, entity, message));
    }
}
