# Non-Goals

`mirsem` does not implement:

- native code generation;
- optimization;
- RISC-V32 backend behavior;
- host C ABI;
- varargs;
- free/realloc/GC;
- dynamic linking to host C symbols;
- lazy JIT;
- lazy basic-block versioning;
- code replacement;
- deoptimization;
- concurrency;
- live IDE or ECS workspace behavior.

The module is a semantic oracle for MIR-F0, not a production interpreter.

