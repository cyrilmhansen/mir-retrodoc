use mircap::ModuleImage;
use mirplan::{build_compile_plan, lower_compile_plan, summarize_cost};
use mirspace::ProgramSpace;

fn cost_fixture(name: &str) -> mirplan::ProgramCostSummary {
    let path = format!(
        "{}/../mircap/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(path).expect("read fixture");
    let image = ModuleImage::from_bytes(&bytes).expect("load fixture");
    let space = ProgramSpace::from_module_image(&image).expect("space");
    let plan = build_compile_plan(&space);
    let lowered = lower_compile_plan(&plan);
    summarize_cost(&lowered)
}

#[test]
fn summarizes_straight_line_arithmetic_cost() {
    let cost = cost_fixture("valid_arithmetic_u32.mircap.txt");
    assert_eq!(cost.module_name, "arithmetic_u32");
    assert!(cost.bounded);
    assert_eq!(cost.functions.len(), 1);
    assert_eq!(cost.totals.instructions, 6);
    assert_eq!(cost.totals.branches, 0);
    assert_eq!(cost.totals.calls, 0);
    assert_eq!(cost.totals.memory_reads, 0);
    assert_eq!(cost.totals.memory_writes, 0);
    assert_eq!(cost.totals.allocations, 0);
    assert_eq!(cost.totals.traps, 0);
    assert_eq!(cost.functions[0].bound_kind, "acyclic-structural");
}

#[test]
fn summarizes_memory_cost_units() {
    let cost = cost_fixture("valid_alloc_store_load_u32.mircap.txt");
    assert!(cost.bounded);
    assert_eq!(cost.totals.instructions, 5);
    assert_eq!(cost.totals.allocations, 1);
    assert_eq!(cost.totals.memory_reads, 1);
    assert_eq!(cost.totals.memory_writes, 1);
}

#[test]
fn summarizes_branch_cost_units() {
    let cost = cost_fixture("valid_branch.mircap.txt");
    assert!(cost.bounded);
    assert_eq!(cost.totals.instructions, 6);
    assert_eq!(cost.totals.branches, 1);
}

#[test]
fn marks_counted_loops_as_bounded() {
    let cost = cost_fixture("valid_loop.mircap.txt");
    assert_eq!(cost.module_name, "loop");
    assert!(cost.bounded);
    assert_eq!(cost.functions[0].bound_kind, "cyclic-counted-loop");
    assert_eq!(cost.totals.instructions, 19);
    assert_eq!(cost.totals.branches, 8);
}
