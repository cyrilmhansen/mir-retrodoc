use mircap::ModuleImage;
use mirplan::{build_compile_plan, format_lowered, lower_compile_plan};
use mirspace::ProgramSpace;

fn formatted_lowered_fixture(name: &str) -> String {
    let path = format!(
        "{}/../mircap/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(path).expect("read fixture");
    let image = ModuleImage::from_bytes(&bytes).expect("load fixture");
    let space = ProgramSpace::from_module_image(&image).expect("space");
    let plan = build_compile_plan(&space);
    let lowered = lower_compile_plan(&plan);
    format_lowered(&lowered)
}

#[test]
fn formats_lowered_branch() {
    assert_eq!(
        formatted_lowered_fixture("valid_branch.mircap.txt"),
        r#"lowered module branch
functions:
  fn f0#1 main entry=b0#1
    params: -
    results: i32
    block b0#1
      i0#1 value const_u32 writes=[v0#0:u32] reads=[-]
      i1#2 branch branch_if writes=[-] reads=[v0#0:u32] targets=[true:b1#2, false:b2#3]
      successors: true:b1#2, false:b2#3
    block b1#2
      i2#3 value const_i32 writes=[v1#1:i32] reads=[-]
      i3#4 return ret writes=[-] reads=[v1#1:i32]
    block b2#3
      i4#5 value const_i32 writes=[v1#1:i32] reads=[-]
      i5#6 return ret writes=[-] reads=[v1#1:i32]
"#
    );
}

#[test]
fn formats_lowered_direct_call() {
    assert_eq!(
        formatted_lowered_fixture("valid_direct_call.mircap.txt"),
        r#"lowered module direct_call
functions:
  fn f0#1 main entry=b0#1
    params: -
    results: i32
    block b0#1
      i0#1 value const_i32 writes=[v0#0:i32] reads=[-]
      i1#2 call call writes=[v1#1:i32] reads=[v0#0:i32] callee=f1#2 callee
      i2#3 return ret writes=[-] reads=[v1#1:i32]
  fn f1#2 callee entry=b1#2
    params: v2#0:i32
    results: i32
    block b1#2
      i3#4 value copy writes=[v3#1:i32] reads=[v2#0:i32]
      i4#5 return ret writes=[-] reads=[v3#1:i32]
"#
    );
}

#[test]
fn formats_lowered_memory_loop() {
    let output = formatted_lowered_fixture("valid_memory_loop_sum.mircap.txt");
    assert!(output.contains("lowered module memory_loop_sum"));
    assert!(output.contains("memory alloc writes=[v0#0:addr32] reads=[-] op=alloc"));
    assert!(output.contains("memory store_u32 writes=[-] reads=[v2#2:addr32, v9#9:u32] op=store"));
    assert!(output
        .contains("memory addr_add writes=[v2#2:addr32] reads=[v2#2:addr32, v1#1:u32] op=address"));
    assert!(output.contains("memory load_i32 writes=[v8#8:i32] reads=[v2#2:addr32] op=load"));
}
