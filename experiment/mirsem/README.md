# mirsem

`mirsem` is a strict MIR-F0 semantic executor / reference evaluator for
`mircap::ModuleImage`.

It is not a production interpreter, compiler, optimizer, JIT, RISC-V32 backend,
or host ABI bridge. Its purpose is to provide an oracle for future baseline
compiler work.

The executor consumes validated MIR-F0 images, runs supported instructions
deterministically, and emits a separate trace snapshot. It does not mutate the
immutable module image.

Memory execution is limited to the current `mircap` opcodes: `alloc`, typed
loads/stores for supported integer and byte widths, `addr_add`, and
`data_addr`. The executor uses a deterministic `LinearMemory` helper and treats
bounds, alignment, and heap/stack collisions as traps.
