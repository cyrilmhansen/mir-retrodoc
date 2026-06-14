use mircap::{BlockId, FunctionId, InstructionId, ModuleImage, Opcode, TypeKind, ValueId};
use mirplan::{build_compile_plan, lower_compile_plan, LoweredInstructionKind, LoweredMemoryOp};
use mirspace::{EdgeKind, ProgramSpace};

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
fn lowers_branch_targets_and_value_reads() {
    let lowered = lowered_fixture("valid_branch.mircap.txt");
    let main = &lowered.functions[0];
    let branch = &main.blocks[0].instructions[1];

    assert_eq!(main.entry.id, BlockId(1));
    assert_eq!(branch.id, InstructionId(2));
    assert_eq!(branch.opcode, Opcode::BranchIf);
    assert_eq!(
        branch
            .reads
            .iter()
            .map(|value| value.id)
            .collect::<Vec<_>>(),
        vec![ValueId(0)]
    );
    assert!(branch.writes.is_empty());

    let LoweredInstructionKind::Branch { targets } = &branch.kind else {
        panic!("expected lowered branch");
    };
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].kind, EdgeKind::TrueBranch);
    assert_eq!(targets[0].block.id, BlockId(2));
    assert_eq!(targets[1].kind, EdgeKind::FalseBranch);
    assert_eq!(targets[1].block.id, BlockId(3));
    assert_eq!(main.blocks[0].successors, *targets);
}

#[test]
fn lowers_direct_calls_with_reads_and_writes() {
    let lowered = lowered_fixture("valid_direct_call.mircap.txt");
    let main = &lowered.functions[0];
    let call = &main.blocks[0].instructions[1];

    assert_eq!(lowered.module_name, "direct_call");
    assert_eq!(lowered.functions.len(), 2);
    assert_eq!(call.id, InstructionId(2));
    assert_eq!(call.opcode, Opcode::Call);
    assert_eq!(
        call.writes
            .iter()
            .map(|value| (value.id, value.type_kind))
            .collect::<Vec<_>>(),
        vec![(ValueId(1), TypeKind::I32)]
    );
    assert_eq!(
        call.reads.iter().map(|value| value.id).collect::<Vec<_>>(),
        vec![ValueId(0)]
    );

    let LoweredInstructionKind::Call { callee } = &call.kind else {
        panic!("expected lowered call");
    };
    assert_eq!(callee.id, FunctionId(2));
    assert_eq!(callee.name, "callee");
}

#[test]
fn lowers_memory_loop_operations() {
    let lowered = lowered_fixture("valid_memory_loop_sum.mircap.txt");
    let main = &lowered.functions[0];

    let memory_ops = main
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.kind {
            LoweredInstructionKind::Memory { op } => {
                Some((instruction.id, instruction.opcode, op.clone()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        memory_ops[0],
        (InstructionId(1), Opcode::Alloc, LoweredMemoryOp::Alloc)
    );
    assert_eq!(
        memory_ops
            .iter()
            .filter(|(_, _, op)| *op == LoweredMemoryOp::Store)
            .count(),
        8
    );
    assert!(memory_ops
        .iter()
        .any(|(_, opcode, op)| *opcode == Opcode::AddrAdd && *op == LoweredMemoryOp::Address));
    assert!(memory_ops
        .iter()
        .any(|(_, opcode, op)| *opcode == Opcode::LoadI32 && *op == LoweredMemoryOp::Load));

    let load = main
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|instruction| instruction.id == InstructionId(36))
        .expect("load instruction");
    assert_eq!(load.writes.len(), 1);
    assert_eq!(load.reads.len(), 1);
    assert_eq!(load.writes[0].type_kind, TypeKind::I32);
    assert_eq!(load.reads[0].type_kind, TypeKind::Addr32);
}

#[test]
fn lowers_data_segment_summaries() {
    let lowered = lowered_fixture("valid_data_segment_load.mircap.txt");

    assert_eq!(lowered.data_segments.len(), 1);
    assert_eq!(lowered.data_segments[0].name, "global0");
    assert_eq!(lowered.data_segments[0].offset, 100);
    assert_eq!(lowered.data_segments[0].length, 4);
}
