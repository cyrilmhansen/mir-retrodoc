# Project Scope

This project documents the preserved source archive of Vladimir Makarov's MIR lightweight JIT compiler. The source archive is the authority. The documentation is retrospective: it records what can be observed in the code and existing project documents, then separates those observations from hypotheses, design inferences, and open questions.

The local source archive used by this pass is `mir-preservation/git/mir-restored`. The surrounding preservation bundle also contains archived repository metadata, GitHub data, web material, and slides under `mir-preservation/`.

## Goals

- Preserve a technical map of the MIR codebase before proposing redesign.
- Document how MIR represents modules, items, functions, instructions, operands, labels, types, and data.
- Document how MIR text, binary MIR, and API construction reach in-memory MIR objects.
- Document the interpreter, JIT generator, optimizer, backends, runtime boundaries, and code-cache behavior in later passes.
- Identify facts, hypotheses, limitations, and unanswered questions with enough precision that a future maintainer can verify them in source.
- Evaluate, without prematurely redesigning MIR, whether a smaller MIR-inspired runtime could be extracted for RISC-V32, a fantasy computer, or a constrained environment.

## Non-Goals For This Pass

- No MIR source changes.
- No refactoring.
- No tests.
- No attempt to fully document the JIT pipeline yet.
- No proposal for a RISC-V32 implementation beyond source-map notes and questions.
- No claims about design intent unless directly supported by code comments or project documentation.

## Documentation Method

Facts are statements directly observed in source files, headers, comments, build files, or preserved project documentation. Inferred statements are marked as such. Speculative statements are marked as `Hypothesis`.

Every deeper document should include:

- observed facts with file paths and symbol names;
- limitations and trade-offs;
- open questions;
- relevance to RISC-V32 or fantasy-computer extraction, where applicable.

## Mechanism Classification

When analyzing MIR, especially the JIT and backend code, classify mechanisms with one or more of these labels:

- `essential to MIR semantics`: required to preserve the meaning of MIR modules, items, instructions, operands, types, control flow, calls, or data.
- `required for native host execution`: required to execute generated native machine code on a host OS or CPU.
- `required only for C ABI compatibility`: required to interoperate with C calls, varargs, platform calling conventions, or C2MIR output.
- `backend-specific engineering detail`: target-dependent encoding, register, frame, relocation, or cache-management work.
- `optimization convenience`: useful for compile speed or generated-code quality, but not required for MIR semantics.
- `removable for a fantasy computer subset`: plausibly unnecessary if the subset owns its ABI, runtime, and execution environment.

These labels are provisional until verified in the relevant implementation files.

## Preservation Boundary

The original code remains the source of truth. Any future simplification or reimplementation must be clearly separated from archival documentation and justified by observed architecture. A smaller MIR-inspired subset should not be presented as MIR unless the unsupported semantics, ABI behavior, data formats, and runtime boundaries are explicitly documented.

## Limitations And Trade-Offs

- This pass classifies files and symbols from a first inspection only. It does not yet trace every call path.
- Some categories overlap. For example, `mir.c` contains IR construction, simplification, module loading, linking, binary IO, text scanning, and executable-code helpers.
- The MIR generator is partly organized by including target-specific `.c` files into `mir-gen.c`, so "backend abstraction" is a source-level convention rather than a separate interface object.
- Existing author documentation is used as evidence, but source verification remains required for behavioral claims.

## Relevance To RISC-V32 / Fantasy Computer Extraction

The project should keep two questions separate:

- What does MIR require as an IR and execution model?
- What does current MIR require because it targets real host ABIs, OS executable memory, and C interoperability?

RISC-V64 support exists in the archive (`mir-riscv64.c`, `mir-gen-riscv64.c`, and `c2mir/riscv64/`). This does not imply RISC-V32 support. A RISC-V32 or fantasy-computer subset may be possible, but the hard parts must be identified from code first: pointer width assumptions, C ABI lowering, block argument passing, varargs, code allocation, instruction cache flushing, long double behavior, register allocation constraints, and external-symbol linkage.

## Open Questions

- Which parts of MIR semantics depend on 64-bit integer registers versus target pointer size?
- How much of the interpreter depends on generated target thunks and C ABI shims?
- Which JIT mechanisms compile whole functions, and which support lazy basic-block versioning?
- What is the smallest subset that can preserve MIR textual/API construction while avoiding host C ABI complexity?
- Is RISC-V64 the best reference for RISC-V32, or would a simpler backend be a better architectural model?
