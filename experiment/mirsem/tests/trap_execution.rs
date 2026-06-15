use mircap::ModuleImage;
use mirsem::{ExecutionError, ExecutionProfile, ExecutionTrap, Runner};

fn runner_from_text(text: &str, profile: ExecutionProfile) -> Runner {
    let image = ModuleImage::from_text(text).expect("load fixture");
    Runner::new(image, profile).expect("validated runner")
}

#[test]
fn explicit_trap_is_reported() {
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
    let mut runner = runner_from_text(text, ExecutionProfile::default());
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::ExplicitTrap { .. })
    ));
    assert!(matches!(
        runner.trace_snapshot().outcome,
        mirsem::trace::TraceOutcome::Trapped(ExecutionTrap::ExplicitTrap { .. })
    ));
    let trace = runner.trace_snapshot();
    assert_eq!(trace.trap_count, 1);
    assert_eq!(trace.return_count, 0);
    assert_eq!(trace.functions[0].traps, 1);
}

#[test]
fn fuel_exhaustion_is_reported() {
    let text = r#"
mircap mircap
version 0
module 1 infinite_loop
type 1 i32
symbol 1 main function
function 1 1 - 1 1 0 1
func_block 1 1
block 1 1 1
insn 1 branch b:1
"#;
    let profile = ExecutionProfile {
        max_instructions: 8,
        ..ExecutionProfile::default()
    };
    let mut runner = runner_from_text(text, profile);
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("fuel trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::FuelExhausted {
            max_instructions: 8
        })
    ));
}

#[test]
fn stack_overflow_is_reported() {
    let text = r#"
mircap mircap
version 0
module 1 recursive
type 1 i32
symbol 1 main function
function 1 1 - 1 1 0 1
func_block 1 1
block 1 1 1 2
insn 1 call r:0 f:1
insn 2 ret v:0
"#;
    let profile = ExecutionProfile {
        max_call_depth: 4,
        ..ExecutionProfile::default()
    };
    let mut runner = runner_from_text(text, profile);
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("stack trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::StackOverflow { max_depth: 4 })
    ));
}

#[test]
fn heap_stack_collision_is_reported() {
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
    let mut runner = runner_from_text(text, profile);
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("heap/stack trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::HeapStackCollision { .. })
    ));
}

#[test]
fn misaligned_load_is_reported() {
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
    let profile = ExecutionProfile::default();
    let mut runner = runner_from_text(text, profile);
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("misaligned trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::MisalignedLoad { .. })
    ));
}

#[test]
fn misaligned_store_is_reported() {
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
    let mut runner = runner_from_text(text, ExecutionProfile::default());
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("misaligned store trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::MisalignedStore { .. })
    ));
}

#[test]
fn out_of_memory_is_reported() {
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
    let mut runner = runner_from_text(text, ExecutionProfile::default());
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("out of memory trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::OutOfMemory { .. })
    ));
}

#[test]
fn address_overflow_is_reported() {
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
    let mut runner = runner_from_text(text, ExecutionProfile::default());
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("address overflow trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::AddressOverflow { .. })
    ));
}

#[test]
fn data_addr_dynamic_oob_is_reported() {
    let text = include_str!("../../mircap/tests/fixtures/trap_data_addr_dynamic_oob.mircap.txt");
    let mut runner = runner_from_text(text, ExecutionProfile::default());
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("data_addr dynamic oob trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::OutOfBoundsLoad { .. })
    ));
}

#[test]
fn store_oob_is_reported() {
    let text = include_str!("../../mircap/tests/fixtures/trap_store_oob.mircap.txt");
    let mut runner = runner_from_text(text, ExecutionProfile::default());
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("store oob trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::OutOfBoundsStore { .. })
    ));
}

#[test]
fn load_oob_is_reported() {
    let text = include_str!("../../mircap/tests/fixtures/trap_load_oob.mircap.txt");
    let mut runner = runner_from_text(text, ExecutionProfile::default());
    let err = runner
        .run_entry_by_name("main", &[])
        .expect_err("load oob trap expected");
    assert!(matches!(
        err,
        ExecutionError::Trap(ExecutionTrap::OutOfBoundsLoad { .. })
    ));
}
