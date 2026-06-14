# mirc0

`mirc0` is a baseline correctness-oriented compiler that translates validated MIR-F0 `ModuleImage` values to simple, portable C code.

It is used to compare compiled execution against the strict semantic interpreter `mirsem` on the host system.

The stable entry point is `mirc0::compile(&ModuleImage, entry_name)`.
`mirc0::compile_lowered(&mirplan::LoweredProgram, entry_name)` is experimental
F1 plumbing used to prove that the lowered planning contract is sufficient for
backend work. `mirtool compile-c` still uses the stable `ModuleImage` path.
