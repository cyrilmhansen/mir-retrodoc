use crate::ids::{BlockId, FunctionId, InstructionId, SymbolId, TypeId, ValueId};
use crate::image::{
    Block, DataSegment, Function, Header, Instruction, Module, ModuleImage, Opcode, Operand,
    Symbol, SymbolKind, TypeDef, TypeKind, FORMAT_SCHEMA_NAME, FORMAT_VERSION,
};
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum LoadError {
    InvalidUtf8,
    InvalidLine { line: usize, message: String },
}

pub fn from_bytes(bytes: &[u8]) -> Result<ModuleImage, LoadError> {
    let text = std::str::from_utf8(bytes).map_err(|_| LoadError::InvalidUtf8)?;
    from_text(text)
}

pub fn from_text(text: &str) -> Result<ModuleImage, LoadError> {
    let mut header = Header {
        schema_name: FORMAT_SCHEMA_NAME.to_string(),
        format_version: FORMAT_VERSION,
        producer_name: "mircap-text-fixture".to_string(),
        producer_version: "0".to_string(),
        source_language: None,
        target_assumptions: None,
        feature_flags: Vec::new(),
    };
    let mut module = Module {
        id: 0,
        name: String::from("unnamed"),
    };
    let mut types = Vec::new();
    let mut symbols = Vec::new();
    let mut functions = Vec::new();
    let mut data_segments = Vec::new();
    let mut blocks = Vec::new();
    let mut instructions = Vec::new();
    let mut pending_blocks: BTreeMap<FunctionId, Vec<BlockId>> = BTreeMap::new();

    for (line_no, raw) in text.lines().enumerate() {
        let line_no = line_no + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.first().copied() {
            Some("mircap") => {
                expect_len(&parts, 2, line_no)?;
                header.schema_name = parts[1].to_string();
            }
            Some("version") => {
                expect_len(&parts, 2, line_no)?;
                header.format_version = parse_u32(parts[1], line_no)?;
            }
            Some("module") => {
                expect_len(&parts, 3, line_no)?;
                module = Module {
                    id: parse_u32(parts[1], line_no)?,
                    name: parts[2].to_string(),
                };
            }
            Some("type") => {
                expect_len(&parts, 3, line_no)?;
                types.push(TypeDef {
                    id: TypeId(parse_u32(parts[1], line_no)?),
                    kind: parse_type(parts[2], line_no)?,
                });
            }
            Some("symbol") => {
                expect_len(&parts, 4, line_no)?;
                symbols.push(Symbol {
                    id: SymbolId(parse_u32(parts[1], line_no)?),
                    name: parts[2].to_string(),
                    kind: parse_symbol_kind(parts[3], line_no)?,
                });
            }
            Some("function") => {
                expect_min_len(&parts, 7, line_no)?;
                let id = FunctionId(parse_u32(parts[1], line_no)?);
                let symbol = SymbolId(parse_u32(parts[2], line_no)?);
                let params = parse_type_list(parts[3], line_no)?;
                let results = parse_type_list(parts[4], line_no)?;
                let value_count = parse_u32(parts[5], line_no)?;
                let flags = parse_u32(parts[6], line_no)?;
                let value_types = if parts.len() >= 8 {
                    parse_type_list(parts[7], line_no)?
                } else {
                    Vec::new()
                };
                functions.push(Function {
                    id,
                    symbol,
                    params,
                    results,
                    value_count,
                    value_types,
                    blocks: Vec::new(),
                    flags,
                    source_span: None,
                });
            }
            Some("data") => {
                expect_len(&parts, 5, line_no)?;
                data_segments.push(DataSegment {
                    symbol: SymbolId(parse_u32(parts[1], line_no)?),
                    offset: parse_u32(parts[2], line_no)?,
                    bytes: parse_hex_bytes(parts[3], line_no)?,
                    zero_fill: parse_u32(parts[4], line_no)?,
                });
            }
            Some("func_block") => {
                expect_len(&parts, 3, line_no)?;
                pending_blocks
                    .entry(FunctionId(parse_u32(parts[1], line_no)?))
                    .or_default()
                    .push(BlockId(parse_u32(parts[2], line_no)?));
            }
            Some("block") => {
                expect_min_len(&parts, 4, line_no)?;
                let id = BlockId(parse_u32(parts[1], line_no)?);
                let parent = FunctionId(parse_u32(parts[2], line_no)?);
                let mut insns = Vec::new();
                for token in &parts[3..] {
                    insns.push(InstructionId(parse_u32(token, line_no)?));
                }
                let terminator = *insns
                    .last()
                    .ok_or_else(|| err(line_no, "block needs at least one instruction"))?;
                blocks.push(Block {
                    id,
                    parent,
                    instructions: insns,
                    terminator,
                    source_span: None,
                });
            }
            Some("insn") => {
                expect_min_len(&parts, 3, line_no)?;
                let id = InstructionId(parse_u32(parts[1], line_no)?);
                let opcode = parse_opcode(parts[2], line_no)?;
                let mut results = Vec::new();
                let mut operands = Vec::new();
                for token in &parts[3..] {
                    if let Some(rest) = token.strip_prefix("r:") {
                        results.push(ValueId(parse_u32(rest, line_no)?));
                    } else {
                        operands.push(parse_operand(token, line_no)?);
                    }
                }
                instructions.push(Instruction {
                    id,
                    opcode,
                    results,
                    operands,
                    source_span: None,
                });
            }
            Some(_) | None => return Err(err(line_no, "unknown directive")),
        }
    }

    for function in &mut functions {
        if let Some(blocks) = pending_blocks.remove(&function.id) {
            function.blocks = blocks;
        }
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

fn expect_len(parts: &[&str], expected: usize, line: usize) -> Result<(), LoadError> {
    if parts.len() == expected {
        Ok(())
    } else {
        Err(err(line, format!("expected {expected} fields")))
    }
}

fn expect_min_len(parts: &[&str], expected: usize, line: usize) -> Result<(), LoadError> {
    if parts.len() >= expected {
        Ok(())
    } else {
        Err(err(line, format!("expected at least {expected} fields")))
    }
}

fn parse_u32(s: &str, line: usize) -> Result<u32, LoadError> {
    s.parse()
        .map_err(|_| err(line, format!("invalid u32: {s}")))
}

fn parse_i32(s: &str, line: usize) -> Result<i32, LoadError> {
    s.parse()
        .map_err(|_| err(line, format!("invalid i32: {s}")))
}

fn parse_i64(s: &str, line: usize) -> Result<i64, LoadError> {
    s.parse()
        .map_err(|_| err(line, format!("invalid i64: {s}")))
}

fn parse_f32_bits(s: &str, line: usize) -> Result<u32, LoadError> {
    s.parse::<f32>()
        .map(f32::to_bits)
        .map_err(|_| err(line, format!("invalid f32: {s}")))
}

fn parse_f64_bits(s: &str, line: usize) -> Result<u64, LoadError> {
    s.parse::<f64>()
        .map(f64::to_bits)
        .map_err(|_| err(line, format!("invalid f64: {s}")))
}

fn parse_type(s: &str, line: usize) -> Result<TypeKind, LoadError> {
    match s {
        "void" => Ok(TypeKind::Void),
        "i32" => Ok(TypeKind::I32),
        "u32" => Ok(TypeKind::U32),
        "addr32" => Ok(TypeKind::Addr32),
        "i64" => Ok(TypeKind::I64),
        "f32" => Ok(TypeKind::F32),
        "f64" => Ok(TypeKind::F64),
        "float" => Ok(TypeKind::UnsupportedFloat),
        "long_double" => Ok(TypeKind::UnsupportedLongDouble),
        "aggregate" => Ok(TypeKind::UnsupportedAggregate),
        "varargs" => Ok(TypeKind::UnsupportedVarargs),
        "host_c_abi" => Ok(TypeKind::UnsupportedHostCAbi),
        _ => Err(err(line, format!("unknown type kind: {s}"))),
    }
}

fn parse_symbol_kind(s: &str, line: usize) -> Result<SymbolKind, LoadError> {
    match s {
        "function" => Ok(SymbolKind::Function),
        "data" => Ok(SymbolKind::Data),
        "runtime_helper" => Ok(SymbolKind::RuntimeHelper),
        _ => Err(err(line, format!("unknown symbol kind: {s}"))),
    }
}

fn parse_opcode(s: &str, line: usize) -> Result<Opcode, LoadError> {
    match s {
        "const_i32" => Ok(Opcode::ConstI32),
        "const_u32" => Ok(Opcode::ConstU32),
        "copy" => Ok(Opcode::Copy),
        "add_i32" => Ok(Opcode::AddI32),
        "sub_i32" => Ok(Opcode::SubI32),
        "mul_i32" => Ok(Opcode::MulI32),
        "eq_i32" => Ok(Opcode::EqI32),
        "ne_i32" => Ok(Opcode::NeI32),
        "lt_i32" => Ok(Opcode::LtI32),
        "add_u32" => Ok(Opcode::AddU32),
        "sub_u32" => Ok(Opcode::SubU32),
        "mul_u32" => Ok(Opcode::MulU32),
        "eq_u32" => Ok(Opcode::EqU32),
        "ne_u32" => Ok(Opcode::NeU32),
        "lt_u32" => Ok(Opcode::LtU32),
        "le_u32" => Ok(Opcode::LeU32),
        "gt_u32" => Ok(Opcode::GtU32),
        "ge_u32" => Ok(Opcode::GeU32),
        "branch" => Ok(Opcode::Branch),
        "branch_if" => Ok(Opcode::BranchIf),
        "call" => Ok(Opcode::Call),
        "ret" => Ok(Opcode::Ret),
        "trap" => Ok(Opcode::Trap),
        "alloc" => Ok(Opcode::Alloc),
        "load_i32" => Ok(Opcode::LoadI32),
        "load_u32" => Ok(Opcode::LoadU32),
        "store_i32" => Ok(Opcode::StoreI32),
        "store_u32" => Ok(Opcode::StoreU32),
        "load_u8" => Ok(Opcode::LoadU8),
        "store_u8" => Ok(Opcode::StoreU8),
        "addr_add" => Ok(Opcode::AddrAdd),
        "data_addr" => Ok(Opcode::DataAddr),
        "const_i64" => Ok(Opcode::ConstI64),
        "indirect_call" => Ok(Opcode::UnsupportedIndirectCall),
        "add_i64" => Ok(Opcode::AddI64),
        "sub_i64" => Ok(Opcode::SubI64),
        "mul_i64" => Ok(Opcode::MulI64),
        "eq_i64" => Ok(Opcode::EqI64),
        "ne_i64" => Ok(Opcode::NeI64),
        "lt_i64" => Ok(Opcode::LtI64),
        "load_i64" => Ok(Opcode::LoadI64),
        "store_i64" => Ok(Opcode::StoreI64),
        "const_f32" => Ok(Opcode::ConstF32),
        "const_f64" => Ok(Opcode::ConstF64),
        "add_f32" => Ok(Opcode::AddF32),
        "sub_f32" => Ok(Opcode::SubF32),
        "mul_f32" => Ok(Opcode::MulF32),
        "div_f32" => Ok(Opcode::DivF32),
        "neg_f32" => Ok(Opcode::NegF32),
        "eq_f32" => Ok(Opcode::EqF32),
        "ne_f32" => Ok(Opcode::NeF32),
        "lt_f32" => Ok(Opcode::LtF32),
        "le_f32" => Ok(Opcode::LeF32),
        "gt_f32" => Ok(Opcode::GtF32),
        "ge_f32" => Ok(Opcode::GeF32),
        "add_f64" => Ok(Opcode::AddF64),
        "sub_f64" => Ok(Opcode::SubF64),
        "mul_f64" => Ok(Opcode::MulF64),
        "div_f64" => Ok(Opcode::DivF64),
        "neg_f64" => Ok(Opcode::NegF64),
        "eq_f64" => Ok(Opcode::EqF64),
        "ne_f64" => Ok(Opcode::NeF64),
        "lt_f64" => Ok(Opcode::LtF64),
        "le_f64" => Ok(Opcode::LeF64),
        "gt_f64" => Ok(Opcode::GtF64),
        "ge_f64" => Ok(Opcode::GeF64),
        "i32_to_f32" => Ok(Opcode::I32ToF32),
        "f32_to_i32" => Ok(Opcode::F32ToI32),
        "i32_to_f64" => Ok(Opcode::I32ToF64),
        "f64_to_i32" => Ok(Opcode::F64ToI32),
        "f32_to_f64" => Ok(Opcode::F32ToF64),
        "f64_to_f32" => Ok(Opcode::F64ToF32),
        _ => Err(err(line, format!("unknown opcode: {s}"))),
    }
}

fn parse_operand(s: &str, line: usize) -> Result<Operand, LoadError> {
    let (kind, value) = s
        .split_once(':')
        .ok_or_else(|| err(line, format!("invalid operand: {s}")))?;
    match kind {
        "v" => Ok(Operand::Value(ValueId(parse_u32(value, line)?))),
        "i" => Ok(Operand::ImmI32(parse_i32(value, line)?)),
        "u" => Ok(Operand::ImmU32(parse_u32(value, line)?)),
        "l" => Ok(Operand::ImmI64(parse_i64(value, line)?)),
        "f32" => Ok(Operand::ImmF32(parse_f32_bits(value, line)?)),
        "f64" => Ok(Operand::ImmF64(parse_f64_bits(value, line)?)),
        "b" => Ok(Operand::Block(BlockId(parse_u32(value, line)?))),
        "f" => Ok(Operand::Function(FunctionId(parse_u32(value, line)?))),
        "s" => Ok(Operand::Symbol(SymbolId(parse_u32(value, line)?))),
        "t" => Ok(Operand::Type(TypeId(parse_u32(value, line)?))),
        _ => Err(err(line, format!("unknown operand kind: {kind}"))),
    }
}

fn parse_type_list(s: &str, line: usize) -> Result<Vec<TypeId>, LoadError> {
    if s == "-" {
        return Ok(Vec::new());
    }
    s.split(',')
        .map(|part| parse_u32(part, line).map(TypeId))
        .collect()
}

fn parse_hex_bytes(s: &str, line: usize) -> Result<Vec<u8>, LoadError> {
    if s == "-" {
        return Ok(Vec::new());
    }
    if s.len() % 2 != 0 {
        return Err(err(line, "hex byte string must have even length"));
    }
    let mut bytes = Vec::new();
    for idx in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[idx..idx + 2], 16)
            .map_err(|_| err(line, format!("invalid hex byte: {}", &s[idx..idx + 2])))?;
        bytes.push(byte);
    }
    Ok(bytes)
}

fn err(line: usize, message: impl Into<String>) -> LoadError {
    LoadError::InvalidLine {
        line,
        message: message.into(),
    }
}
