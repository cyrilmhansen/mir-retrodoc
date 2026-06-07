# The MIR-F0 Pipeline Integration

This document details the compile-time and execution workflows implemented in the MIR-F0 experimental rewrite.

## 1. File Formats and Deserialization
When a file is loaded by `mirtool`, the pipeline detects format based on extension or explicit options.
- **Text Formats (`.mircap.txt`)** are parsed dynamically line-by-line into the hierarchical `ModuleImage` representation in memory.
- **Binary Formats (`.mircap`)** are deserialized via Cap'n Proto. The flat list tables (blocks, instructions, operands, results) are mapped into memory and reconstructed back into hierarchical Rust data structures.

## 2. Evaluation Model (`mirsem`)
The interpreter (`mirsem`) runs on the `ModuleImage`:
- Resolves the function entry point by symbol name.
- Builds an execution stack and linear memory instance configured via `ExecutionProfile`.
- Evaluates instructions step-by-step.
- On normal function return, prints returning values.
- On invalid memory access, stack overflow, or explicit trap instruction, raises an `ExecutionTrap` containing the exact error location and type.

## 3. C Transpilation (`mirc0`)
`mirc0` targets standard, portable C11:
- Emits linear memory heap, bump allocators, stack guard infrastructure, and runtime helper functions.
- Generates C equivalents for all MIR-F0 blocks and instructions.
- Enforces exact execution traps, printing a stable machine-readable trap line to stderr and exiting with the trap's numeric code.

## 4. Differential Verification Workflow
The `diff` command joins both evaluation branches:
1. Reference interpreter runs the bytecode in memory and reports the expected exit code/trap status.
2. The transpiler emits portable C, compiled via:
   `cc -std=c11 -Wall -Wextra -Werror -O0 -o temp_bin temp_src.c`
3. The binary runs on the host system.
4. Out-of-bounds, arithmetic overflow, and return result matching verifies that the transpiled C performs identically to the semantic interpreter oracle.
