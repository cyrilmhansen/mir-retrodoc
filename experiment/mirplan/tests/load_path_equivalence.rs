use mircap::ModuleImage;
use mirplan::{build_compile_plan, format_lowered, format_plan, lower_compile_plan};
use mirspace::ProgramSpace;

fn load_text_fixture(name: &str) -> ModuleImage {
    let path = format!(
        "{}/../mircap/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).expect("read fixture");
    ModuleImage::from_text(&text).expect("load text fixture")
}

fn capnp_roundtrip(image: &ModuleImage) -> ModuleImage {
    let bytes = image.to_capnp_bytes();
    ModuleImage::from_capnp_bytes(&bytes).expect("load capnp bytes")
}

fn plan_image(image: &ModuleImage) -> mirplan::CompilePlan {
    let before = image.clone();
    let space = ProgramSpace::from_module_image(image).expect("space");
    let plan = build_compile_plan(&space);
    assert_eq!(*image, before, "planning must not mutate ModuleImage");
    plan
}

#[test]
fn plan_and_lowering_are_stable_across_text_and_capnp_load_paths() {
    for fixture in [
        "valid_branch.mircap.txt",
        "valid_direct_call.mircap.txt",
        "valid_memory_loop_sum.mircap.txt",
        "valid_data_segment_load.mircap.txt",
    ] {
        let text_image = load_text_fixture(fixture);
        let binary_image = capnp_roundtrip(&text_image);
        assert_eq!(
            text_image, binary_image,
            "Cap'n Proto roundtrip must preserve ModuleImage for {fixture}"
        );

        let text_plan = plan_image(&text_image);
        let binary_plan = plan_image(&binary_image);
        assert_eq!(
            text_plan, binary_plan,
            "CompilePlan must be stable across load paths for {fixture}"
        );
        assert_eq!(
            format_plan(&text_plan),
            format_plan(&binary_plan),
            "formatted CompilePlan must be stable across load paths for {fixture}"
        );

        let text_lowered = lower_compile_plan(&text_plan);
        let binary_lowered = lower_compile_plan(&binary_plan);
        assert_eq!(
            text_lowered.data_segments, text_plan.data_segments,
            "LoweredProgram must preserve data segment summaries for {fixture}"
        );
        assert_eq!(
            text_lowered, binary_lowered,
            "LoweredProgram must be stable across load paths for {fixture}"
        );
        assert_eq!(
            format_lowered(&text_lowered),
            format_lowered(&binary_lowered),
            "formatted LoweredProgram must be stable across load paths for {fixture}"
        );
    }
}
