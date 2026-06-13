# mirspace

`mirspace` is the internal structured program space for the MIR-F0 experimental rewrite.

It imports immutable `mircap::ModuleImage` values and builds dense indexed tables, lookup maps, and control-flow graph (CFG) edges.

## Purpose
- **Derived View**: The `mircap::ModuleImage` is the sole serialization and validation source of truth. `ProgramSpace` is an indexed, analysis-ready derived view constructed at runtime.
- **Fast Traversal**: Translates external stable IDs (`FunctionId`, `BlockId`, etc.) into type-safe dense index newtypes (`FunctionIx`, `BlockIx`, etc.) wrapping `usize`.
- **CFG Analysis**: Performs static control-flow analysis on terminator instructions, explicitly recording edges and predecessor/successor mappings.
