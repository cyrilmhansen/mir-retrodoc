use crate::error::CliError;
use crate::io::{detect_format, load_image, FileFormat};
use mircap::image::ModuleImage;
use mircap::Opcode;
use mirplan::{DataSegmentPlan, LoweredInstruction, LoweredOperand, LoweredProgram};
use mirsem::runner::Runner;
use mirsem::trap::ExecutionTrap;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

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
                        mirsem::Value::I64(v) => println!("Result: i64 {}", v),
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

pub fn cmd_plan(input_path: &str, format_opt: Option<&str>) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    print!("{}", mirplan::format_plan(&plan));
    Ok(())
}

pub fn cmd_lower(
    input_path: &str,
    format_opt: Option<&str>,
    optimize: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }
    print!("{}", mirplan::format_lowered(&lowered));
    Ok(())
}

pub fn cmd_bench_load(
    input_path: &str,
    format_opt: Option<&str>,
    iterations: u32,
) -> Result<(), CliError> {
    let format = detect_format(input_path, format_opt)?;
    let start = Instant::now();
    let mut checksum = 0usize;

    match format {
        FileFormat::Text => {
            let text = std::fs::read_to_string(input_path)?;
            for _ in 0..iterations {
                let image = ModuleImage::from_text(&text)?;
                checksum = checksum
                    .wrapping_add(image.module.name.len())
                    .wrapping_add(image.functions.len())
                    .wrapping_add(image.instructions.len());
                black_box(&image);
            }
            print_bench_result("text", iterations, start.elapsed().as_nanos(), checksum);
        }
        FileFormat::Binary => {
            let bytes = std::fs::read(input_path)?;
            for _ in 0..iterations {
                let image = ModuleImage::from_capnp_bytes(&bytes)?;
                checksum = checksum
                    .wrapping_add(image.module.name.len())
                    .wrapping_add(image.functions.len())
                    .wrapping_add(image.instructions.len());
                black_box(&image);
            }
            print_bench_result("binary", iterations, start.elapsed().as_nanos(), checksum);
        }
    }

    Ok(())
}

fn print_bench_result(format: &str, iterations: u32, total_ns: u128, checksum: usize) {
    let avg_ns = total_ns / u128::from(iterations);
    println!("format: {format}");
    println!("iterations: {iterations}");
    println!("total_ns: {total_ns}");
    println!("avg_ns: {avg_ns}");
    println!("checksum: {checksum}");
}

pub fn cmd_compile_c(
    input_path: &str,
    output_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    optimize: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    use mirplan::Backend;
    let backend = mirc0::C11Backend::new(entry_name);
    let c_code = backend.compile(&lowered)?;

    std::fs::write(output_path, c_code)?;
    Ok(())
}

pub fn cmd_compile_rv32i(
    input_path: &str,
    output_path: &str,
    format_opt: Option<&str>,
    optimize: bool,
) -> Result<(), CliError> {
    let image = load_image(input_path, format_opt)?;
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    use mirplan::Backend;
    let backend = mirrv32::Riscv32Backend;
    let asm_code = backend.compile(&lowered)
        .map_err(|err| CliError::Generic(err.to_string()))?;

    std::fs::write(output_path, asm_code)?;
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
    optimize: bool,
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
            return Err(CliError::Generic(format!(
                "Reference interpreter run failed: {:?}",
                err
            )));
        }
    };

    // 2. Generate C
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    use mirplan::Backend;
    let backend = mirc0::C11Backend::new(entry_name);
    let c_code = backend.compile(&lowered)?;

    // 3. Check for C compiler
    let cc_check = std::process::Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        println!(
            "Host C compiler 'cc' is unavailable. Skipping differential execution verification."
        );
        return Ok(());
    }

    // 4. Write C source code and compile
    let cur_dir = std::env::current_dir()?;
    let input_name = Path::new(input_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("temp")
        .replace('.', "_");
    let c_path = cur_dir.join(format!("temp_mirtool_diff_{}.c", input_name));
    let bin_path = cur_dir.join(format!("temp_mirtool_diff_{}", input_name));

    std::fs::write(&c_path, c_code)?;

    let mut compile_cmd = std::process::Command::new("cc");
    compile_cmd
        .arg("-O0")
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
                println!(
                    "FAIL: Expected exit code 0 for normal return, got status {:?}",
                    output.status
                );
                return Ok(());
            }
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let result_line = stdout_str.lines().find(|l| l.starts_with("Result: "));
            let expected_str = match expected_val {
                None | Some(mirsem::Value::Void) => "Result: void".to_string(),
                Some(mirsem::Value::I32(v)) => format!("Result: i32 {}", v),
                Some(mirsem::Value::U32(v)) => format!("Result: u32 {}", v),
                Some(mirsem::Value::Addr32(v)) => format!("Result: addr32 {}", v),
                Some(mirsem::Value::I64(v)) => format!("Result: i64 {}", v),
            };
            if result_line == Some(expected_str.as_str()) {
                println!("PASS");
            } else {
                println!(
                    "FAIL: Result mismatch. Expected '{}', got '{:?}'",
                    expected_str, result_line
                );
            }
        }
        DiffOutcome::Trap(expected_code) => {
            if output.status.code() != Some(expected_code as i32) {
                println!(
                    "FAIL: Expected exit status to match trap code {}, got status {:?}",
                    expected_code, output.status
                );
                return Ok(());
            }
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let trap_line = stderr_str.lines().find(|l| l.starts_with("Trap: "));
            if let Some(line) = trap_line {
                let expected_line = format!("Trap: {} {}", expected_code, trap_name(expected_code));
                if line == expected_line.as_str() {
                    println!("PASS");
                } else {
                    println!(
                        "FAIL: Trap line mismatch. Expected '{}', got '{}'",
                        expected_line, line
                    );
                }
            } else {
                println!(
                    "FAIL: Expected stderr to contain 'Trap: ' line. Stderr:\n{}",
                    stderr_str
                );
            }
        }
    }

    Ok(())
}

fn print_trace_summary(snapshot: &mirsem::TraceSnapshot) {
    println!("--- Trace Summary ---");
    println!(
        "Executed Instructions: {}",
        snapshot.executed_instruction_count
    );
    println!(
        "Maximum Call Depth: {}",
        snapshot.maximum_call_depth_reached
    );
    println!("Allocations: {}", snapshot.allocation_count);
    println!("Allocated Bytes: {}", snapshot.allocated_bytes);
}

pub fn image_to_text(image: &ModuleImage) -> String {
    let mut out = String::new();
    out.push_str("# Note: Decode output is for debugging purposes and is not yet a canonical source format\n");
    out.push_str(&format!("mircap {}\n", image.header.schema_name));
    out.push_str(&format!("version {}\n", image.header.format_version));
    out.push_str(&format!(
        "module {} {}\n",
        image.module.id, image.module.name
    ));

    for ty in &image.types {
        let kind_str = match ty.kind {
            mircap::TypeKind::Void => "void",
            mircap::TypeKind::I32 => "i32",
            mircap::TypeKind::U32 => "u32",
            mircap::TypeKind::Addr32 => "addr32",
            mircap::TypeKind::I64 => "i64",
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
            list.iter()
                .map(|t| t.0.to_string())
                .collect::<Vec<_>>()
                .join(",")
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
                bytes
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>()
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
        let insn_ids = block
            .instructions
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "block {} {} {}\n",
            block.id.0, block.parent.0, insn_ids
        ));
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
            mircap::Opcode::ConstI64 => "const_i64",
            mircap::Opcode::AddI64 => "add_i64",
            mircap::Opcode::SubI64 => "sub_i64",
            mircap::Opcode::MulI64 => "mul_i64",
            mircap::Opcode::EqI64 => "eq_i64",
            mircap::Opcode::NeI64 => "ne_i64",
            mircap::Opcode::LtI64 => "lt_i64",
            mircap::Opcode::LoadI64 => "load_i64",
            mircap::Opcode::StoreI64 => "store_i64",
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
                mircap::Operand::ImmI64(val) => format!("l:{}", val),
            };
            parts.push(op_str);
        }
        if parts.is_empty() {
            out.push_str(&format!("insn {} {}\n", insn.id.0, opcode_name));
        } else {
            out.push_str(&format!(
                "insn {} {} {}\n",
                insn.id.0,
                opcode_name,
                parts.join(" ")
            ));
        }
    }

    out
}

pub fn cmd_diff_upstream(
    input_path: &str,
    format_opt: Option<&str>,
    entry_name: &str,
    keep_temp: bool,
    optimize: bool,
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
            return Err(CliError::Generic(format!(
                "Reference interpreter run failed: {:?}",
                err
            )));
        }
    };

    // 2. Generate original MIR
    let space = mirspace::ProgramSpace::from_module_image(&image)
        .map_err(|err| CliError::Generic(format!("Program space construction failed: {err}")))?;
    let plan = mirplan::build_compile_plan(&space);
    let mut lowered = mirplan::lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    let mir_code = translate_to_upstream_mir(&lowered, entry_name);

    // 3. Write MIR source code and compile
    let cur_dir = std::env::current_dir()?;
    let input_name = Path::new(input_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("temp")
        .replace('.', "_");
    let mir_path = cur_dir.join(format!("temp_mirtool_upstream_{}.mir", input_name));
    let bmir_path = cur_dir.join(format!("temp_mirtool_upstream_{}.bmir", input_name));

    std::fs::write(&mir_path, mir_code)?;

    let m2b_path = "/home/john/project/mir-preservation/git/mir-restored/m2b";
    let mir_bin_run_path = "/home/john/project/mir-preservation/git/mir-restored/mir-bin-run";

    let mut compile_cmd = std::process::Command::new(m2b_path);
    compile_cmd.stdin(std::fs::File::open(&mir_path)?);
    compile_cmd.stdout(std::fs::File::create(&bmir_path)?);

    let compile_output = compile_cmd.output();
    match compile_output {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !keep_temp {
                    let _ = std::fs::remove_file(&mir_path);
                    let _ = std::fs::remove_file(&bmir_path);
                }
                println!("FAIL: m2b compilation failed:\n{}", stderr);
                return Ok(());
            }
        }
        Err(err) => {
            if !keep_temp {
                let _ = std::fs::remove_file(&mir_path);
                let _ = std::fs::remove_file(&bmir_path);
            }
            println!("FAIL: Failed to run m2b compiler: {}", err);
            return Ok(());
        }
    }

    // 4. Run compiled binary with mir-bin-run
    let run_output = std::process::Command::new(mir_bin_run_path)
        .arg(&bmir_path)
        .arg("dummy_name")
        .output();

    if !keep_temp {
        let _ = std::fs::remove_file(&mir_path);
        let _ = std::fs::remove_file(&bmir_path);
    }

    let output = match run_output {
        Ok(o) => o,
        Err(err) => {
            println!(
                "FAIL: Failed to execute compiled binary under mir-bin-run: {}",
                err
            );
            return Ok(());
        }
    };

    let exit_code = output.status.code();

    // 5. Compare exit codes
    match expected {
        DiffOutcome::Success(expected_val) => {
            let expected_code = match expected_val {
                None | Some(mirsem::Value::Void) => 0,
                Some(mirsem::Value::I32(v)) => v,
                Some(mirsem::Value::U32(v)) => v as i32,
                Some(mirsem::Value::Addr32(v)) => v as i32,
                Some(mirsem::Value::I64(v)) => v as i32,
            };
            let expected_exit_status = (expected_code & 0xff) as i32;
            let actual_exit_status = exit_code.map(|c| c & 0xff);
            if actual_exit_status == Some(expected_exit_status) {
                println!("PASS");
            } else {
                println!(
                    "FAIL: Result mismatch. Expected exit code {} (masked: {}), got {:?}",
                    expected_code, expected_exit_status, exit_code
                );
            }
        }
        DiffOutcome::Trap(expected_trap_code) => {
            let expected_exit_status = (expected_trap_code & 0xff) as i32;
            let actual_exit_status = exit_code.map(|c| c & 0xff);
            if actual_exit_status == Some(expected_exit_status) {
                println!("PASS");
            } else {
                println!(
                    "FAIL: Trap mismatch. Expected exit status to match trap code {} (masked: {}), got {:?}",
                    expected_trap_code, expected_exit_status, exit_code
                );
            }
        }
    }

    Ok(())
}

fn map_type(kind: mircap::TypeKind) -> &'static str {
    match kind {
        mircap::TypeKind::Void => "void",
        mircap::TypeKind::I32 => "i32",
        mircap::TypeKind::U32 => "u32",
        mircap::TypeKind::Addr32 => "p",
        mircap::TypeKind::I64 => "i64",
        _ => "i64",
    }
}

pub fn translate_to_upstream_mir(program: &LoweredProgram, entry_name: &str) -> String {
    let mut out = String::new();

    // Module header
    out.push_str(&format!("{}: module\n", program.module_name));

    // Standard imports and prototypes
    out.push_str("import malloc, abort, exit\n");
    out.push_str("proto_malloc: proto p, i64:size\n");
    out.push_str("proto_abort: proto\n");
    out.push_str("proto_exit: proto i64:code\n\n");

    // Globals
    out.push_str("g_heap_ptr: i64 0\n");
    out.push_str("g_memory: bss 1048576\n\n");

    // Emit data segment declarations
    for (idx, segment) in program.data_segments.iter().enumerate() {
        out.push_str(&format!("data_seg_{}: u8 ", idx));
        if segment.bytes.is_empty() {
            out.push_str("0");
        } else {
            let byte_strs = segment
                .bytes
                .iter()
                .map(|b| format!("0x{b:02x}"))
                .collect::<Vec<_>>();
            out.push_str(&byte_strs.join(", "));
        }
        out.push_str("\n");
    }
    out.push('\n');

    // Emit static safety helpers
    out.push_str(HELPER_FUNCTIONS);

    // Emit dynamic init_data_segments function
    out.push_str("init_data_segments: func\n");
    out.push_str(
        "                    local i64:mem_addr, i64:seg_addr, i64:i, i64:temp, i64:val\n",
    );
    for (idx, segment) in program.data_segments.iter().enumerate() {
        let len = segment.bytes.len();
        out.push_str(&format!("                    # Segment {}\n", idx));
        if len > 0 {
            out.push_str("                    mov i, 0\n");
            out.push_str(&format!("L_loop_seg_{}:\n", idx));
            out.push_str(&format!(
                "                    bge L_end_seg_{}, i, {}\n",
                idx, len
            ));
            out.push_str(&format!(
                "                    add temp, {}, i\n",
                segment.offset
            ));
            out.push_str(&format!(
                "                    mov seg_addr, data_seg_{}\n",
                idx
            ));
            out.push_str("                    add seg_addr, seg_addr, i\n");
            out.push_str("                    mov val, u8:0(seg_addr)\n");
            out.push_str("                    mov mem_addr, g_memory\n");
            out.push_str("                    add mem_addr, mem_addr, temp\n");
            out.push_str("                    mov u8:0(mem_addr), val\n");
            out.push_str("                    add i, i, 1\n");
            out.push_str(&format!("                    jmp L_loop_seg_{}\n", idx));
            out.push_str(&format!("L_end_seg_{}:\n", idx));
        }
        if segment.zero_fill > 0 {
            let zero_start = segment.offset as usize + len;
            out.push_str("                    mov i, 0\n");
            out.push_str(&format!("L_zero_loop_seg_{}:\n", idx));
            out.push_str(&format!(
                "                    bge L_zero_end_seg_{}, i, {}\n",
                idx, segment.zero_fill
            ));
            out.push_str(&format!(
                "                    add temp, {}, i\n",
                zero_start
            ));
            out.push_str("                    mov mem_addr, g_memory\n");
            out.push_str("                    add mem_addr, mem_addr, temp\n");
            out.push_str("                    mov u8:0(mem_addr), 0\n");
            out.push_str("                    add i, i, 1\n");
            out.push_str(&format!(
                "                    jmp L_zero_loop_seg_{}\n",
                idx
            ));
            out.push_str(&format!("L_zero_end_seg_{}:\n", idx));
        }
    }
    out.push_str("                    ret\n");
    out.push_str("                    endfunc\n\n");

    // Prototypes for other functions in the module
    for func in &program.functions {
        let name = if func.name == entry_name {
            "main"
        } else {
            &func.name
        };
        let results_str = func
            .results
            .iter()
            .map(|t| map_type(*t))
            .collect::<Vec<_>>()
            .join(", ");
        let params_str = func
            .params
            .iter()
            .map(|p| format!("{}:v_{}", map_type(p.type_kind), p.id.0))
            .collect::<Vec<_>>()
            .join(", ");

        let signature = if results_str.is_empty() {
            if params_str.is_empty() {
                "".to_string()
            } else {
                format!(" {}", params_str)
            }
        } else {
            if params_str.is_empty() {
                format!(" {}", results_str)
            } else {
                format!(" {}, {}", results_str, params_str)
            }
        };
        out.push_str(&format!("proto_{}: proto{}\n", name, signature));
    }
    out.push('\n');

    // Translate functions
    let mut sorted_functions: Vec<_> = program.functions.iter().collect();
    sorted_functions.sort_by_key(|f| f.name == entry_name);
    for func in sorted_functions {
        let name = if func.name == entry_name {
            "main"
        } else {
            &func.name
        };
        let results_str = func
            .results
            .iter()
            .map(|t| map_type(*t))
            .collect::<Vec<_>>()
            .join(", ");
        let params_str = func
            .params
            .iter()
            .map(|p| format!("{}:v_{}", map_type(p.type_kind), p.id.0))
            .collect::<Vec<_>>()
            .join(", ");

        let signature = if results_str.is_empty() {
            if params_str.is_empty() {
                "".to_string()
            } else {
                format!(" {}", params_str)
            }
        } else {
            if params_str.is_empty() {
                format!(" {}", results_str)
            } else {
                format!(" {}, {}", results_str, params_str)
            }
        };
        out.push_str(&format!("{}: func{}\n", name, signature));

        // Declare local variables (all used registers except parameters, type i64)
        let param_ids: std::collections::HashSet<u32> =
            func.params.iter().map(|p| p.id.0).collect();
        let mut local_ids = std::collections::HashSet::new();
        for block in &func.blocks {
            for insn in &block.instructions {
                for r in &insn.writes {
                    if !param_ids.contains(&r.id.0) {
                        local_ids.insert(r.id.0);
                    }
                }
                for r in &insn.reads {
                    if !param_ids.contains(&r.id.0) {
                        local_ids.insert(r.id.0);
                    }
                }
                for op in &insn.operands {
                    if let LoweredOperand::Value(val) = op {
                        if !param_ids.contains(&val.id.0) {
                            local_ids.insert(val.id.0);
                        }
                    }
                }
            }
        }

        let mut sorted_ids: Vec<_> = local_ids.into_iter().collect();
        sorted_ids.sort();
        let mut decls = sorted_ids
            .iter()
            .map(|id| format!("i64:v_{}", id))
            .collect::<Vec<_>>();
        decls.push("i64:addr_add_temp".to_string());
        let local_declarations = decls.join(", ");
        out.push_str(&format!(
            "                    local {}\n",
            local_declarations
        ));

        // If this is the entry function ("main"), call init_data_segments first
        if name == "main" {
            out.push_str("                    call proto_init_data_segments, init_data_segments\n");
        }

        // If the first block is not the entry block, jump to the entry block
        if let Some(first_block) = func.blocks.first() {
            if first_block.label.id != func.entry.id {
                out.push_str(&format!("                    jmp L_{}\n", func.entry.id.0));
            }
        }

        // Translate blocks
        for block in &func.blocks {
            out.push_str(&format!("L_{}:\n", block.label.id.0));
            for insn in &block.instructions {
                out.push_str("                    ");
                let insn_str = translate_instruction(insn, entry_name, &program.data_segments);
                out.push_str(&insn_str);
                out.push('\n');
            }
        }
        out.push_str("                    endfunc\n\n");
    }

    out.push_str("endmodule\n");
    out
}

fn translate_instruction(
    insn: &LoweredInstruction,
    entry_name: &str,
    data_segments: &[DataSegmentPlan],
) -> String {
    let dest_str = if !insn.writes.is_empty() {
        format!("v_{}", insn.writes[0].id.0)
    } else {
        String::new()
    };

    let format_op = |op: &LoweredOperand| -> String {
        match op {
            LoweredOperand::Value(val) => format!("v_{}", val.id.0),
            LoweredOperand::ImmI32(imm) => imm.to_string(),
            LoweredOperand::ImmU32(imm) => imm.to_string(),
            LoweredOperand::ImmI64(imm) => imm.to_string(),
            LoweredOperand::Block(lbl) => format!("L_{}", lbl.id.0),
            LoweredOperand::Function(f) => {
                let name = if f.name == entry_name {
                    "main"
                } else {
                    &f.name
                };
                name.to_string()
            }
            LoweredOperand::Symbol { name, .. } => name.clone(),
            _ => String::new(),
        }
    };

    match insn.opcode {
        Opcode::ConstI32 | Opcode::ConstU32 | Opcode::ConstI64 => {
            let val = format_op(&insn.operands[0]);
            format!("mov {}, {}", dest_str, val)
        }
        Opcode::Copy => {
            let val = format_op(&insn.operands[0]);
            format!("mov {}, {}", dest_str, val)
        }
        Opcode::AddI32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("adds {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::SubI32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("subs {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::MulI32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("muls {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::AddU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("adds {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::SubU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("subs {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::MulU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("umuls {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::EqI32 | Opcode::EqU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("eqs {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::NeI32 | Opcode::NeU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("nes {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LtI32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("lts {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LtU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("ults {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LeU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("ules {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::GtU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("ugts {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::GeU32 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("uges {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::Branch => {
            let lbl = format_op(&insn.operands[0]);
            format!("jmp {}", lbl)
        }
        Opcode::BranchIf => {
            let cond = format_op(&insn.operands[0]);
            let true_lbl = format_op(&insn.operands[1]);
            let false_lbl = format_op(&insn.operands[2]);
            format!(
                "bt {}, {}\n                    jmp {}",
                true_lbl, cond, false_lbl
            )
        }
        Opcode::Ret => {
            let read_strs: Vec<String> =
                insn.reads.iter().map(|v| format!("v_{}", v.id.0)).collect();
            if read_strs.is_empty() {
                "ret".to_string()
            } else {
                format!("ret {}", read_strs.join(", "))
            }
        }
        Opcode::Call => {
            let callee_ref = match insn
                .operands
                .iter()
                .find(|op| matches!(op, LoweredOperand::Function(_)))
            {
                Some(LoweredOperand::Function(f)) => f,
                _ => panic!("Call instruction must have function callee"),
            };
            let callee_name = if callee_ref.name == entry_name {
                "main"
            } else {
                &callee_ref.name
            };

            let args: Vec<String> = insn
                .operands
                .iter()
                .filter(|op| !matches!(op, LoweredOperand::Function(_)))
                .map(|op| format_op(op))
                .collect();

            let mut parts = vec![format!("proto_{}", callee_name), callee_name.to_string()];
            if !dest_str.is_empty() {
                parts.push(dest_str);
            }
            parts.extend(args);
            format!("call {}", parts.join(", "))
        }
        Opcode::Trap => "call proto_exit, exit, 3".to_string(),
        Opcode::Alloc => {
            let size = format_op(&insn.operands[0]);
            let align = format_op(&insn.operands[1]);
            format!(
                "call proto_mir_alloc, mir_alloc, {}, {}, {}",
                dest_str, size, align
            )
        }
        Opcode::LoadI32 => {
            let addr = format_op(&insn.operands[0]);
            format!(
                "call proto_mir_load_i32, mir_load_i32, {}, {}",
                dest_str, addr
            )
        }
        Opcode::LoadU32 => {
            let addr = format_op(&insn.operands[0]);
            format!(
                "call proto_mir_load_u32, mir_load_u32, {}, {}",
                dest_str, addr
            )
        }
        Opcode::StoreI32 => {
            let addr = format_op(&insn.operands[0]);
            let val = format_op(&insn.operands[1]);
            format!("call proto_mir_store_i32, mir_store_i32, {}, {}", addr, val)
        }
        Opcode::StoreU32 => {
            let addr = format_op(&insn.operands[0]);
            let val = format_op(&insn.operands[1]);
            format!("call proto_mir_store_u32, mir_store_u32, {}, {}", addr, val)
        }
        Opcode::LoadU8 => {
            let addr = format_op(&insn.operands[0]);
            format!(
                "call proto_mir_load_u8, mir_load_u8, {}, {}",
                dest_str, addr
            )
        }
        Opcode::StoreU8 => {
            let addr = format_op(&insn.operands[0]);
            let val = format_op(&insn.operands[1]);
            format!("call proto_mir_store_u8, mir_store_u8, {}, {}", addr, val)
        }
        Opcode::AddrAdd => {
            let base = format_op(&insn.operands[0]);
            let offset = format_op(&insn.operands[1]);
            let insn_id = insn.id.0;
            format!(
                "mov addr_add_temp, {base}\n                    adds {dest}, addr_add_temp, {offset}\n                    ublts L_overflow_addradd_insn_{insn_id}, {dest}, addr_add_temp\n                    uext32 {dest}, {dest}\n                    jmp L_ok_addradd_insn_{insn_id}\nL_overflow_addradd_insn_{insn_id}:\n                    call proto_exit, exit, 17\nL_ok_addradd_insn_{insn_id}:",
                dest = dest_str,
                base = base,
                offset = offset,
                insn_id = insn_id
            )
        }
        Opcode::DataAddr => {
            let sym_name = match &insn.operands[0] {
                LoweredOperand::Symbol { name, .. } => name.clone(),
                _ => String::new(),
            };
            let offset = format_op(&insn.operands[1]);
            let ds = data_segments
                .iter()
                .find(|ds| ds.name == sym_name)
                .expect("Data segment must exist for DataAddr instruction");
            let ds_len = ds.bytes.len() as u32 + ds.zero_fill;
            format!(
                "call proto_mir_data_addr, mir_data_addr, {}, {}, {}, {}",
                dest_str, ds.offset, offset, ds_len
            )
        }
        Opcode::AddI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("add {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::SubI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("sub {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::MulI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("mul {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::EqI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("eq {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::NeI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("ne {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LtI64 => {
            let lhs = format_op(&insn.operands[0]);
            let rhs = format_op(&insn.operands[1]);
            format!("lt {}, {}, {}", dest_str, lhs, rhs)
        }
        Opcode::LoadI64 => {
            let addr = format_op(&insn.operands[0]);
            format!(
                "call proto_mir_load_i64, mir_load_i64, {}, {}",
                dest_str, addr
            )
        }
        Opcode::StoreI64 => {
            let addr = format_op(&insn.operands[0]);
            let val = format_op(&insn.operands[1]);
            format!("call proto_mir_store_i64, mir_store_i64, {}, {}", addr, val)
        }
        _ => String::new(),
    }
}

const HELPER_FUNCTIONS: &str = r#"proto_init_data_segments: proto
proto_mir_alloc: proto i64, i64:size, i64:align
proto_mir_load_i32: proto i64, i64:addr
proto_mir_load_u32: proto i64, i64:addr
proto_mir_store_i32: proto i64:addr, i64:val
proto_mir_store_u32: proto i64:addr, i64:val
proto_mir_load_u8: proto i64, i64:addr
proto_mir_store_u8: proto i64:addr, i64:val
proto_mir_load_i64: proto i64, i64:addr
proto_mir_store_i64: proto i64:addr, i64:val
proto_mir_data_addr: proto i64, i64:base, i64:offset, i64:len

mir_trap: func i64:code
          call proto_exit, exit, code
          ret
          endfunc

mir_alloc: func i64, i64:size, i64:align
           local i64:mask, i64:heap_ptr_addr, i64:heap_val, i64:aligned, i64:end, i64:temp, i64:limit, i64:not_mask
           bne L_align_ok_1_alloc, align, 0
           call proto_exit, exit, 16
L_align_ok_1_alloc:
           sub temp, align, 1
           and temp, align, temp
           beq L_align_ok_2_alloc, temp, 0
           call proto_exit, exit, 16
L_align_ok_2_alloc:
           sub mask, align, 1
           mov heap_ptr_addr, g_heap_ptr
           mov heap_val, i64:0(heap_ptr_addr)
           mov limit, 4294967295
           sub limit, limit, mask
           ubgt L_oom_1_alloc, heap_val, limit
           jmp L_heap_ok_1_alloc
L_oom_1_alloc:
           call proto_exit, exit, 11
L_heap_ok_1_alloc:
           add aligned, heap_val, mask
           xor not_mask, mask, -1
           and aligned, aligned, not_mask
           mov limit, 4294967295
           sub limit, limit, aligned
           ubgt L_oom_2_alloc, size, limit
           jmp L_heap_ok_2_alloc
L_oom_2_alloc:
           call proto_exit, exit, 11
L_heap_ok_2_alloc:
           add end, aligned, size
           ubgt L_collision_alloc, end, 983040
           jmp L_heap_ok_3_alloc
L_collision_alloc:
           call proto_exit, exit, 12
L_heap_ok_3_alloc:
           mov i64:0(heap_ptr_addr), end
           uext32 aligned, aligned
           ret aligned
           endfunc

mir_load_i32: func i64, i64:addr
              local i64:rem, i64:mem_addr, i64:val
              mod rem, addr, 4
              beq L_align_ok_load_i32, rem, 0
              call proto_exit, exit, 15
L_align_ok_load_i32:
              ubgt L_bounds_ok_load_i32, addr, 1048572
              jmp L_bounds_ok2_load_i32
L_bounds_ok_load_i32:
              call proto_exit, exit, 13
L_bounds_ok2_load_i32:
              mov mem_addr, g_memory
              add mem_addr, mem_addr, addr
              mov val, i32:0(mem_addr)
              ret val
              endfunc

mir_load_u32: func i64, i64:addr
              local i64:rem, i64:mem_addr, i64:val
              mod rem, addr, 4
              beq L_align_ok_load_u32, rem, 0
              call proto_exit, exit, 15
L_align_ok_load_u32:
              ubgt L_bounds_ok_load_u32, addr, 1048572
              jmp L_bounds_ok2_load_u32
L_bounds_ok_load_u32:
              call proto_exit, exit, 13
L_bounds_ok2_load_u32:
              mov mem_addr, g_memory
              add mem_addr, mem_addr, addr
              mov val, u32:0(mem_addr)
              ret val
              endfunc

mir_store_i32: func i64:addr, i64:val
               local i64:rem, i64:mem_addr
               mod rem, addr, 4
               beq L_align_ok_store_i32, rem, 0
               call proto_exit, exit, 16
L_align_ok_store_i32:
               ubgt L_bounds_ok_store_i32, addr, 1048572
               jmp L_bounds_ok2_store_i32
L_bounds_ok_store_i32:
               call proto_exit, exit, 14
L_bounds_ok2_store_i32:
               mov mem_addr, g_memory
               add mem_addr, mem_addr, addr
               mov i32:0(mem_addr), val
               ret
               endfunc

mir_store_u32: func i64:addr, i64:val
               local i64:rem, i64:mem_addr
               mod rem, addr, 4
               beq L_align_ok_store_u32, rem, 0
               call proto_exit, exit, 16
L_align_ok_store_u32:
               ubgt L_bounds_ok_store_u32, addr, 1048572
               jmp L_bounds_ok2_store_u32
L_bounds_ok_store_u32:
               call proto_exit, exit, 14
L_bounds_ok2_store_u32:
               mov mem_addr, g_memory
               add mem_addr, mem_addr, addr
               mov u32:0(mem_addr), val
               ret
               endfunc

mir_load_u8: func i64, i64:addr
             local i64:mem_addr, i64:val
             ubgt L_bounds_ok_load_u8, addr, 1048575
             jmp L_bounds_ok2_load_u8
L_bounds_ok_load_u8:
             call proto_exit, exit, 13
L_bounds_ok2_load_u8:
             mov mem_addr, g_memory
             add mem_addr, mem_addr, addr
             mov val, u8:0(mem_addr)
             ret val
             endfunc

mir_store_u8: func i64:addr, i64:val
              local i64:mem_addr
              ubgt L_bounds_ok_store_u8, addr, 1048575
              jmp L_bounds_ok2_store_u8
L_bounds_ok_store_u8:
              call proto_exit, exit, 14
L_bounds_ok2_store_u8:
              mov mem_addr, g_memory
              add mem_addr, mem_addr, addr
              mov u8:0(mem_addr), val
              ret
              endfunc

mir_load_i64: func i64, i64:addr
              local i64:rem, i64:mem_addr, i64:val
              mod rem, addr, 8
              beq L_align_ok_load_i64, rem, 0
              call proto_exit, exit, 15
L_align_ok_load_i64:
              ubgt L_bounds_ok_load_i64, addr, 1048568
              jmp L_bounds_ok2_load_i64
L_bounds_ok_load_i64:
              call proto_exit, exit, 13
L_bounds_ok2_load_i64:
              mov mem_addr, g_memory
              add mem_addr, mem_addr, addr
              mov val, i64:0(mem_addr)
              ret val
              endfunc

mir_store_i64: func i64:addr, i64:val
               local i64:rem, i64:mem_addr
               mod rem, addr, 8
               beq L_align_ok_store_i64, rem, 0
               call proto_exit, exit, 16
L_align_ok_store_i64:
               ubgt L_bounds_ok_store_i64, addr, 1048568
               jmp L_bounds_ok2_store_i64
L_bounds_ok_store_i64:
               call proto_exit, exit, 14
L_bounds_ok2_store_i64:
               mov mem_addr, g_memory
               add mem_addr, mem_addr, addr
               mov i64:0(mem_addr), val
               ret
               endfunc

mir_data_addr: func i64, i64:base, i64:offset, i64:len
               local i64:limit, i64:res
               ubgt L_bounds_ok_data_addr, offset, len
               jmp L_bounds_ok2_data_addr
L_bounds_ok_data_addr:
               call proto_exit, exit, 13
L_bounds_ok2_data_addr:
               mov limit, 4294967295
               sub limit, limit, offset
               ubgt L_overflow_data_addr, base, limit
               jmp L_overflow_ok_data_addr
L_overflow_data_addr:
               call proto_exit, exit, 17
L_overflow_ok_data_addr:
               add res, base, offset
               uext32 res, res
               ret res
               endfunc

"#;
