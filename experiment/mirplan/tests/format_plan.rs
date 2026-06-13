use mircap::ModuleImage;
use mirplan::{build_compile_plan, format_plan};
use mirspace::ProgramSpace;

fn formatted_fixture(name: &str) -> String {
    let path = format!(
        "{}/../mircap/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(path).expect("read fixture");
    let image = ModuleImage::from_bytes(&bytes).expect("load fixture");
    let space = ProgramSpace::from_module_image(&image).expect("space");
    let plan = build_compile_plan(&space);
    format_plan(&plan)
}

#[test]
fn formats_branch_plan() {
    assert_eq!(
        formatted_fixture("valid_branch.mircap.txt"),
        r#"module branch
functions:
  fn f0#1 main entry=b0
    params: -
    results: i32
    block b0#1
      i0#1 const_u32 -> [v0#0:u32] [u32:1]
      i1#2 branch_if -> [-] [v0#0:u32, b1#2, b2#3]
      successors: true:b1#2, false:b2#3
    block b1#2
      i2#3 const_i32 -> [v1#1:i32] [i32:7]
      i3#4 ret -> [-] [v1#1:i32]
    block b2#3
      i4#5 const_i32 -> [v1#1:i32] [i32:0]
      i5#6 ret -> [-] [v1#1:i32]
"#
    );
}

#[test]
fn formats_direct_call_plan() {
    assert_eq!(
        formatted_fixture("valid_direct_call.mircap.txt"),
        r#"module direct_call
functions:
  fn f0#1 main entry=b0
    params: -
    results: i32
    block b0#1
      i0#1 const_i32 -> [v0#0:i32] [i32:41]
      i1#2 call -> [v1#1:i32] [f1#2:callee, v0#0:i32]
      i2#3 ret -> [-] [v1#1:i32]
    call-sites:
      i1#2 -> f1#2 callee
  fn f1#2 callee entry=b1
    params: v2#0:i32
    results: i32
    block b1#2
      i3#4 copy -> [v3#1:i32] [v2#0:i32]
      i4#5 ret -> [-] [v3#1:i32]
"#
    );
}

#[test]
fn formats_data_segment_memory_plan() {
    assert_eq!(
        formatted_fixture("valid_data_segment_load.mircap.txt"),
        r#"module data_segment_load
data:
  symbol#2 global0 offset=100 length=4
functions:
  fn f0#1 main entry=b0
    params: -
    results: u32
    block b0#1
      i0#1 const_u32 -> [v1#1:u32] [u32:0]
      i1#2 data_addr -> [v0#0:addr32] [s1#2:global0, v1#1:u32]
      i2#3 load_u8 -> [v2#2:u32] [v0#0:addr32]
      i3#4 const_u32 -> [v3#3:u32] [u32:1]
      i4#5 data_addr -> [v0#0:addr32] [s1#2:global0, v3#3:u32]
      i5#6 load_u8 -> [v4#4:u32] [v0#0:addr32]
      i6#7 ret -> [-] [v4#4:u32]
    memory-ops:
      i1#2 data_addr
      i2#3 load_u8
      i4#5 data_addr
      i5#6 load_u8
"#
    );
}
