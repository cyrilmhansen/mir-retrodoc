use crate::error::CompileError;
use mircap::ModuleImage;
use mirplan::{build_compile_plan, lower_compile_plan};
use mirspace::ProgramSpace;

pub fn compile(image: &ModuleImage, entry_name: &str) -> Result<String, CompileError> {
    // 1. Validate the ModuleImage first
    image.validate().map_err(CompileError::Validation)?;

    // 2. Build ProgramSpace
    let space = ProgramSpace::from_module_image(image)
        .map_err(|err| CompileError::PlanningFailed(err.to_string()))?;

    // 3. Build Compile Plan
    let plan = build_compile_plan(&space);

    // 4. Lower Compile Plan
    let lowered = lower_compile_plan(&plan);

    // 5. Transpile using compile_lowered
    crate::compile_lowered::compile_lowered(&lowered, entry_name)
}
