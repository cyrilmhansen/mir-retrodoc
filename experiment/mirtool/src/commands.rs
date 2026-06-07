use crate::error::CliError;
use crate::io::load_image;
use mircap::image::ModuleImage;
use mirsem::runner::Runner;
use mirsem::trap::ExecutionTrap;
use std::path::Path;

fn trap_info(trap: &ExecutionTrap) -> (u32, &'static str) {
    match trap {
        ExecutionTrap::StackOverflow { .. } => (1, "StackOverflow"),
        ExecutionTrap::FuelExhausted { .. } => (2, "FuelExhausted"),
        ExecutionTrap::ExplicitTrap { .. } => (3, "ExplicitTrap"),
        ExecutionTrap::OutOfMemory { .. } => (11, "OutOfMemory"),
        ExecutionTrap::HeapStackCollision { .. } => (12, "HeapStackCollision"),
        ExecutionTrap::OutOfBoundsLoad { .. } => (13, "OutOfBoundsLoad"),
        ExecutionTrap::OutOfBoundsStore { .. } => (14, "OutOfBoundsStore"),
        ExecutionTrap::MisalignedLoad { .. } => (15, "MisalignedLoad"),
        ExecutionTrap::MisalignedStore { .. } => (16, "MisalignedStore"),
        ExecutionTrap::AddressOverflow { .. } => (17, "AddressOverflow"),
        _ => (99, "Unknown"),
    }
}

fn trap_name(code: u32) -> &'static str {
    match code {
        1 => "StackOverflow",
        2 => "FuelExhausted",
        3 => "ExplicitTrap",
        11 => "OutOfMemory",
        12 => "HeapStackCollision",
        13 => "OutOfBoundsLoad",
        14 => "OutOfBoundsStore",
        15 => "MisalignedLoad",
        16 => "MisalignedStore",
        17 => "AddressOverflow",
        _ => "Unknown",
    }
}

pub fn cmd_validate(path: &str, format_opt: Option<&str>) -> Result<(), CliError> {
    let load_res = load_image(path, format_opt);
    let image = match load_res {
        Ok(img) => img,
        Err(err) => {
            println!("ERROR: LoadError: {}", err);
            return Ok(());
        }
    };

    match image.validate() {
        Ok(_) => {
            println!("OK");
            Ok(())
        }
        Err(errors) => {
            if let Some(err) = errors.first() {
                println!("ERROR: {:?}: {}", err.kind, err.message);
            } else {
                println!("ERROR: Unknown: Validation failed with no details");
            }
            Ok(())
        }
    }
}

pub fn cmd_encode(input_path: &str, output_path: &str, force: bool) -> Result<(), CliError> {
    let image = load_image(input_path, Some("text"))?;
    
    // Safety check for existing file
    let out_path = Path::new(output_path);
    if out_path.exists() && !force {
        return Err(CliError::Generic(format!(
            "Output file '{}' already exists. Use --force to overwrite.",
            output_path
        )));
    }

    let bytes = image.to_capnp_bytes();
    std::fs::write(output_path, bytes)?;
    Ok(())
}

pub fn cmd_decode(input_path: &str, format_opt: Option<&str>) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let text = image_to_text(&image);
    print!("{}", text);
    Ok(())
}

pub fn cmd_run(
    input_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    show_trace: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let mut runner = Runner::new(image, mirsem::ExecutionProfile::default())?;

    match runner.run_entry_by_name(entry_name, &[]) {
        Ok(res) => {
            if res.values.is_empty() {
                println!("Result: void");
            } else {
                for val in res.values {
                    match val {
                        mirsem::Value::Void => println!("Result: void"),
                        mirsem::Value::I32(v) => println!("Result: i32 {}", v),
                        mirsem::Value::U32(v) => println!("Result: u32 {}", v),
                        mirsem::Value::Addr32(v) => println!("Result: addr32 {}", v),
                    }
                }
            }
            if show_trace {
                print_trace_summary(&runner.trace_snapshot());
            }
            Ok(())
        }
        Err(mirsem::RunError::Trap(trap)) => {
            let (code, name) = trap_info(&trap);
            println!("Trap: {} {}", code, name);
            if show_trace {
                print_trace_summary(&runner.trace_snapshot());
            }
            Ok(())
        }
        Err(err) => Err(CliError::Run(err)),
    }
}

pub fn cmd_compile_c(
    input_path: &str,
    output_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let c_code = mirc0::compile(&image, entry_name)?;
    std::fs::write(output_path, c_code)?;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DiffOutcome {
    Success(Option<mirsem::Value>),
    Trap(u32),
}

pub fn cmd_diff(
    input_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    keep_temp: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;

    // 1. Run interpreter
    let mut runner = Runner::new(image.clone(), mirsem::ExecutionProfile::default())?;
    let expected = match runner.run_entry_by_name(entry_name, &[]) {
        Ok(res) => DiffOutcome::Success(res.values.first().cloned()),
        Err(mirsem::RunError::Trap(trap)) => {
            let (code, _) = trap_info(&trap);
            DiffOutcome::Trap(code)
        }
        Err(err) => {
            return Err(CliError::Generic(format!("Reference interpreter run failed: {:?}", err)));
        }
    };

    // 2. Generate C
    let c_code = mirc0::compile(&image, entry_name)?;

    // 3. Check for C compiler
    let cc_check = std::process::Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        println!("Host C compiler 'cc' is unavailable. Skipping differential execution verification.");
        return Ok(());
    }

    // 4. Write C source code and compile
    let cur_dir = std::env::current_dir()?;
    let c_path = cur_dir.join("temp_mirtool_diff.c");
    let bin_path = cur_dir.join("temp_mirtool_diff");

    std::fs::write(&c_path, c_code)?;

    let mut compile_cmd = std::process::Command::new("cc");
    compile_cmd.arg("-O0")
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-o")
        .arg(&bin_path)
        .arg(&c_path);

    let compile_output = compile_cmd.output();
    match compile_output {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !keep_temp {
                    let _ = std::fs::remove_file(&c_path);
                }
                println!("FAIL: C compilation failed:\n{}", stderr);
                return Ok(());
            }
        }
        Err(err) => {
            if !keep_temp {
                let _ = std::fs::remove_file(&c_path);
            }
            println!("FAIL: Failed to run C compiler: {}", err);
            return Ok(());
        }
    }

    // 5. Run compiled binary
    let run_output = std::process::Command::new(&bin_path).output();
    if !keep_temp {
        let _ = std::fs::remove_file(&c_path);
        let _ = std::fs::remove_file(&bin_path);
    }

    let output = match run_output {
        Ok(o) => o,
        Err(err) => {
            println!("FAIL: Failed to execute compiled binary: {}", err);
            return Ok(());
        }
    };

    // 6. Compare results
    match expected {
        DiffOutcome::Success(expected_val) => {
            if output.status.code() != Some(0) {
                println!("FAIL: Expected exit code 0 for normal return, got status {:?}", output.status);
                return Ok(());
            }
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let result_line = stdout_str.lines().find(|l| l.starts_with("Result: "));
            let expected_str = match expected_val {
                None | Some(mirsem::Value::Void) => "Result: void".to_string(),
                Some(mirsem::Value::I32(v)) => format!("Result: {}", v),
                Some(mirsem::Value::U32(v)) => format!("Result: {}", v),
                Some(mirsem::Value::Addr32(v)) => format!("Result: {}", v),
            };
            if result_line == Some(expected_str.as_str()) {
                println!("PASS");
            } else {
                println!("FAIL: Result mismatch. Expected '{}', got '{:?}'", expected_str, result_line);
            }
        }
        DiffOutcome::Trap(expected_code) => {
            if output.status.code() != Some(expected_code as i32) {
                println!("FAIL: Expected exit status to match trap code {}, got status {:?}", expected_code, output.status);
                return Ok(());
            }
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let trap_line = stderr_str.lines().find(|l| l.starts_with("Trap: "));
            if let Some(line) = trap_line {
                let expected_line = format!("Trap: {} {}", expected_code, trap_name(expected_code));
                if line == expected_line.as_str() {
                    println!("PASS");
                } else {
                    println!("FAIL: Trap line mismatch. Expected '{}', got '{}'", expected_line, line);
                }
            } else {
                println!("FAIL: Expected stderr to contain 'Trap: ' line. Stderr:\n{}", stderr_str);
            }
        }
    }

    Ok(())
}

fn print_trace_summary(snapshot: &mirsem::TraceSnapshot) {
    println!("--- Trace Summary ---");
    println!("Executed Instructions: {}", snapshot.executed_instruction_count);
    println!("Maximum Call Depth: {}", snapshot.maximum_call_depth_reached);
    println!("Allocations: {}", snapshot.allocation_count);
    println!("Allocated Bytes: {}", snapshot.allocated_bytes);
}

pub fn image_to_text(image: &ModuleImage) -> String {
    let mut out = String::new();
    out.push_str("# Note: Decode output is for debugging purposes and is not yet a canonical source format\n");
    out.push_str(&format!("mircap {}\n", image.header.schema_name));
    out.push_str(&format!("version {}\n", image.header.format_version));
    out.push_str(&format!("module {} {}\n", image.module.id, image.module.name));

    for ty in &image.types {
        let kind_str = match ty.kind {
            mircap::TypeKind::Void => "void",
            mircap::TypeKind::I32 => "i32",
            mircap::TypeKind::U32 => "u32",
            mircap::TypeKind::Addr32 => "addr32",
            mircap::TypeKind::UnsupportedI64 => "i64",
            mircap::TypeKind::UnsupportedFloat => "float",
            mircap::TypeKind::UnsupportedLongDouble => "long_double",
            mircap::TypeKind::UnsupportedAggregate => "aggregate",
            mircap::TypeKind::UnsupportedVarargs => "varargs",
            mircap::TypeKind::UnsupportedHostCAbi => "host_c_abi",
        };
        out.push_str(&format!("type {} {}\n", ty.id.0, kind_str));
    }

    for sym in &image.symbols {
        let kind_str = match sym.kind {
            mircap::SymbolKind::Function => "function",
            mircap::SymbolKind::Data => "data",
            mircap::SymbolKind::RuntimeHelper => "runtime_helper",
        };
        out.push_str(&format!("symbol {} {} {}\n", sym.id.0, sym.name, kind_str));
    }

    fn fmt_type_list(list: &[mircap::ids::TypeId]) -> String {
        if list.is_empty() {
            "-".to_string()
        } else {
            list.iter().map(|t| t.0.to_string()).collect::<Vec<_>>().join(",")
        }
    }

    for func in &image.functions {
        out.push_str(&format!(
            "function {} {} {} {} {} {} {}\n",
            func.id.0,
            func.symbol.0,
            fmt_type_list(&func.params),
            fmt_type_list(&func.results),
            func.value_count,
            func.flags,
            fmt_type_list(&func.value_types)
        ));
        for block_id in &func.blocks {
            out.push_str(&format!("func_block {} {}\n", func.id.0, block_id.0));
        }
    }

    for ds in &image.data_segments {
        fn fmt_bytes(bytes: &[u8]) -> String {
            if bytes.is_empty() {
                "-".to_string()
            } else {
                bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>()
            }
        }
        out.push_str(&format!(
            "data {} {} {} {}\n",
            ds.symbol.0,
            ds.offset,
            fmt_bytes(&ds.bytes),
            ds.zero_fill
        ));
    }

    for block in &image.blocks {
        let insn_ids = block.instructions.iter().map(|id| id.0.to_string()).collect::<Vec<_>>().join(" ");
        out.push_str(&format!("block {} {} {}\n", block.id.0, block.parent.0, insn_ids));
    }

    for insn in &image.instructions {
        let opcode_name = match insn.opcode {
            mircap::Opcode::ConstI32 => "const_i32",
            mircap::Opcode::ConstU32 => "const_u32",
            mircap::Opcode::Copy => "copy",
            mircap::Opcode::AddI32 => "add_i32",
            mircap::Opcode::SubI32 => "sub_i32",
            mircap::Opcode::MulI32 => "mul_i32",
            mircap::Opcode::EqI32 => "eq_i32",
            mircap::Opcode::NeI32 => "ne_i32",
            mircap::Opcode::LtI32 => "lt_i32",
            mircap::Opcode::AddU32 => "add_u32",
            mircap::Opcode::SubU32 => "sub_u32",
            mircap::Opcode::MulU32 => "mul_u32",
            mircap::Opcode::EqU32 => "eq_u32",
            mircap::Opcode::NeU32 => "ne_u32",
            mircap::Opcode::LtU32 => "lt_u32",
            mircap::Opcode::LeU32 => "le_u32",
            mircap::Opcode::GtU32 => "gt_u32",
            mircap::Opcode::GeU32 => "ge_u32",
            mircap::Opcode::Branch => "branch",
            mircap::Opcode::BranchIf => "branch_if",
            mircap::Opcode::Call => "call",
            mircap::Opcode::Ret => "ret",
            mircap::Opcode::Trap => "trap",
            mircap::Opcode::Alloc => "alloc",
            mircap::Opcode::LoadI32 => "load_i32",
            mircap::Opcode::LoadU32 => "load_u32",
            mircap::Opcode::StoreI32 => "store_i32",
            mircap::Opcode::StoreU32 => "store_u32",
            mircap::Opcode::LoadU8 => "load_u8",
            mircap::Opcode::StoreU8 => "store_u8",
            mircap::Opcode::AddrAdd => "addr_add",
            mircap::Opcode::DataAddr => "data_addr",
            mircap::Opcode::UnsupportedI64 => "unsupported_i64",
            mircap::Opcode::UnsupportedIndirectCall => "indirect_call",
        };

        let mut parts = Vec::new();
        for &res in &insn.results {
            parts.push(format!("r:{}", res.0));
        }
        for op in &insn.operands {
            let op_str = match op {
                mircap::Operand::Value(val) => format!("v:{}", val.0),
                mircap::Operand::ImmI32(val) => format!("i:{}", val),
                mircap::Operand::ImmU32(val) => format!("u:{}", val),
                mircap::Operand::Block(val) => format!("b:{}", val.0),
                mircap::Operand::Function(val) => format!("f:{}", val.0),
                mircap::Operand::Symbol(val) => format!("s:{}", val.0),
                mircap::Operand::Type(val) => format!("t:{}", val.0),
            };
            parts.push(op_str);
        }
        if parts.is_empty() {
            out.push_str(&format!("insn {} {}\n", insn.id.0, opcode_name));
        } else {
            out.push_str(&format!("insn {} {} {}\n", insn.id.0, opcode_name, parts.join(" ")));
        }
    }

    out
}
