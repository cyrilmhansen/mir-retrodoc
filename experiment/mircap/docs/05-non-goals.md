# Non-Goals

This module intentionally does not implement:

- upstream MIR compatibility;
- interpreter execution;
- native code generation;
- optimization;
- RISC-V32 backend work;
- ECS or live IDE workspace state;
- runtime counters or trace snapshots;
- code replacement;
- lazy basic-block versioning;
- host C ABI support;
- C2MIR;
- full memory model finalization;
- execution-time memory bounds checking.

## Reason

The first module only defines the immutable loaded module image and the first
validation pass. Runtime behavior belongs in later modules.
