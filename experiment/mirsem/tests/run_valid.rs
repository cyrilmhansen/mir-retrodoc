use mircap::ModuleImage;
use mirsem::{trace::TraceOutcome, ExecutionProfile, Runner, Value};

fn run_fixture(text: &str) -> Vec<Value> {
    run_text(text).0
}

fn run_text(text: &str) -> (Vec<Value>, mirsem::trace::TraceSnapshot) {
    let image = ModuleImage::from_text(text).expect("load fixture");
    let mut runner = Runner::new(image, ExecutionProfile::default()).expect("validated runner");
    let result = runner.run_entry_by_name("main", &[]).expect("run main");
    let trace = runner.trace_snapshot();
    (result.values, trace)
}

#[test]
fn runs_const_return() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_const_return.mircap.txt"
    ));
    assert_eq!(values, vec![Value::I32(42)]);
}

#[test]
fn runs_arithmetic() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_arithmetic.mircap.txt"
    ));
    assert_eq!(values, vec![Value::I32(42)]);
}

#[test]
fn runs_arithmetic_u32() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_arithmetic_u32.mircap.txt"
    ));
    assert_eq!(values, vec![Value::U32(1)]);
}

#[test]
fn runs_branch() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_branch.mircap.txt"
    ));
    assert_eq!(values, vec![Value::I32(7)]);
}

#[test]
fn runs_loop() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_loop.mircap.txt"
    ));
    assert_eq!(values, vec![Value::I32(3)]);
}

#[test]
fn runs_direct_call() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_direct_call.mircap.txt"
    ));
    assert_eq!(values, vec![Value::I32(41)]);
}

#[test]
fn runs_alloc_store_load_i32() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_alloc_store_load_i32.mircap.txt"
    ));
    assert_eq!(values, vec![Value::I32(42)]);
}

#[test]
fn runs_alloc_store_load_u32() {
    let (values, trace) = run_text(include_str!(
        "../../mircap/tests/fixtures/valid_alloc_store_load_u32.mircap.txt"
    ));
    assert_eq!(values, vec![Value::U32(42)]);
    assert_eq!(trace.allocation_count, 1);
    assert_eq!(trace.memory_read_count, 1);
    assert_eq!(trace.memory_write_count, 1);
    assert_eq!(trace.return_count, 1);
    assert_eq!(trace.trap_count, 0);
    let main = trace.functions.first().expect("main trace");
    assert_eq!(main.allocations, 1);
    assert_eq!(main.memory_reads, 1);
    assert_eq!(main.memory_writes, 1);
    assert_eq!(main.returns, 1);
    assert_eq!(main.traps, 0);
}

#[test]
fn runs_addr_add_two_cells() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_addr_add_two_cells.mircap.txt"
    ));
    assert_eq!(values, vec![Value::I32(42)]);
}

#[test]
fn runs_memory_loop_sum() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_memory_loop_sum.mircap.txt"
    ));
    assert_eq!(values, vec![Value::I32(28)]);
}

#[test]
fn runs_sieve_32_trace_plan() {
    let (values, trace) = run_text(include_str!(
        "../../mircap/tests/fixtures/valid_sieve_32.mircap.txt"
    ));
    assert_eq!(values, vec![Value::I32(11)]);
    assert!(matches!(trace.outcome, TraceOutcome::Returned(_)));
    assert_eq!(trace.allocation_count, 1);
    assert_eq!(trace.allocated_bytes, 128);
    assert_eq!(trace.maximum_call_depth_reached, 1);
    assert!(trace.executed_instruction_count > 0);
    assert!(trace
        .functions
        .iter()
        .any(|function| !function.blocks.is_empty()));
}

#[test]
fn runs_sieve_32_u32() {
    let (values, trace) = run_text(include_str!(
        "../../mircap/tests/fixtures/valid_sieve_32_u32.mircap.txt"
    ));
    assert_eq!(values, vec![Value::U32(11)]);
    assert!(matches!(trace.outcome, TraceOutcome::Returned(_)));
    assert_eq!(trace.allocation_count, 1);
    assert_eq!(trace.allocated_bytes, 128);
}

#[test]
fn runs_data_segment_load() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_data_segment_load.mircap.txt"
    ));
    assert_eq!(values, vec![Value::U32(43)]);
}

#[test]
fn runs_load_store_u8() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_load_store_u8.mircap.txt"
    ));
    assert_eq!(values, vec![Value::U32(171)]);
}

#[test]
fn runs_float_constants() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_float_constants.mircap.txt"
    ));
    assert_eq!(
        values,
        vec![
            Value::F32(1.5f32.to_bits()),
            Value::F64((-0.25f64).to_bits())
        ]
    );
}

#[test]
fn runs_float_arithmetic() {
    let values = run_fixture(include_str!(
        "../../mircap/tests/fixtures/valid_float_arithmetic.mircap.txt"
    ));
    assert_eq!(
        values,
        vec![
            Value::F32((-16.0f32).to_bits()),
            Value::F64((-16.0f64).to_bits())
        ]
    );
}

#[test]
fn runs_sieve_32_u32_capnp() {
    let original = ModuleImage::from_text(include_str!(
        "../../mircap/tests/fixtures/valid_sieve_32_u32.mircap.txt"
    ))
    .expect("load original");
    let capnp_bytes = original.to_capnp_bytes();
    let decoded = ModuleImage::from_capnp_bytes(&capnp_bytes).expect("decode capnp");

    let mut runner = Runner::new(decoded, ExecutionProfile::default()).expect("validated runner");
    let result = runner.run_entry_by_name("main", &[]).expect("run main");
    assert_eq!(result.values, vec![Value::U32(11)]);
}

#[test]
fn trace_counts_are_separate_from_image() {
    let image = ModuleImage::from_text(include_str!(
        "../../mircap/tests/fixtures/valid_direct_call.mircap.txt"
    ))
    .expect("load fixture");
    let mut runner = Runner::new(image, ExecutionProfile::default()).expect("validated runner");
    let result = runner.run_entry_by_name("main", &[]).expect("run main");
    let trace = runner.trace_snapshot();
    assert_eq!(result.values, vec![Value::I32(41)]);
    assert_eq!(trace.executed_instruction_count, 5);
    assert_eq!(trace.maximum_call_depth_reached, 2);
    assert_eq!(trace.functions.len(), 2);
    assert_eq!(trace.call_edges.len(), 1);
    assert_eq!(trace.call_edges[0].caller.0, 1);
    assert_eq!(trace.call_edges[0].callee.0, 2);
    assert_eq!(trace.call_edges[0].calls, 1);
    assert_eq!(trace.return_count, 2);
    assert_eq!(trace.allocation_count, 0);
    assert_eq!(trace.allocated_bytes, 0);
    assert!(trace
        .functions
        .iter()
        .all(|function| function.executed_instructions > 0));
}
