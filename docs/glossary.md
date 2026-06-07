# Glossary

This glossary records terms as used in the MIR source and existing documentation. Some definitions are provisional and should be refined by later source traces.

## ABI

Application Binary Interface. In MIR documentation this mostly matters for calls, returns, argument passing, block values, varargs, hard registers, stack frames, and interoperation with external C functions.

Classification: `required only for C ABI compatibility` when modeling C calls; `required for native host execution` when generated code must be callable by host code.

## AST

Abstract Syntax Tree. Current evidence suggests MIR's durable in-memory form is not an AST. It is a lower-level IR made of modules, items, functions, instruction lists, and operands. The text scanner in `mir.c` uses token/scanner structures, but this pass has not found a retained syntax tree.

Classification: likely not an MIR mechanism; verify in `MIR_scan_string`.

## Basic Block

A control-flow region used by the generator in `mir-gen.c`. Common generator code builds a CFG from MIR instructions and uses basic blocks for optimization, liveness, register allocation, and lazy basic-block versioning.

Classification: `optimization convenience` and `required for native host execution` in the current generator; not necessarily part of user-visible MIR semantics.

## Binary MIR

Compact serialized MIR representation read and written by `MIR_read*` and `MIR_write*` in `mir.c`. Utilities `mir-utils/m2b.c` and `mir-utils/b2m.c` convert between text and binary forms.

Classification: `optimization convenience`; useful for preservation but not required to define MIR semantics if text/API forms are available.

## C2MIR

The C frontend under `c2mir/`. It compiles C into MIR and contains target-specific ABI and predefined-header support.

Classification: not `essential to MIR semantics`; much of it is `required only for C ABI compatibility`.

## Code Cache / Machine Code Context

Executable memory and generated code storage managed through `MIR_code_alloc_t` in `mir-code-alloc.h`, default OS-backed allocation in `mir-code-alloc-default.c`, and helper functions in `mir.c` such as `_MIR_publish_code`, `_MIR_get_new_code_addr`, `_MIR_change_code`, and `_MIR_update_code_arr`.

Classification: `required for native host execution`; OS details may be `removable for a fantasy computer subset`.

## Context

The per-program/per-runtime MIR state represented by `MIR_context_t`. Created by `MIR_init` or `MIR_init2` and destroyed by `MIR_finish`. Existing `MIR.md` says different threads can use MIR functions without synchronization if each thread uses a different context.

Classification: `essential to MIR semantics` for ownership and lifetime.

## External Symbol

A symbol loaded from outside MIR modules, commonly with `MIR_load_external`. Linking resolves imports and sets call interfaces.

Classification: `required only for C ABI compatibility` for host C functions; `required for native host execution` for mixed host/MIR programs.

## Function Item

A `MIR_item_t` whose `item_type` is `MIR_func_item` and whose payload is `MIR_func_t`. A MIR function has result types, arguments, variables, instruction lists, and runtime fields such as `machine_code`, `call_addr`, and `internal`.

Classification: `essential to MIR semantics`.

## Hard Register

A target physical register described by backend code. In generator internals, hard registers are represented in `MIR_OP_VAR` / `MIR_OP_VAR_MEM` space with numbers at or below `MAX_HARD_REG`; pseudo registers are above that range.

Classification: `required for native host execution`; specific numbering is `backend-specific engineering detail`.

## Instruction

A `struct MIR_insn` with an opcode (`MIR_insn_code_t`) and operands (`MIR_op_t ops[]`). Functions store instructions in doubly linked lists.

Classification: `essential to MIR semantics`.

## Interpreter

The non-native execution path in `mir-interp.c`, exposed through `MIR_interp`, `MIR_interp_arr`, `MIR_interp_arr_varg`, and `MIR_set_interp_interface`. It still depends on target support for thunks, C-callable shims, varargs helpers, and foreign-function calls.

Classification: MIR operation execution is `essential to MIR semantics` for non-JIT use; C-callable shims are `required only for C ABI compatibility`.

## Item

A module-level object represented by `struct MIR_item`. Item types include functions, prototypes, imports, exports, forwards, data, reference data, label-reference data, expression data, and bss.

Classification: `essential to MIR semantics`.

## JIT Generator

The native code-generation system exposed by `mir-gen.h` and implemented primarily in `mir-gen.c` plus `mir-gen-<target>.c`. Supports explicit generation and lazy generation interfaces.

Classification: `required for native host execution`; many optimization pieces are `optimization convenience`.

## Label

An instruction pointer-like IR object represented as `MIR_label_t`, typedefed to `struct MIR_insn *`. Label operands use `MIR_OP_LABEL`. Label-reference data can refer to label addresses.

Classification: `essential to MIR semantics`.

## Lazy Generation

A mode installed by `MIR_set_lazy_gen_interface` where a wrapper generates function machine code on first call. `mir-gen.c` also contains `MIR_set_lazy_bb_gen_interface` and basic-block versioning machinery.

Classification: `optimization convenience` and `required for native host execution` in current runtime design.

## Liveness

Generator analysis of which variables are live at program points/basic-block boundaries. Implemented in `mir-gen.c` and used by register allocation, coalescing, dead-code elimination, and pressure tracking.

Classification: `optimization convenience` and `required for native host execution` in the current register allocator.

## Long Double

MIR type `MIR_T_LD`. Existing `MIR.md` says it is machine-dependent and may be double, x86 80-bit, or IEEE quad precision. Data items may be changed to double when long double coincides with double for the target or ABI.

Classification: `essential to MIR semantics` as a type, but exact representation is `backend-specific engineering detail` and `required only for C ABI compatibility` in many cases.

## MIR

Medium Internal Representation. The project provides a strongly typed IR and a lightweight JIT/interpreter system.

## Module

A top-level MIR program container represented by `struct MIR_module`. Modules contain item lists and are created/finished with `MIR_new_module` and `MIR_finish_module`.

Classification: `essential to MIR semantics`.

## Operand

A `MIR_op_t` value attached to an instruction. Public operand modes include register, integer/unsigned integer, float/double/long double, reference, string, memory, and label. Internal modes include `MIR_OP_VAR` and `MIR_OP_VAR_MEM`.

Classification: `essential to MIR semantics` for public modes; internal var modes are `optimization convenience`.

## Optimization Level

Generator setting controlled by `MIR_gen_set_optimize_level`. `mir-gen.c` comments describe `0` as fast generation, `1` as register allocation plus combiner, `2` as adding GVN/constant propagation and default behavior, and `>=3` as enabling everything.

Classification: `optimization convenience`.

## Prototype Item

A module item describing a callable signature, represented by `MIR_proto_t`. Used by call instructions to describe result and argument types.

Classification: `essential to MIR semantics`; ABI interpretation is `required only for C ABI compatibility`.

## RISC-V64 Backend

Existing target support in `mir-riscv64.c`, `mir-riscv64.h`, `mir-gen-riscv64.c`, and `c2mir/riscv64/`. It is a likely reference for RISC-V-family work but is not RISC-V32.

Classification: `backend-specific engineering detail`.

## RISC-V32

A possible future target of interest. No RISC-V32 backend was observed in this pass. RISC-V32 feasibility must account for pointer width, ABI, instruction encoding, code allocation, cache flushing, integer operation widths, and C2MIR target assumptions.

## SSA

Single Static Assignment form built internally by the generator for optimization levels `-O2` and above, according to `mir-gen.c` comments. It introduces internal `MIR_PHI` instructions and SSA edges.

Classification: `optimization convenience`.

## Textual MIR

Assembler-like MIR source representation parsed by `MIR_scan_string` and emitted by `MIR_output*`. It is documented in `MIR.md`.

Classification: `essential to MIR semantics` as a human-readable source format, though API construction can bypass it.

## Thunk

Small target-generated code used to redirect calls or bridge interpreted/generated functions. Target files implement functions such as `_MIR_get_thunk`, `_MIR_redirect_thunk`, `_MIR_get_wrapper`, and `_MIR_get_interp_shim`.

Classification: `required for native host execution`; often `required only for C ABI compatibility`; potentially `removable for a fantasy computer subset`.

## W^X

Write-xor-execute memory protection discipline. MIR's default code allocator exposes `PROT_WRITE_EXEC` and `PROT_READ_EXEC` states and has platform-specific handling in `mir-code-alloc-default.c`.

Classification: `required for native host execution` on protected host OSes; `backend-specific engineering detail`; potentially `removable for a fantasy computer subset`.
