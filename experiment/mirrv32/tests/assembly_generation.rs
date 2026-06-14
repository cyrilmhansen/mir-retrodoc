use mircap::ModuleImage;
use mirplan::{build_compile_plan, lower_compile_plan, Backend};
use mirrv32::Riscv32Backend;

const CONST_RETURN_FIXTURE: &str =
    include_str!("../../mircap/tests/fixtures/valid_const_return.mircap.txt");
const BRANCH_FIXTURE: &str = include_str!("../../mircap/tests/fixtures/valid_branch.mircap.txt");

#[test]
fn test_assembly_generation_const_return() {
    let image = ModuleImage::from_text(CONST_RETURN_FIXTURE).unwrap();
    let space = mirspace::ProgramSpace::from_module_image(&image).unwrap();
    let plan = build_compile_plan(&space);
    let lowered = lower_compile_plan(&plan);

    let backend = Riscv32Backend;
    let asm = backend.compile(&lowered).unwrap();

    println!("Generated RV32I Assembly:\n{}", asm);

    // Verify metadata directives
    assert!(asm.contains(".attribute arch, \"rv32im\""));
    assert!(asm.contains(".section .text"));

    // Verify function global declaration & symbol
    assert!(asm.contains(".global mir_fn_1"));
    assert!(asm.contains("mir_fn_1:"));

    // Verify stack frame prologue/epilogue
    assert!(asm.contains("addi sp, sp, -16"));
    assert!(asm.contains("sw ra, 12(sp)"));
    assert!(asm.contains("sw s0, 8(sp)"));
    assert!(asm.contains("addi s0, sp, 16"));

    // Verify constant load & callee-saved register use
    assert!(asm.contains("sw s1, -12(s0)"));
    assert!(asm.contains("li s1, 42"));
    assert!(asm.contains("mv a0, s1"));
    assert!(asm.contains("lw s1, -12(s0)"));

    // Verify epilogue & return
    assert!(asm.contains("lw ra, 12(sp)"));
    assert!(asm.contains("lw s0, 8(sp)"));
    assert!(asm.contains("addi sp, sp, 16"));
    assert!(asm.contains("jr ra"));
}

#[test]
fn test_assembly_generation_branch() {
    let image = ModuleImage::from_text(BRANCH_FIXTURE).unwrap();
    let space = mirspace::ProgramSpace::from_module_image(&image).unwrap();
    let plan = build_compile_plan(&space);
    let lowered = lower_compile_plan(&plan);

    let backend = Riscv32Backend;
    let asm = backend.compile(&lowered).unwrap();

    // Verify block labels and branch jumps are emitted correctly
    assert!(asm.contains("block_1_1:"));
    assert!(asm.contains("block_1_2:"));
    assert!(asm.contains("block_1_3:"));
    assert!(asm.contains("bne s1, zero, block_1_2"));
    assert!(asm.contains("j block_1_3"));
}
