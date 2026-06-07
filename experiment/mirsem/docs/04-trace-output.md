# Trace Output

`mirsem` emits a deterministic `TraceSnapshot` after execution.

## Fields

- module ID;
- module name;
- entry function;
- result or trap;
- executed instruction count;
- function call counts;
- block entry counts;
- maximum call depth reached;
- memory profile used;
- allocation count;
- allocated bytes.

The trace is separate from `mircap::ModuleImage`. It is intended for
differential tests against future baseline compiler output.

