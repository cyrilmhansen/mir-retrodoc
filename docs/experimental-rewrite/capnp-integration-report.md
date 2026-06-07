# Cap'n Proto Integration Report & Schema/Model Mismatches

This document details the Cap'n Proto serialization integration, verification results, and any structural mismatches between the Cap'n Proto schema (`mircap.capnp`) and the Rust memory model (`ModuleImage`).

## 1. Build Integration
The Rust crates use `capnp = "0.19"` for serialization runtime and `capnpc = "0.19"` inside a custom `build.rs` to compile the schema file `/schema/mircap.capnp` during compilation. The generated Rust accessors are compiled into the `OUT_DIR` directory and included via:
```rust
pub mod mircap_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema/mircap_capnp.rs"));
}
```

## 2. Schema vs. Rust Model Mismatches

### A. Flat Table Layout vs. Nested Hierarchy
- **Rust Memory Model**: Represents the program structure hierarchically via nested `Vec` structures (e.g. `Function` has `Vec<BlockId>`, `Block` has `Vec<InstructionId>`, and `Instruction` contains nested lists of `results` and `operands`).
- **Cap'n Proto Schema**: To optimize deserialization, traversal speed, and layout verification, the schema uses a flat list architecture (e.g. flat `blocks`, `instructions`, `operands`, and `results` lists at the root of `ModuleImage`).
- **Wiring**: The SerDe adapter flattens these collections during serialization and reconstructs the hierarchy using index bounds (`firstBlock`/`blockCount`, `firstInstruction`/`instructionCount`, etc.) during deserialization.

### B. Optional Fields
- **Rust Memory Model**: Employs `Option<T>` for optional values, such as `Option<SourceSpanId>` and optional header fields like `source_language` or `target_assumptions`.
- **Cap'n Proto Schema**: Cap'n Proto lacks native representation for `Option`.
  - Optional IDs (like `SourceSpanId`) are encoded as `0` inside Cap'n Proto. `0` is deserialized back into `None`.
  - Optional strings are encoded as empty strings (`""`) inside Cap'n Proto. `""` is deserialized back into `None`.

### C. Unmapped Collections (`source_spans` & `metadata`)
- **Rust Memory Model**: `ModuleImage` defines `source_spans: Vec<SourceSpan>` and `metadata: Vec<Metadata>`.
- **Cap'n Proto Schema**: The schema contains `sourceSpans @9 :List(SourceSpan)` and `metadata @10 :List(Metadata)`.
- **Mismatch**: The current serialization/deserialization code initializes these to empty collections and does not yet wire their contents.

### D. Anonymous Unions
- **Rust Memory Model**: `Operand` is a tagged enum containing different data variants.
- **Cap'n Proto Schema**: `Operand` wraps an anonymous union. Because the union is anonymous in Cap'n Proto, the enum field setters and getters are placed directly on the struct builder/reader instead of nesting inside a union-specific accessor.

---

## 3. Verification & Status Report

### A. Roundtrip Status
- **Success**: Verified roundtrip path:
  `text fixture` -> `ModuleImage` -> `capnp bytes` -> `ModuleImage`
- Logically identical structure is successfully asserted (`assert_eq!(original, decoded)`).

### B. Validation Status
- **Success**: Deserialized `ModuleImage` is successfully validated against the `mircap` static validator and produces identical validation reports.

### C. interpreter/Transpiler Status
- **mirsem (Interpreter)**: Runs sieve and execution tests on decoded images with identical results.
- **mirc0 (Transpiler)**: Compiles decoded images to portable C11 binaries, executing identically under host compilation (`cc -std=c11 -Wall -Wextra -Werror -O0`).

### D. Usability as Immutable Bytecode Format
- Cap'n Proto is **fully usable** as the immutable bytecode format. The flat layout enables loading and indexing without pointer chasing or heavy initial allocations.
