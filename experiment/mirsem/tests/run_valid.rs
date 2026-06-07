use mircap::ModuleImage;
use mirsem::{trace::TraceOutcome, ExecutionProfile, Runner, TraceSnapshot, Value};

fn run_fixture(text: &str) -> Vec<Value> {
    run_text(text).0
}

fn run_text(text: &str) -> (Vec<Value>, TraceSnapshot) {
    let image = ModuleImage::from_text(text).expect("load fixture");
    let mut runner = Runner::new(image, ExecutionProfile::default()).expect("validated runner");
    let result = runner.run_entry_by_name("main", &[]).expect("run main");
    let trace = runner.trace_snapshot();
    (result.values, trace)
}

fn ids(values: &[u32]) -> String {
    values.iter().map(u32::to_string).collect::<Vec<_>>().join(" ")
}

#[test]
fn runs_const_return() {
    let values = run_fixture(include_str!("../../mircap/tests/fixtures/valid_const_return.mircap.txt"));
    assert_eq!(values, vec![Value::I32(42)]);
}

#[test]
fn runs_arithmetic() {
    let values = run_fixture(include_str!("../../mircap/tests/fixtures/valid_arithmetic.mircap.txt"));
    assert_eq!(values, vec![Value::I32(42)]);
}

#[test]
fn runs_branch() {
    let values = run_fixture(include_str!("../../mircap/tests/fixtures/valid_branch.mircap.txt"));
    assert_eq!(values, vec![Value::I32(7)]);
}

#[test]
fn runs_direct_call() {
    let values = run_fixture(include_str!("../../mircap/tests/fixtures/valid_direct_call.mircap.txt"));
    assert_eq!(values, vec![Value::I32(41)]);
}

#[test]
fn runs_alloc_store_load_i32() {
    let values = run_fixture(include_str!("../../mircap/tests/fixtures/valid_alloc_store_load_i32.mircap.txt"));
    assert_eq!(values, vec![Value::I32(42)]);
}

#[test]
fn runs_alloc_store_load_u32() {
    let values = run_fixture(include_str!("../../mircap/tests/fixtures/valid_alloc_store_load_u32.mircap.txt"));
    assert_eq!(values, vec![Value::U32(42)]);
}

#[test]
fn runs_addr_add_two_cells() {
    let text = r#"
mircap mircap
version 0
module 1 addr_add_two_cells
type 1 i32
type 2 u32
type 3 addr32
symbol 1 main function
function 1 1 - 1 8 0 3,2,3,2,2,1,1,1
func_block 1 1
block 1 1 1 2 3 4 5 6 7 8 9 10 11
insn 1 alloc r:0 u:8 u:4
insn 2 const_u32 r:1 u:4
insn 3 addr_add r:2 v:0 v:1
insn 4 const_u32 r:3 u:10
insn 5 const_u32 r:4 u:32
insn 6 store_u32 v:0 v:3
insn 7 store_u32 v:2 v:4
insn 8 load_i32 r:5 v:0
insn 9 load_i32 r:6 v:2
insn 10 add_i32 r:7 v:5 v:6
insn 11 ret v:7
"#;
    let values = run_fixture(text);
    assert_eq!(values, vec![Value::I32(42)]);
}

#[test]
fn runs_memory_loop_sum() {
    let mut insns = Vec::new();
    let mut entry = Vec::new();
    let mut id = 1u32;
    let mut push = |block: &mut Vec<u32>, text: String| {
        block.push(id);
        insns.push(format!("insn {id} {text}"));
        id += 1;
    };

    push(&mut entry, "alloc r:0 u:32 u:4".to_string());
    push(&mut entry, "const_u32 r:1 u:4".to_string());
    push(&mut entry, "copy r:2 v:0".to_string());
    for value in 0..8u32 {
        let value_reg = 9 + value;
        push(&mut entry, format!("const_u32 r:{value_reg} u:{value}"));
        push(&mut entry, format!("store_u32 v:2 v:{value_reg}"));
        if value != 7 {
            push(&mut entry, "addr_add r:2 v:2 v:1".to_string());
        }
    }
    push(&mut entry, "const_i32 r:3 i:0".to_string());
    push(&mut entry, "const_i32 r:4 i:8".to_string());
    push(&mut entry, "const_i32 r:5 i:1".to_string());
    push(&mut entry, "const_i32 r:6 i:0".to_string());
    push(&mut entry, "copy r:2 v:0".to_string());
    push(&mut entry, "branch b:2".to_string());

    let mut check = Vec::new();
    push(&mut check, "lt_i32 r:7 v:6 v:4".to_string());
    push(&mut check, "branch_if v:7 b:4 b:3".to_string());

    let mut exit = Vec::new();
    push(&mut exit, "ret v:3".to_string());

    let mut body = Vec::new();
    push(&mut body, "load_i32 r:8 v:2".to_string());
    push(&mut body, "add_i32 r:3 v:3 v:8".to_string());
    push(&mut body, "addr_add r:2 v:2 v:1".to_string());
    push(&mut body, "add_i32 r:6 v:6 v:5".to_string());
    push(&mut body, "branch b:2".to_string());

    let text = format!(
        "mircap mircap\nversion 0\nmodule 1 memory_loop_sum\ntype 1 i32\ntype 2 u32\ntype 3 addr32\nsymbol 1 main function\nfunction 1 1 - 1 17 0 3,2,3,1,1,1,1,2,1,2,2,2,2,2,2,2,2\nfunc_block 1 1\nfunc_block 1 2\nfunc_block 1 3\nfunc_block 1 4\n{}\nblock 1 1 {}\nblock 2 1 {}\nblock 3 1 {}\nblock 4 1 {}\n",
        insns.join("\n"),
        ids(&entry),
        ids(&check),
        ids(&exit),
        ids(&body)
    );

    let values = run_fixture(&text);
    assert_eq!(values, vec![Value::I32(28)]);
}

#[test]
fn runs_sieve_32_trace_plan() {
    let mut insns = Vec::new();
    let mut entry = Vec::new();
    let mut id = 1u32;
    let mut push = |block: &mut Vec<u32>, text: String| {
        block.push(id);
        insns.push(format!("insn {id} {text}"));
        id += 1;
    };

    push(&mut entry, "alloc r:0 u:128 u:4".to_string());
    push(&mut entry, "const_u32 r:1 u:4".to_string());
    push(&mut entry, "copy r:2 v:0".to_string());
    push(&mut entry, "const_i32 r:3 i:0".to_string());
    push(&mut entry, "const_i32 r:4 i:32".to_string());
    push(&mut entry, "const_i32 r:5 i:1".to_string());
    push(&mut entry, "const_i32 r:6 i:0".to_string());
    push(&mut entry, "const_u32 r:9 u:0".to_string());
    push(&mut entry, "const_u32 r:10 u:1".to_string());
    push(&mut entry, "const_u32 r:11 u:16".to_string());
    push(&mut entry, "const_u32 r:12 u:8".to_string());
    push(&mut entry, "const_i32 r:13 i:4".to_string());
    push(&mut entry, "const_i32 r:14 i:2".to_string());
    push(&mut entry, "const_u32 r:15 u:36".to_string());
    push(&mut entry, "const_u32 r:16 u:12".to_string());
    push(&mut entry, "const_i32 r:17 i:9".to_string());
    push(&mut entry, "const_i32 r:18 i:3".to_string());
    push(&mut entry, "const_u32 r:19 u:100".to_string());
    push(&mut entry, "const_u32 r:20 u:20".to_string());
    push(&mut entry, "const_i32 r:21 i:25".to_string());
    push(&mut entry, "const_i32 r:22 i:5".to_string());
    for idx in 0..32 {
        push(&mut entry, "store_u32 v:2 v:10".to_string());
        if idx != 31 {
            push(&mut entry, "addr_add r:2 v:2 v:1".to_string());
        }
    }
    push(&mut entry, "store_u32 v:0 v:9".to_string());
    push(&mut entry, "addr_add r:2 v:0 v:1".to_string());
    push(&mut entry, "store_u32 v:2 v:9".to_string());
    push(&mut entry, "addr_add r:2 v:0 v:11".to_string());
    push(&mut entry, "branch b:5".to_string());

    let mut mark2_check = Vec::new();
    push(&mut mark2_check, "lt_i32 r:7 v:13 v:4".to_string());
    push(&mut mark2_check, "branch_if v:7 b:6 b:7".to_string());

    let mut mark2_body = Vec::new();
    push(&mut mark2_body, "store_u32 v:2 v:9".to_string());
    push(&mut mark2_body, "addr_add r:2 v:2 v:12".to_string());
    push(&mut mark2_body, "add_i32 r:13 v:13 v:14".to_string());
    push(&mut mark2_body, "branch b:5".to_string());

    let mut mark3_setup = Vec::new();
    push(&mut mark3_setup, "addr_add r:2 v:0 v:15".to_string());
    push(&mut mark3_setup, "branch b:8".to_string());

    let mut mark3_check = Vec::new();
    push(&mut mark3_check, "lt_i32 r:7 v:17 v:4".to_string());
    push(&mut mark3_check, "branch_if v:7 b:9 b:10".to_string());

    let mut mark3_body = Vec::new();
    push(&mut mark3_body, "store_u32 v:2 v:9".to_string());
    push(&mut mark3_body, "addr_add r:2 v:2 v:16".to_string());
    push(&mut mark3_body, "add_i32 r:17 v:17 v:18".to_string());
    push(&mut mark3_body, "branch b:8".to_string());

    let mut mark5_setup = Vec::new();
    push(&mut mark5_setup, "addr_add r:2 v:0 v:19".to_string());
    push(&mut mark5_setup, "branch b:11".to_string());

    let mut mark5_check = Vec::new();
    push(&mut mark5_check, "lt_i32 r:7 v:21 v:4".to_string());
    push(&mut mark5_check, "branch_if v:7 b:12 b:13".to_string());

    let mut mark5_body = Vec::new();
    push(&mut mark5_body, "store_u32 v:2 v:9".to_string());
    push(&mut mark5_body, "addr_add r:2 v:2 v:20".to_string());
    push(&mut mark5_body, "add_i32 r:21 v:21 v:22".to_string());
    push(&mut mark5_body, "branch b:11".to_string());

    let mut count_setup = Vec::new();
    push(&mut count_setup, "copy r:2 v:0".to_string());
    push(&mut count_setup, "branch b:2".to_string());

    let mut check = Vec::new();
    push(&mut check, "lt_i32 r:7 v:6 v:4".to_string());
    push(&mut check, "branch_if v:7 b:4 b:3".to_string());

    let mut exit = Vec::new();
    push(&mut exit, "ret v:3".to_string());

    let mut body = Vec::new();
    push(&mut body, "load_i32 r:8 v:2".to_string());
    push(&mut body, "add_i32 r:3 v:3 v:8".to_string());
    push(&mut body, "addr_add r:2 v:2 v:1".to_string());
    push(&mut body, "add_i32 r:6 v:6 v:5".to_string());
    push(&mut body, "branch b:2".to_string());

    let text = format!(
        "mircap mircap\nversion 0\nmodule 1 sieve_32\ntype 1 i32\ntype 2 u32\ntype 3 addr32\nsymbol 1 main function\nfunction 1 1 - 1 23 0 3,2,3,1,1,1,1,2,1,2,2,2,2,1,1,2,2,1,1,2,2,1,1\nfunc_block 1 1\nfunc_block 1 5\nfunc_block 1 6\nfunc_block 1 7\nfunc_block 1 8\nfunc_block 1 9\nfunc_block 1 10\nfunc_block 1 11\nfunc_block 1 12\nfunc_block 1 13\nfunc_block 1 2\nfunc_block 1 3\nfunc_block 1 4\n{}\nblock 1 1 {}\nblock 5 1 {}\nblock 6 1 {}\nblock 7 1 {}\nblock 8 1 {}\nblock 9 1 {}\nblock 10 1 {}\nblock 11 1 {}\nblock 12 1 {}\nblock 13 1 {}\nblock 2 1 {}\nblock 3 1 {}\nblock 4 1 {}\n",
        insns.join("\n"),
        ids(&entry),
        ids(&mark2_check),
        ids(&mark2_body),
        ids(&mark3_setup),
        ids(&mark3_check),
        ids(&mark3_body),
        ids(&mark5_setup),
        ids(&mark5_check),
        ids(&mark5_body),
        ids(&count_setup),
        ids(&check),
        ids(&exit),
        ids(&body)
    );

    let (values, trace) = run_text(&text);
    assert_eq!(values, vec![Value::I32(11)]);
    assert!(matches!(trace.outcome, TraceOutcome::Returned(_)));
    assert_eq!(trace.allocation_count, 1);
    assert_eq!(trace.allocated_bytes, 128);
    assert_eq!(trace.maximum_call_depth_reached, 1);
    assert!(trace.executed_instruction_count > 0);
    assert!(trace.functions.iter().any(|function| !function.blocks.is_empty()));
}

#[test]
fn runs_loop_with_explicit_branch_targets() {
    let text = r#"
mircap mircap
version 0
module 1 loop
type 1 i32
type 2 u32
symbol 1 main function
function 1 1 - 1 4 0 1,1,2,1
func_block 1 1
func_block 1 2
func_block 1 4
func_block 1 3
block 1 1 1 2 3 4
block 2 1 5 6
block 3 1 7 8
block 4 1 9
insn 1 const_i32 r:0 i:0
insn 2 const_i32 r:1 i:3
insn 3 const_i32 r:3 i:1
insn 4 branch b:2
insn 5 lt_i32 r:2 v:0 v:1
insn 6 branch_if v:2 b:3 b:4
insn 7 add_i32 r:0 v:0 v:3
insn 8 branch b:2
insn 9 ret v:0
"#;
    let values = run_fixture(text);
    assert_eq!(values, vec![Value::I32(3)]);
}

#[test]
fn trace_counts_are_separate_from_image() {
    let image = ModuleImage::from_text(include_str!("../../mircap/tests/fixtures/valid_direct_call.mircap.txt")).expect("load fixture");
    let mut runner = Runner::new(image, ExecutionProfile::default()).expect("validated runner");
    let result = runner.run_entry_by_name("main", &[]).expect("run main");
    let trace = runner.trace_snapshot();
    assert_eq!(result.values, vec![Value::I32(41)]);
    assert_eq!(trace.executed_instruction_count, 5);
    assert_eq!(trace.maximum_call_depth_reached, 2);
    assert_eq!(trace.functions.len(), 2);
    assert_eq!(trace.allocation_count, 0);
    assert_eq!(trace.allocated_bytes, 0);
}
