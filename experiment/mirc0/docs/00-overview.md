# mirc0 Overview

`mirc0` is a baseline correctness-oriented compiler that translates validated MIR-F0 `ModuleImage` values to simple, portable C code.

## Purpose

It is not:
- An optimizing compiler
- A native backend (e.g. RISC-V or x86)
- A JIT compiler

Rather, it is a **baseline reference compiler** used to validate compiler backend concepts by emitting C code and running differential tests against the reference interpreter (`mirsem`) on the host system.
