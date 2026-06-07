# Non-Goals

The experimental rewrite does not currently attempt:

- full MIR compatibility;
- MIR source parsing;
- C2MIR support;
- host C ABI support;
- interpreter execution;
- compiler implementation;
- optimizer implementation;
- RISC-V32 backend implementation;
- runtime tracing;
- code replacement;
- lazy BBV;
- ECS/live IDE workspace integration.

The first module exists to make the immutable loaded image and validation rules
reviewable before runtime behavior is implemented.

