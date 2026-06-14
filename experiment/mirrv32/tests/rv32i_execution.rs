use mircap::ModuleImage;
use mirplan::{build_compile_plan, lower_compile_plan, Backend};
use mirrv32::Riscv32Backend;
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::Command;

const CONST_RETURN_FIXTURE: &str =
    include_str!("../../mircap/tests/fixtures/valid_const_return.mircap.txt");
const BRANCH_FIXTURE: &str = include_str!("../../mircap/tests/fixtures/valid_branch.mircap.txt");
const SIEVE_FIXTURE: &str =
    include_str!("../../mircap/tests/fixtures/valid_sieve_32_u32.mircap.txt");
const ARITHMETIC_FIXTURE: &str =
    include_str!("../../mircap/tests/fixtures/valid_arithmetic.mircap.txt");
const ARITHMETIC_U32_FIXTURE: &str =
    include_str!("../../mircap/tests/fixtures/valid_arithmetic_u32.mircap.txt");
const DIRECT_CALL_FIXTURE: &str =
    include_str!("../../mircap/tests/fixtures/valid_direct_call.mircap.txt");
const TRAP_FIXTURE: &str = include_str!("../../mircap/tests/fixtures/trap_load_oob.mircap.txt");

const I64_FIXTURE: &str = r#"mircap mircap
version 0
module 1 test_i64
type 1 i32
type 2 i64
type 3 addr32
type 4 u32
symbol 1 main function
function 1 1 - 1 16 0 2,2,2,2,2,2,3,1,1,2,1,1,4,1,4,1
func_block 1 1
func_block 1 2
func_block 1 3
func_block 1 4
block 1 1 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15
insn 1 const_i64 r:0 l:100000000000
insn 2 const_i64 r:1 l:200000000000
insn 3 add_i64 r:2 v:0 v:1
insn 4 sub_i64 r:3 v:2 v:0
insn 5 const_i64 r:4 l:2
insn 6 mul_i64 r:5 v:3 v:4
insn 7 const_i32 r:8 i:8
insn 8 const_i32 r:7 i:8
insn 9 alloc r:6 v:8 v:7
insn 10 store_i64 v:6 v:5
insn 11 load_i64 r:9 v:6
insn 12 eq_i64 r:10 v:9 v:5
insn 13 const_i32 r:11 i:1
insn 14 eq_i32 r:12 v:10 v:11
insn 15 branch_if v:12 b:2 b:3
block 2 1 16 17 18
insn 16 lt_i64 r:13 v:3 v:2
insn 17 eq_i32 r:14 v:13 v:11
insn 18 branch_if v:14 b:4 b:3
block 4 1 19 20
insn 19 const_i32 r:15 i:42
insn 20 ret v:15
block 3 1 21 22
insn 21 const_i32 r:15 i:99
insn 22 ret v:15
"#;

fn check_tools() -> bool {
    let gcc_check = Command::new("riscv64-linux-gnu-gcc")
        .arg("--version")
        .output();
    let qemu_check = Command::new("qemu-riscv32").arg("--version").output();
    gcc_check.is_ok() && qemu_check.is_ok()
}

fn execute_rv32i(fixture: &str, test_name: &str, optimize: bool) -> i32 {
    let image = ModuleImage::from_text(fixture).unwrap();
    let space = mirspace::ProgramSpace::from_module_image(&image).unwrap();
    let plan = build_compile_plan(&space);
    let mut lowered = lower_compile_plan(&plan);
    if optimize {
        lowered = mirplan::optimize_program(lowered);
    }

    let backend = Riscv32Backend;
    let generated_asm = backend.compile(&lowered).unwrap();

    // Append our runtime stub and custom mir_alloc
    let mut full_asm = String::new();
    full_asm.push_str(&generated_asm);
    full_asm.push_str(
        r#"
.section .text
.global _start
_start:
    jal ra, mir_fn_1
    # Exit syscall (sys_exit is 93 on RISC-V)
    li a7, 93
    ecall

.global mir_alloc
mir_alloc:
    # a0 = size, a1 = align
    la t0, heap_ptr
    lw t1, 0(t0)          # t1 = current heap_ptr
    
    # Align: mask = a1 - 1
    addi t2, a1, -1       # t2 = mask
    add t1, t1, t2        # t1 = heap_ptr + mask
    not t2, t2            # t2 = ~mask
    and t1, t1, t2        # t1 = aligned heap_ptr
    
    la t3, heap_buffer
    li t4, 1048576        # 1MB size limit
    add t3, t3, t4        # t3 = heap_buffer + 1MB
    
    add t4, t1, a0        # t4 = new heap_ptr
    bgtu t4, t3, .Loom
    
    # Update heap_ptr
    sw t4, 0(t0)
    # Return aligned address in a0
    mv a0, t1
    ret
    
.Loom:
    # Exit with OutOfMemory code 11
    li a0, 11
    li a7, 93
    ecall

.section .data
.align 4
heap_ptr:
    .word heap_buffer

.section .bss
.align 16
heap_buffer:
    .zero 1048576          # 1MB heap buffer
"#,
    );

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = fs::create_dir_all(&dir);
    let s_path = dir.join(format!("temp_exec_{}.s", test_name));
    let bin_path = dir.join(format!("temp_exec_{}", test_name));

    fs::write(&s_path, full_asm).unwrap();

    let mut compile_cmd = Command::new("riscv64-linux-gnu-gcc");
    compile_cmd
        .arg("-mabi=ilp32")
        .arg("-march=rv32im")
        .arg("-static")
        .arg("-nostdlib")
        .arg("-o")
        .arg(&bin_path)
        .arg(&s_path);

    let compile_output = compile_cmd.output().unwrap();
    let _ = fs::remove_file(&s_path);

    if !compile_output.status.success() {
        panic!(
            "Compilation failed: {}",
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }

    let run_output = Command::new("qemu-riscv32")
        .arg(&bin_path)
        .output()
        .unwrap();

    let _ = fs::remove_file(&bin_path);

    if let Some(code) = run_output.status.code() {
        code
    } else if let Some(sig) = run_output.status.signal() {
        128 + sig
    } else {
        255
    }
}

#[test]
fn test_rv32i_const_return() {
    if !check_tools() {
        return;
    }
    let code_unopt = execute_rv32i(CONST_RETURN_FIXTURE, "const_return_unopt", false);
    assert_eq!(code_unopt, 42);

    let code_opt = execute_rv32i(CONST_RETURN_FIXTURE, "const_return_opt", true);
    assert_eq!(code_opt, 42);
}

#[test]
fn test_rv32i_branch() {
    if !check_tools() {
        return;
    }
    let code_unopt = execute_rv32i(BRANCH_FIXTURE, "branch_unopt", false);
    assert_eq!(code_unopt, 7);

    let code_opt = execute_rv32i(BRANCH_FIXTURE, "branch_opt", true);
    assert_eq!(code_opt, 7);
}

#[test]
fn test_rv32i_arithmetic() {
    if !check_tools() {
        return;
    }
    let code_unopt = execute_rv32i(ARITHMETIC_FIXTURE, "arithmetic_unopt", false);
    assert_eq!(code_unopt, 42);

    let code_opt = execute_rv32i(ARITHMETIC_FIXTURE, "arithmetic_opt", true);
    assert_eq!(code_opt, 42);
}

#[test]
fn test_rv32i_arithmetic_u32() {
    if !check_tools() {
        return;
    }
    let code_unopt = execute_rv32i(ARITHMETIC_U32_FIXTURE, "arithmetic_u32_unopt", false);
    assert_eq!(code_unopt, 1);

    let code_opt = execute_rv32i(ARITHMETIC_U32_FIXTURE, "arithmetic_u32_opt", true);
    assert_eq!(code_opt, 1);
}

#[test]
fn test_rv32i_direct_call() {
    if !check_tools() {
        return;
    }
    let code_unopt = execute_rv32i(DIRECT_CALL_FIXTURE, "direct_call_unopt", false);
    assert_eq!(code_unopt, 41);

    let code_opt = execute_rv32i(DIRECT_CALL_FIXTURE, "direct_call_opt", true);
    assert_eq!(code_opt, 41);
}

#[test]
fn test_rv32i_sieve_32_u32() {
    if !check_tools() {
        return;
    }
    let code_unopt = execute_rv32i(SIEVE_FIXTURE, "sieve_32_u32_unopt", false);
    assert_eq!(code_unopt, 11);

    let code_opt = execute_rv32i(SIEVE_FIXTURE, "sieve_32_u32_opt", true);
    assert_eq!(code_opt, 11);
}

#[test]
fn test_rv32i_trap() {
    if !check_tools() {
        return;
    }
    let code_unopt = execute_rv32i(TRAP_FIXTURE, "trap_unopt", false);
    assert_eq!(code_unopt, 139);

    let code_opt = execute_rv32i(TRAP_FIXTURE, "trap_opt", true);
    assert_eq!(code_opt, 139);
}

#[test]
fn test_rv32i_i64() {
    if !check_tools() {
        return;
    }
    let code_unopt = execute_rv32i(I64_FIXTURE, "i64_unopt", false);
    assert_eq!(code_unopt, 42);

    let code_opt = execute_rv32i(I64_FIXTURE, "i64_opt", true);
    assert_eq!(code_opt, 42);
}
