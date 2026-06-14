use mircap::ModuleImage;
use mirsem::{ExecutionProfile, ExecutionTrap, RunError, Runner, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
enum ExpectedOutcome {
    Success(Option<Value>),
    Trap(u32),
}

fn map_mirsem_trap(trap: &ExecutionTrap) -> u32 {
    match trap {
        ExecutionTrap::StackOverflow { .. } => 1,
        ExecutionTrap::FuelExhausted { .. } => 2,
        ExecutionTrap::ExplicitTrap { .. } => 3,
        ExecutionTrap::OutOfMemory { .. } => 11,
        ExecutionTrap::HeapStackCollision { .. } => 12,
        ExecutionTrap::OutOfBoundsLoad { .. } => 13,
        ExecutionTrap::OutOfBoundsStore { .. } => 14,
        ExecutionTrap::MisalignedLoad { .. } => 15,
        ExecutionTrap::MisalignedStore { .. } => 16,
        ExecutionTrap::AddressOverflow { .. } => 17,
        _ => 99,
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

fn run_mirsem(image: &ModuleImage, profile: ExecutionProfile) -> ExpectedOutcome {
    let mut runner = Runner::new(image.clone(), profile).expect("runner new");
    match runner.run_entry_by_name("main", &[]) {
        Ok(res) => {
            let val = res.values.first().cloned();
            ExpectedOutcome::Success(val)
        }
        Err(RunError::Trap(trap)) => ExpectedOutcome::Trap(map_mirsem_trap(&trap)),
        Err(e) => {
            panic!("Unexpected mirsem execution error: {:?}", e);
        }
    }
}

fn run_differential(test_name: &str, text: &str, profile: ExecutionProfile) {
    let original = ModuleImage::from_text(text).expect("load image");
    let capnp_bytes = original.to_capnp_bytes();
    let image = ModuleImage::from_capnp_bytes(&capnp_bytes).expect("decode capnp");
    let expected = run_mirsem(&image, profile.clone());

    // Generate C code
    let c_code = mirc0::compile(&image, "main").expect("compile");

    // Check if compiler is available
    let cc_check = Command::new("cc").arg("--version").output();
    if cc_check.is_err() {
        println!("C compiler 'cc' is not available. Skipping compilation and execution check.");
        return;
    }

    // Write to a temporary file in crate dir
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = dir.join(format!("temp_{}.c", test_name));
    let bin_path = dir.join(format!("temp_{}", test_name));

    fs::write(&c_path, c_code).expect("write C source");

    // Compile with cc. Pass memory configuration flags if they differ from defaults
    let mut compile_cmd = Command::new("cc");
    compile_cmd.arg("-O0");
    compile_cmd.arg("-std=c11");
    compile_cmd.arg("-Wall");
    compile_cmd.arg("-Wextra");
    compile_cmd.arg("-Werror");
    if profile.linear_memory_size != 1024 * 1024 {
        compile_cmd.arg(format!("-DMEMORY_SIZE={}", profile.linear_memory_size));
    }
    if profile.stack_size != 64 * 1024 {
        compile_cmd.arg(format!("-DSTACK_SIZE={}", profile.stack_size));
    }

    let compile_output = compile_cmd
        .arg("-o")
        .arg(&bin_path)
        .arg(&c_path)
        .output()
        .expect("run cc");

    if !compile_output.status.success() {
        panic!(
            "C compilation failed for test {}:\nstdout: {}\nstderr: {}",
            test_name,
            String::from_utf8_lossy(&compile_output.stdout),
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }

    // Run the compiled executable
    let output = Command::new(&bin_path).output().expect("run bin");

    // Clean up
    let _ = fs::remove_file(&c_path);
    let _ = fs::remove_file(&bin_path);

    // Compare results
    match expected {
        ExpectedOutcome::Success(expected_val) => {
            assert_eq!(
                output.status.code(),
                Some(0),
                "Expected exit code 0 for normal return in test {}, got status {:?}",
                test_name,
                output.status
            );

            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let result_line = stdout_str.lines().find(|l| l.starts_with("Result: "));
            match expected_val {
                None => {
                    assert_eq!(result_line, Some("Result: void"));
                }
                Some(Value::Void) => {
                    assert_eq!(result_line, Some("Result: void"));
                }
                Some(Value::I32(val)) => {
                    let expected_line = format!("Result: i32 {}", val);
                    assert_eq!(result_line, Some(expected_line.as_str()));
                }
                Some(Value::U32(val)) => {
                    let expected_line = format!("Result: u32 {}", val);
                    assert_eq!(result_line, Some(expected_line.as_str()));
                }
                Some(Value::Addr32(val)) => {
                    let expected_line = format!("Result: addr32 {}", val);
                    assert_eq!(result_line, Some(expected_line.as_str()));
                }
                Some(Value::I64(val)) => {
                    let expected_line = format!("Result: i64 {}", val);
                    assert_eq!(result_line, Some(expected_line.as_str()));
                }
            }
        }
        ExpectedOutcome::Trap(expected_code) => {
            // Must have exited with the trap code
            assert_eq!(
                output.status.code(),
                Some(expected_code as i32),
                "Expected exit status to match trap code {} in test {}, got status {:?}",
                expected_code,
                test_name,
                output.status
            );

            // Stderr must have the trap line
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let trap_line = stderr_str.lines().find(|l| l.starts_with("Trap: "));
            assert!(
                trap_line.is_some(),
                "Expected stderr to contain a 'Trap: ' line, got stderr: {}",
                stderr_str
            );
            let trap_content = trap_line.unwrap();
            let expected_line = format!("Trap: {} {}", expected_code, trap_name(expected_code));
            assert_eq!(
                trap_content,
                expected_line.as_str(),
                "Expected exact trap line '{}', got '{}'",
                expected_line,
                trap_content
            );
        }
    }
}

#[test]
fn diff_const_return() {
    run_differential(
        "const_return",
        include_str!("../../mircap/tests/fixtures/valid_const_return.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_arithmetic() {
    run_differential(
        "arithmetic",
        include_str!("../../mircap/tests/fixtures/valid_arithmetic.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_branch() {
    run_differential(
        "branch",
        include_str!("../../mircap/tests/fixtures/valid_branch.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_loop() {
    run_differential(
        "loop",
        include_str!("../../mircap/tests/fixtures/valid_loop.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_direct_call() {
    run_differential(
        "direct_call",
        include_str!("../../mircap/tests/fixtures/valid_direct_call.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_alloc_store_load_u32() {
    run_differential(
        "alloc_store_load_u32",
        include_str!("../../mircap/tests/fixtures/valid_alloc_store_load_u32.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_addr_add_two_cells() {
    run_differential(
        "addr_add_two_cells",
        include_str!("../../mircap/tests/fixtures/valid_addr_add_two_cells.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_memory_loop_sum() {
    run_differential(
        "memory_loop_sum",
        include_str!("../../mircap/tests/fixtures/valid_memory_loop_sum.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_sieve_32() {
    run_differential(
        "sieve_32",
        include_str!("../../mircap/tests/fixtures/valid_sieve_32.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_arithmetic_u32() {
    run_differential(
        "arithmetic_u32",
        include_str!("../../mircap/tests/fixtures/valid_arithmetic_u32.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_sieve_32_u32() {
    run_differential(
        "sieve_32_u32",
        include_str!("../../mircap/tests/fixtures/valid_sieve_32_u32.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_data_segment_load() {
    run_differential(
        "data_segment_load",
        include_str!("../../mircap/tests/fixtures/valid_data_segment_load.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_trap_explicit() {
    let text = r#"
mircap mircap
version 0
module 1 explicit_trap
type 1 i32
symbol 1 main function
function 1 1 - - 0 0 -
func_block 1 1
block 1 1 1
insn 1 trap
"#;
    run_differential("trap_explicit", text, ExecutionProfile::default());
}

#[test]
fn diff_trap_heap_stack_collision() {
    let text = r#"
mircap mircap
version 0
module 1 heap_stack_collision
type 1 i32
type 2 u32
type 3 addr32
symbol 1 main function
function 1 1 - 3 1 0 3
func_block 1 1
block 1 1 1 2
insn 1 alloc r:0 u:128 u:4
insn 2 ret v:0
"#;
    let profile = ExecutionProfile {
        linear_memory_size: 128,
        stack_size: 64,
        ..ExecutionProfile::default()
    };
    run_differential("trap_heap_stack", text, profile);
}

#[test]
fn diff_trap_misaligned_load() {
    let text = r#"
mircap mircap
version 0
module 1 misaligned_load
type 1 i32
type 2 u32
type 3 addr32
symbol 1 main function
function 1 1 - 1 3 0 3,3,1
func_block 1 1
block 1 1 1 2 3 4
insn 1 alloc r:0 u:1 u:1
insn 2 alloc r:1 u:4 u:1
insn 3 load_i32 r:2 v:1
insn 4 ret v:2
"#;
    run_differential("trap_misaligned_load", text, ExecutionProfile::default());
}

#[test]
fn diff_trap_misaligned_store() {
    let text = r#"
mircap mircap
version 0
module 1 misaligned_store
type 1 i32
type 2 u32
type 3 addr32
symbol 1 main function
function 1 1 - 1 3 0 3,3,1
func_block 1 1
block 1 1 1 2 3 4 5
insn 1 alloc r:0 u:1 u:1
insn 2 alloc r:1 u:4 u:1
insn 3 const_i32 r:2 i:42
insn 4 store_i32 v:1 v:2
insn 5 ret v:2
"#;
    run_differential("trap_misaligned_store", text, ExecutionProfile::default());
}

#[test]
fn diff_trap_out_of_memory() {
    let text = r#"
mircap mircap
version 0
module 1 out_of_memory
type 1 i32
type 2 u32
type 3 addr32
symbol 1 main function
function 1 1 - 3 2 0 3,3
func_block 1 1
block 1 1 1 2 3
insn 1 alloc r:0 u:4 u:4
insn 2 alloc r:1 u:4294967295 u:4
insn 3 ret v:1
"#;
    run_differential("trap_out_of_memory", text, ExecutionProfile::default());
}

#[test]
fn diff_trap_address_overflow() {
    let text = r#"
mircap mircap
version 0
module 1 address_overflow
type 1 i32
type 2 u32
type 3 addr32
symbol 1 main function
function 1 1 - 3 4 0 3,2,2,3
func_block 1 1
block 1 1 1 2 3 4 5 6
insn 1 alloc r:0 u:4 u:4
insn 2 const_u32 r:1 u:4294967295
insn 3 const_u32 r:2 u:1
insn 4 addr_add r:3 v:0 v:1
insn 5 addr_add r:3 v:3 v:2
insn 6 ret v:3
"#;
    run_differential("trap_address_overflow", text, ExecutionProfile::default());
}

#[test]
fn diff_trap_data_addr_dynamic_oob() {
    run_differential(
        "trap_data_addr_dynamic_oob",
        include_str!("../../mircap/tests/fixtures/trap_data_addr_dynamic_oob.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_trap_store_oob() {
    run_differential(
        "trap_store_oob",
        include_str!("../../mircap/tests/fixtures/trap_store_oob.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_trap_load_oob() {
    run_differential(
        "trap_load_oob",
        include_str!("../../mircap/tests/fixtures/trap_load_oob.mircap.txt"),
        ExecutionProfile::default(),
    );
}

#[test]
fn diff_load_store_u8() {
    run_differential(
        "load_store_u8",
        include_str!("../../mircap/tests/fixtures/valid_load_store_u8.mircap.txt"),
        ExecutionProfile::default(),
    );
}
