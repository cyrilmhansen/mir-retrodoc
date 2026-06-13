use mircap::{BlockId, FunctionId, InstructionId, ModuleImage, TypeKind, ValueId};
use mirspace::{EdgeKind, OperandRec, ProgramSpace, SpaceError, ValueRole};

fn load_fixture(name: &str) -> ModuleImage {
    let path = format!(
        "{}/../mircap/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(path).expect("read fixture");
    ModuleImage::from_bytes(&bytes).expect("load fixture")
}

#[test]
fn imports_values_symbols_and_data_segments() {
    let image = load_fixture("valid_data_segment_load.mircap.txt");
    let space = ProgramSpace::from_module_image(&image).expect("space");

    assert_eq!(space.name, "data_segment_load");
    assert_eq!(space.functions.len(), 1);
    assert_eq!(space.symbols.len(), 2);
    assert_eq!(space.data_segments.len(), 1);

    let data = &space.data_segments[0];
    assert_eq!(data.offset, 100);
    assert_eq!(data.length, 4);
    assert_eq!(data.bytes, vec![0x2a, 0x2b, 0x2c, 0x2d]);
    assert_eq!(data.zero_fill, 0);

    let function = space.function_by_id(FunctionId(1)).expect("main function");
    assert_eq!(function.params.len(), 0);
    assert_eq!(function.results, vec![TypeKind::U32]);

    let address = space
        .value_by_id(FunctionId(1), ValueId(0))
        .expect("address value");
    assert_eq!(address.type_kind, TypeKind::Addr32);
    assert_eq!(address.role, ValueRole::Local);

    let data_addr = space
        .instruction_by_id(InstructionId(2))
        .expect("data_addr instruction");
    let operands: Vec<_> = space
        .instruction_operands(space.maps.instructions[&InstructionId(2)])
        .iter()
        .map(|operand_ix| space.operands[operand_ix.0])
        .collect();
    assert_eq!(data_addr.results.len(), 1);
    assert!(matches!(
        operands.as_slice(),
        [OperandRec::Symbol(_), OperandRec::Value(_)]
    ));
}

#[test]
fn builds_branch_cfg_edges_and_views() {
    let image = load_fixture("valid_branch.mircap.txt");
    let space = ProgramSpace::from_module_image(&image).expect("space");

    let entry = space.maps.blocks[&BlockId(1)];
    let true_block = space.maps.blocks[&BlockId(2)];
    let false_block = space.maps.blocks[&BlockId(3)];

    assert_eq!(space.edges.len(), 2);
    assert_eq!(space.successors(entry), vec![true_block, false_block]);
    assert_eq!(space.predecessors(true_block), vec![entry]);
    assert_eq!(space.predecessors(false_block), vec![entry]);
    assert_eq!(space.successors(true_block), Vec::new());
    assert_eq!(space.successors(false_block), Vec::new());

    let outgoing = &space.block_by_id(BlockId(1)).expect("entry block").outgoing;
    let first = &space.edges[outgoing[0].0];
    let second = &space.edges[outgoing[1].0];
    assert_eq!(first.kind, EdgeKind::TrueBranch);
    assert_eq!(first.target, true_block);
    assert_eq!(second.kind, EdgeKind::FalseBranch);
    assert_eq!(second.target, false_block);
}

#[test]
fn builds_loop_back_edges() {
    let image = load_fixture("valid_loop.mircap.txt");
    let space = ProgramSpace::from_module_image(&image).expect("space");

    let entry = space.maps.blocks[&BlockId(1)];
    let header = space.maps.blocks[&BlockId(2)];
    let body = space.maps.blocks[&BlockId(3)];
    let exit = space.maps.blocks[&BlockId(4)];

    assert_eq!(space.successors(entry), vec![header]);
    assert_eq!(space.successors(header), vec![body, exit]);
    assert_eq!(space.successors(body), vec![header]);
    assert_eq!(space.predecessors(header), vec![entry, body]);
}

#[test]
fn resolves_direct_call_operands_to_dense_function_indexes() {
    let image = load_fixture("valid_direct_call.mircap.txt");
    let space = ProgramSpace::from_module_image(&image).expect("space");

    let call = space.maps.instructions[&InstructionId(2)];
    let operands: Vec<_> = space
        .instruction_operands(call)
        .iter()
        .map(|operand_ix| space.operands[operand_ix.0])
        .collect();

    let callee = space.maps.functions[&FunctionId(2)];
    let argument = space.maps.values[&(FunctionId(1), ValueId(0))];
    assert_eq!(
        operands,
        vec![OperandRec::Function(callee), OperandRec::Value(argument)]
    );
}

#[test]
fn rejects_invalid_module_images_before_space_construction() {
    let image = load_fixture("invalid_missing_block.mircap.txt");
    let err = ProgramSpace::from_module_image(&image).expect_err("invalid image rejected");

    assert!(matches!(err, SpaceError::Validation(_)));
}

#[test]
fn builds_def_use_index_for_direct_call_values() {
    let image = load_fixture("valid_direct_call.mircap.txt");
    let space = ProgramSpace::from_module_image(&image).expect("space");
    let def_use = space.def_use_index();

    let main_arg = space.maps.values[&(FunctionId(1), ValueId(0))];
    let main_result = space.maps.values[&(FunctionId(1), ValueId(1))];
    let callee_param = space.maps.values[&(FunctionId(2), ValueId(0))];
    let callee_result = space.maps.values[&(FunctionId(2), ValueId(1))];

    assert_eq!(
        def_use.definitions_of(main_arg),
        &[space.maps.instructions[&InstructionId(1)]]
    );
    assert_eq!(
        def_use.uses_of(main_arg),
        &[space.maps.instructions[&InstructionId(2)]]
    );
    assert_eq!(
        def_use.definitions_of(main_result),
        &[space.maps.instructions[&InstructionId(2)]]
    );
    assert_eq!(
        def_use.uses_of(main_result),
        &[space.maps.instructions[&InstructionId(3)]]
    );

    assert_eq!(def_use.definitions_of(callee_param), &[]);
    assert_eq!(
        def_use.uses_of(callee_param),
        &[space.maps.instructions[&InstructionId(4)]]
    );
    assert_eq!(
        def_use.definitions_of(callee_result),
        &[space.maps.instructions[&InstructionId(4)]]
    );
    assert_eq!(
        def_use.uses_of(callee_result),
        &[space.maps.instructions[&InstructionId(5)]]
    );
}

#[test]
fn def_use_index_preserves_multiple_definitions_for_reused_values() {
    let image = load_fixture("valid_loop.mircap.txt");
    let space = ProgramSpace::from_module_image(&image).expect("space");
    let def_use = space.def_use_index();

    let accumulator = space.maps.values[&(FunctionId(1), ValueId(0))];

    assert_eq!(
        def_use.definitions_of(accumulator),
        &[
            space.maps.instructions[&InstructionId(1)],
            space.maps.instructions[&InstructionId(7)],
        ]
    );
    assert_eq!(
        def_use.uses_of(accumulator),
        &[
            space.maps.instructions[&InstructionId(5)],
            space.maps.instructions[&InstructionId(7)],
            space.maps.instructions[&InstructionId(9)],
        ]
    );
}
