# Non-Goals

The following features are explicitly out of scope for `mirc0`:

- **Optimization**: The compiler emits extremely simple, unoptimized 1-to-1 C code.
- **Register Allocation**: The compiler relies on C variables for values and lets the C compiler handle register assignment.
- **RISC-V32**: This is a portability baseline, not a native ISA generator.
- **Lazy JIT / Code Replacement**: The compiler only does ahead-of-time (AOT) static C code translation.
- **Host C ABI / External Call Interoperability**: Direct mapping to custom MIR functions only.
- **Cap'n Proto Integration**: Kept deferred.
