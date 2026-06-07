# Execution Model

Execution starts from an explicit `FunctionId` or from a function symbol named
`main`.

The runner:

1. validates the `ModuleImage` through `mircap`;
2. creates the initial frame;
3. enters the function's first block;
4. executes instructions in order;
5. pushes frames for direct calls;
6. pops frames for returns;
7. stops when the entry frame returns or an execution trap occurs.

## Frames

Each frame stores:

- function ID;
- current block ID;
- instruction position;
- value/register storage sized from `value_count`;
- return destinations for a caller's direct call.

Function parameters occupy the leading value slots according to the `mircap`
function signature and value type table.

## Blocks

`branch` enters its target block.

`branch_if` uses the MIR-F0 two-target form: a non-zero `u32` condition enters
the true target, and zero enters the false target.

Block order does not define conditional fallthrough semantics. Function block
lists are useful for validation and traversal, but control flow must be
explicit in branch instructions.

## Fuel

The default fuel limit is 1,000,000 executed instructions. Exhaustion produces
`ExecutionTrap::FuelExhausted`.

The default maximum call depth is 1024 frames. Exceeding it produces
`ExecutionTrap::StackOverflow`.

## Staged Sieve Test Plan

The memory execution tests are intentionally staged before relying on a larger
sieve-style program:

1. `alloc_store_load_u32`: allocate one aligned cell, store `42`, load it, and
   return `42`.
2. `addr_add_two_cells`: allocate two cells, use explicit `addr_add`, load both
   cells, and return their sum.
3. `memory_loop_sum`: allocate eight cells, write values `0..7`, traverse memory
   with `addr_add`, and return `28`.
4. `sieve_32`: allocate 128 bytes, store `u32` prime flags below 32, count the
   flags through a loop, and return `11`.

The current `sieve_32` test initializes flags, marks composite entries for
p=2, p=3, and p=5 below 32, then counts the remaining prime flags. It uses
constant-step marking loops because MIR-F0 still has no general `u32`
arithmetic or index-to-byte-offset conversion.
