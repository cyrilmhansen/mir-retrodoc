use mircap::{BlockId, FunctionId, InstructionId, ModuleImage, Opcode, TypeKind};
use mirplan::{build_compile_plan, OperandPlan};
use mirspace::{EdgeKind, ProgramSpace};

fn load_fixture(name: &str) -> ModuleImage {
    let path = format!(
        "{}/../mircap/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(path).expect("read fixture");
    ModuleImage::from_bytes(&bytes).expect("load fixture")
}

fn plan_fixture(name: &str) -> mirplan::CompilePlan {
    let image = load_fixture(name);
    let space = ProgramSpace::from_module_image(&image).expect("space");
    build_compile_plan(&space)
}

#[test]
fn plans_branch_edges_in_function_block_order() {
    let plan = plan_fixture("valid_branch.mircap.txt");

    assert_eq!(plan.module_name, "branch");
    assert_eq!(plan.functions.len(), 1);

    let main = &plan.functions[0];
    assert_eq!(main.id, FunctionId(1));
    assert_eq!(main.name, "main");
    assert_eq!(main.results, vec![TypeKind::I32]);
    assert_eq!(
        main.blocks.iter().map(|block| block.id).collect::<Vec<_>>(),
        vec![BlockId(1), BlockId(2), BlockId(3)]
    );

    let entry = &main.blocks[0];
    assert_eq!(entry.instructions[1].id, InstructionId(2));
    assert_eq!(entry.instructions[1].opcode, Opcode::BranchIf);
    assert_eq!(entry.successors.len(), 2);
    assert_eq!(entry.successors[0].kind, EdgeKind::TrueBranch);
    assert_eq!(entry.successors[0].target_id, BlockId(2));
    assert_eq!(entry.successors[1].kind, EdgeKind::FalseBranch);
    assert_eq!(entry.successors[1].target_id, BlockId(3));
}

#[test]
fn records_direct_call_sites_and_operands() {
    let plan = plan_fixture("valid_direct_call.mircap.txt");

    assert_eq!(plan.functions.len(), 2);
    let main = &plan.functions[0];

    assert_eq!(main.call_sites.len(), 1);
    assert_eq!(main.call_sites[0].instruction_id, InstructionId(2));
    assert_eq!(main.call_sites[0].callee_id, FunctionId(2));
    assert_eq!(main.call_sites[0].callee_name, "callee");

    let call = &main.blocks[0].instructions[1];
    assert_eq!(call.opcode, Opcode::Call);
    assert!(matches!(
        call.operands.as_slice(),
        [OperandPlan::Function { name, .. }, OperandPlan::Value(_)] if name == "callee"
    ));
}

#[test]
fn records_memory_operations_and_data_segments() {
    let plan = plan_fixture("valid_data_segment_load.mircap.txt");
    let main = &plan.functions[0];

    assert_eq!(plan.data_segments.len(), 1);
    assert_eq!(plan.data_segments[0].name, "global0");
    assert_eq!(plan.data_segments[0].offset, 100);
    assert_eq!(plan.data_segments[0].length, 4);

    assert_eq!(
        main.memory_ops
            .iter()
            .map(|op| (op.instruction_id, op.opcode))
            .collect::<Vec<_>>(),
        vec![
            (InstructionId(2), Opcode::DataAddr),
            (InstructionId(3), Opcode::LoadU8),
            (InstructionId(5), Opcode::DataAddr),
            (InstructionId(6), Opcode::LoadU8),
        ]
    );
}
