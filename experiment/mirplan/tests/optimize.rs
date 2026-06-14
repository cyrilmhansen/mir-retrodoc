use mircap::{ModuleImage, Opcode};
use mirplan::{build_compile_plan, lower_compile_plan, optimize_program, LoweredOperand};
use mirspace::ProgramSpace;

fn lowered_fixture(name: &str) -> mirplan::LoweredProgram {
    let path = format!(
        "{}/../mircap/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(path).expect("read fixture");
    let image = ModuleImage::from_bytes(&bytes).expect("load fixture");
    let space = ProgramSpace::from_module_image(&image).expect("space");
    let plan = build_compile_plan(&space);
    lower_compile_plan(&plan)
}

#[test]
fn test_constant_folding_and_dce() {
    let lowered = lowered_fixture("valid_arithmetic.mircap.txt");
    let optimized = optimize_program(lowered);

    // Verify optimized program structure
    let main = &optimized.functions[0];
    let instructions = &main.blocks[0].instructions;

    // Original had 4 instructions: const_i32, const_i32, add_i32, ret.
    // Optimized folds constant return value directly into ret, making it: ret i:42.
    assert_eq!(instructions.len(), 1);

    let first_insn = &instructions[0];
    assert_eq!(first_insn.opcode, Opcode::Ret);
    assert_eq!(first_insn.operands.len(), 1);
    assert_eq!(first_insn.operands[0], LoweredOperand::ImmI32(42));
}

#[test]
fn test_branch_folding() {
    let lowered = lowered_fixture("valid_branch.mircap.txt");
    let optimized = optimize_program(lowered);

    let main = &optimized.functions[0];
    let instructions = &main.blocks[0].instructions;

    // The BranchIf was on a constant u32, so it should be folded to Branch unconditional,
    // and the const_u32 should be eliminated.
    assert_eq!(instructions.len(), 1);
    let branch_insn = &instructions[0];
    assert_eq!(branch_insn.opcode, Opcode::Branch);
    assert_eq!(branch_insn.operands.len(), 1);

    // And block successors should only have unconditional target
    assert_eq!(main.blocks[0].successors.len(), 1);
    assert_eq!(
        main.blocks[0].successors[0].kind,
        mirspace::EdgeKind::Unconditional
    );
}
