# Trace Output

`mirsem` emits a deterministic `TraceSnapshot` after execution.

## Fields

- module ID;
- module name;
- entry function;
- result or trap;
- executed instruction count;
- memory read count;
- memory write count;
- return count;
- trap count;
- function call counts;
- observed caller/callee edge counts;
- per-function instruction, allocation, memory, return, and trap counts;
- block entry counts;
- maximum call depth reached;
- memory profile used;
- allocation count;
- allocated bytes.

The trace is separate from `mircap::ModuleImage`. It is intended for
differential tests against future baseline compiler output.
