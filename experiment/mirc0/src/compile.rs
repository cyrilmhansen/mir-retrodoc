use mircap::{ModuleImage, Function, SymbolKind, TypeKind};
use crate::error::CompileError;
use crate::c_emit::{emit_type, emit_instruction};
use crate::runtime_c::RUNTIME_HEADER;
use crate::pretty::pretty_print_c;

pub fn compile(image: &ModuleImage, entry_name: &str) -> Result<String, CompileError> {
    // 1. Validate the ModuleImage first
    image.validate().map_err(CompileError::Validation)?;

    // 2. Validate entry function constraints
    let entry_func = find_entry_function(image, entry_name)?;
    validate_entry_signature(image, entry_func)?;

    let mut out = String::new();

    // Emit headers
    out.push_str(RUNTIME_HEADER);
    out.push('\n');

    // Emit forward declarations of all MIR functions
    out.push_str("/* Forward Declarations */\n");
    for func in &image.functions {
        let decl = emit_function_declaration(image, func)?;
        out.push_str(&decl);
        out.push_str(";\n");
    }
    out.push('\n');

    // Emit data segment arrays
    out.push_str("/* Data Segments */\n");
    for (idx, ds) in image.data_segments.iter().enumerate() {
        out.push_str(&format!("static const uint8_t data_seg_{}[] = {{", idx));
        if ds.bytes.is_empty() {
            out.push_str("0");
        } else {
            let byte_strs: Vec<String> = ds.bytes.iter().map(|b| format!("0x{:02x}", b)).collect();
            out.push_str(&byte_strs.join(", "));
        }
        out.push_str("};\n");
    }
    out.push('\n');

    // Emit init_data_segments function
    out.push_str("static void init_data_segments(void) {\n");
    for (idx, ds) in image.data_segments.iter().enumerate() {
        let len = ds.bytes.len() as u32;
        out.push_str(&format!("    /* Segment {} */\n", idx));
        if len > 0 {
            out.push_str(&format!("    if ({0}u > MEMORY_SIZE || {1}u > MEMORY_SIZE - {0}u) {{ mir_trap(14); }}\n", ds.offset, len));
            out.push_str(&format!("    for (uint32_t i = 0; i < {1}u; i++) {{ g_memory[{0}u + i] = data_seg_{2}[i]; }}\n", ds.offset, len, idx));
        }
        
        let zero_start = ds.offset + len;
        if ds.zero_fill > 0 {
            out.push_str(&format!("    if ({0}u > MEMORY_SIZE || {1}u > MEMORY_SIZE - {0}u) {{ mir_trap(14); }}\n", zero_start, ds.zero_fill));
            out.push_str(&format!("    for (uint32_t i = 0; i < {1}u; i++) {{ g_memory[{0}u + i] = 0; }}\n", zero_start, ds.zero_fill));
        }
        
        let end = zero_start + ds.zero_fill;
        out.push_str(&format!("    if ({0}u > g_heap_ptr) {{ g_heap_ptr = {0}u; }}\n", end));
    }
    out.push_str("}\n\n");

    // Emit function implementations
    for func in &image.functions {
        let impl_str = emit_function_implementation(image, func)?;
        out.push_str(&impl_str);
        out.push('\n');
    }

    // Emit entry wrapper main
    let entry_ret_type = if entry_func.results.is_empty() {
        TypeKind::Void
    } else {
        image.type_kind(entry_func.results[0]).unwrap_or(TypeKind::Void)
    };

    out.push_str("int main(void) {\n");
    out.push_str("    init_data_segments();\n");
    match entry_ret_type {
        TypeKind::Void => {
            out.push_str(&format!("    mir_fn_{}();\n", entry_func.id.0));
            out.push_str("    printf(\"Result: void\\n\");\n");
        }
        TypeKind::I32 => {
            out.push_str(&format!("    int32_t res = mir_fn_{}();\n", entry_func.id.0));
            out.push_str("    printf(\"Result: i32 %\" PRId32 \"\\n\", res);\n");
        }
        TypeKind::U32 => {
            out.push_str(&format!("    uint32_t res = mir_fn_{}();\n", entry_func.id.0));
            out.push_str("    printf(\"Result: u32 %\" PRIu32 \"\\n\", res);\n");
        }
        TypeKind::Addr32 => {
            out.push_str(&format!("    uint32_t res = mir_fn_{}();\n", entry_func.id.0));
            out.push_str("    printf(\"Result: addr32 %\" PRIu32 \"\\n\", res);\n");
        }
        _ => return Err(CompileError::UnsupportedType(entry_ret_type)),
    }
    out.push_str("    return 0;\n");
    out.push_str("}\n");

    Ok(pretty_print_c(&out))
}

fn find_entry_function<'a>(image: &'a ModuleImage, name: &str) -> Result<&'a Function, CompileError> {
    image.functions
        .iter()
        .find(|f| {
            image.symbol(f.symbol)
                .map(|sym| sym.kind == SymbolKind::Function && sym.name == name)
                .unwrap_or(false)
        })
        .ok_or_else(|| CompileError::EntryFunctionNotFound(name.to_string()))
}

fn validate_entry_signature(image: &ModuleImage, func: &Function) -> Result<(), CompileError> {
    if !func.params.is_empty() {
        return Err(CompileError::InvalidEntrySignature("Entry function must have 0 parameters".to_string()));
    }
    if func.results.len() > 1 {
        return Err(CompileError::InvalidEntrySignature("Entry function must have at most 1 result".to_string()));
    }
    if func.results.len() == 1 {
        let ret_ty = image.type_kind(func.results[0]).unwrap_or(TypeKind::Void);
        if !matches!(ret_ty, TypeKind::I32 | TypeKind::U32 | TypeKind::Addr32) {
            return Err(CompileError::InvalidEntrySignature(format!("Unsupported entry function result type: {:?}", ret_ty)));
        }
    }
    Ok(())
}

fn emit_function_declaration(image: &ModuleImage, func: &Function) -> Result<String, CompileError> {
    let ret_str = if func.results.is_empty() {
        "void"
    } else if func.results.len() == 1 {
        let kind = image.type_kind(func.results[0]).ok_or(CompileError::UnsupportedType(TypeKind::Void))?;
        emit_type(kind)?
    } else {
        return Err(CompileError::MultipleResultsNotSupported);
    };

    let mut params = Vec::new();
    for (idx, &param_ty) in func.params.iter().enumerate() {
        let kind = image.type_kind(param_ty).ok_or(CompileError::UnsupportedType(TypeKind::Void))?;
        let type_str = emit_type(kind)?;
        params.push(format!("{} v{}", type_str, idx));
    }

    let params_str = if params.is_empty() {
        "void".to_string()
    } else {
        params.join(", ")
    };

    Ok(format!("{} mir_fn_{}({})", ret_str, func.id.0, params_str))
}

fn emit_function_implementation(image: &ModuleImage, func: &Function) -> Result<String, CompileError> {
    use std::collections::HashSet;
    use mircap::Operand;

    let mut out = String::new();
    let decl = emit_function_declaration(image, func)?;
    out.push_str(&decl);
    out.push_str(" {\n");

    // Scan all instruction operands to find block labels that are actually targeted
    let mut used_blocks = HashSet::new();
    for &block_id in &func.blocks {
        if let Some(block) = image.block(block_id) {
            for &insn_id in &block.instructions {
                if let Some(insn) = image.instruction(insn_id) {
                    for op in &insn.operands {
                        if let Operand::Block(bid) = op {
                            used_blocks.insert(*bid);
                        }
                    }
                }
            }
        }
    }

    // Declare local variables (excluding parameters)
    // Local variables are indices from func.params.len() up to func.value_count
    for i in func.params.len()..(func.value_count as usize) {
        let ty_id = func.value_types.get(i).ok_or(CompileError::UnsupportedType(TypeKind::Void))?;
        let kind = image.type_kind(*ty_id).ok_or(CompileError::UnsupportedType(TypeKind::Void))?;
        if kind != TypeKind::Void {
            let type_str = emit_type(kind)?;
            // Rule 7: Initialize to prevent uninitialized behavior
            let init_val = if matches!(kind, TypeKind::I32) { "0" } else { "0u" };
            out.push_str(&format!("    {} v{} = {};\n", type_str, i, init_val));
        }
    }

    // Cast parameters and local variables to void to prevent unused variable/parameter warnings
    for idx in 0..func.params.len() {
        out.push_str(&format!("    (void)v{};\n", idx));
    }
    for i in func.params.len()..(func.value_count as usize) {
        let ty_id = func.value_types.get(i).ok_or(CompileError::UnsupportedType(TypeKind::Void))?;
        let kind = image.type_kind(*ty_id).ok_or(CompileError::UnsupportedType(TypeKind::Void))?;
        if kind != TypeKind::Void {
            out.push_str(&format!("    (void)v{};\n", i));
        }
    }

    // Emit blocks
    for &block_id in &func.blocks {
        let block = image.block(block_id).ok_or_else(|| CompileError::Validation(vec![]))?;
        if used_blocks.contains(&block_id) {
            out.push_str(&format!("block_{}:\n", block.id.0));
        }
        
        for &insn_id in &block.instructions {
            let insn = image.instruction(insn_id).ok_or_else(|| CompileError::Validation(vec![]))?;
            let insn_c = emit_instruction(insn, image)?;
            out.push_str(&format!("    {}\n", insn_c));
        }
    }

    out.push_str("}\n");
    Ok(out)
}
