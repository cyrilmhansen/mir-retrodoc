# mirspace Overview

`mirspace` represents the internal structured program representation of MIR-F0 modules.

## Roles & Boundary
1. **Source of Truth Boundary**:
   - `mircap::ModuleImage` remains the absolute source of truth for validation and serialization/deserialization.
   - `ProgramSpace` is a derived representation constructed from `ModuleImage` at runtime via `ProgramSpace::from_module_image`.
2. **Analysis-Ready**:
   - Performs parsing and linking of references without changing the execution semantics of the program.
   - Rejects internal inconsistencies (such as missing type declarations or invalid instruction positions) with a structured `SpaceError`.
