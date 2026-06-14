# Non-Goals

The experimental rewrite does not currently attempt:

- full MIR compatibility;
- MIR source parsing;
- C2MIR support;
- host C ABI support;
- full upstream interpreter execution;
- full production compiler implementation;
- production optimizer implementation;
- production RISC-V backend implementation;
- runtime tracing;
- code replacement;
- lazy BBV;
- ECS/live IDE workspace integration.

The first module existed to make the immutable loaded image and validation rules
reviewable before runtime behavior. The workspace now has interpreter, C,
optimization, RV32I, and JIT-demo prototypes, but the production-grade versions
above remain out of scope.
