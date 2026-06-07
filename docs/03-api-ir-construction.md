# API IR Construction

## Minimal API Construction Example

A minimal direct-construction sequence is:

```c
MIR_context_t ctx = MIR_init ();
MIR_module_t m = MIR_new_module (ctx, "m");
MIR_type_t res[] = {MIR_T_I64};
MIR_item_t f = MIR_new_func_arr (ctx, "answer", 1, res, 0, NULL);
MIR_append_insn (ctx, f,
  MIR_new_ret_insn (ctx, 1, MIR_new_int_op (ctx, 42)));
MIR_finish_func (ctx);
MIR_finish_module (ctx);
MIR_load_module (ctx, m);
MIR_link (ctx, MIR_set_interp_interface /* or MIR_set_gen_interface */, NULL);
MIR_finish (ctx);
```

The construction calls create the durable MIR IR directly: module item lists, function instruction lists, and value operands. No retained AST-like structure is involved in this API path.

## Function Construction

`MIR_new_func` and `MIR_new_vararg_func` collect varargs into a temporary `MIR_var_t` array and delegate to `new_func_arr`; `MIR_new_func_arr` and `MIR_new_vararg_func_arr` delegate to `new_func_arr` with `vararg_p` false/true (`mir.c:1436-1459`).

`new_func_arr` rejects nested functions, rejects vararg functions with no mandatory argument, checks result types with `wrong_type_p`, allocates `struct MIR_item` and `struct MIR_func`, stores canonical result types, adds the function item to the current module, initializes instruction lists, creates the function variable vector, initializes register metadata, and converts each argument into a function register numbered from 1 (`mir.c:1365-1433`). Integer-like argument variables are stored as `MIR_T_I64`; floating types keep their float/double/long-double type (`mir.c:1418-1423`).

Local variables are created with `MIR_new_func_reg`; global hard-register-bound variables use `MIR_new_global_func_reg`. Both pass through `new_func_reg`, which accepts only `MIR_T_I64`, `MIR_T_F`, `MIR_T_D`, or `MIR_T_LD`, assigns the next register number, records the name in the function register table, and appends the variable to either `func->vars` or `func->global_vars` (`mir.c:1467-1509`).

Function state is represented by `struct MIR_func`: name, item back-pointer, original/current instruction lists, result and argument counts, result types, vararg/expr/jret flags, variable vectors, generated-code addresses, internal register table, and first label-reference data (`mir.h:300-316`).

## Instruction Construction

Instructions are `struct MIR_insn`: auxiliary `data`, list link, `MIR_insn_code_t code`, operand count, and inline flexible operand array (`mir.h:281-286`). `create_insn` allocates the instruction from the context allocator, canonicalizes long-double opcodes to double opcodes on targets where long double is double, stores the opcode, and clears `data` (`mir.c:2166-2205`).

`MIR_new_insn_arr` is the general constructor. It immediately checks fixed-arity instructions, minimum operand counts for `MIR_SWITCH` and `MIR_PHI`, prototype shape for call/unspec instructions, block argument/result consistency for calls, selected `va_arg` and property-instruction operand constraints, allocates the instruction, stores `nops`, and copies operands by value (`mir.c:2212-2299`).

`MIR_new_insn` is only for fixed-arity ordinary instructions. It rejects `MIR_USE`, `MIR_PHI`, calls, `MIR_UNSPEC`, `MIR_RET`, and `MIR_SWITCH`, directing callers to the array or specialized constructors (`mir.c:2310-2324`). `MIR_new_call_insn`, `MIR_new_jcall_insn`, and `MIR_new_ret_insn` collect varargs and call the shared constructor with the corresponding opcode (`mir.c:2326-2345`).

Instructions are attached to a function with list operations. `MIR_append_insn`, `MIR_prepend_insn`, `MIR_insert_insn_after`, and `MIR_insert_insn_before` only check that the item is a function item and then manipulate `func->insns` (`mir.c:2683-2712`). `MIR_remove_insn` removes the instruction from the active instruction list and frees the instruction memory (`mir.c:793-803`).

## Operand Construction

Public operands are plain `MIR_op_t` values. Constructors initialize the operand mode and store the payload:

- `MIR_new_reg_op`: register number (`mir.c:2458-2463`).
- `MIR_new_int_op` / `MIR_new_uint_op`: 64-bit signed/unsigned immediates (`mir.c:2474-2488`).
- `MIR_new_float_op`, `MIR_new_double_op`, `MIR_new_ldouble_op`: FP immediates, with long double folded to double on Windows or when `__SIZEOF_LONG_DOUBLE__ == 8` (`mir.c:2490-2518`).
- `MIR_new_ref_op`: item reference (`mir.c:2520-2526`).
- `MIR_new_str_op`: interned string (`mir.c:2528-2533`).
- `MIR_new_mem_op` / `MIR_new_alias_mem_op`: memory operand with canonical type, displacement, base/index registers, scale, alias, nonalias, and `nloc = 0` (`mir.c:2536-2561`).
- `MIR_new_label_op`: label instruction pointer (`mir.c:2592-2598`).

Internal operands `MIR_OP_VAR` and `MIR_OP_VAR_MEM` are built by `_MIR_new_var_op` and `_MIR_new_var_mem_op` and are marked internal in the implementation (`mir.c:2466-2472`, `mir.c:2564-2590`). Public API construction should use `MIR_OP_REG` and `MIR_OP_MEM`.

## Labels And Control Flow

Labels are instruction objects. `MIR_label_t` is typedefed to `struct MIR_insn *` (`mir.h:232`, already summarized in `mir.h`). `create_label` allocates a `MIR_LABEL` instruction, stores a unique integer label number in `ops[0]`, then sets `nops = 0`; `MIR_new_label` increments `curr_label_num` and returns that instruction (`mir.c:2385-2390`).

Because labels are real instruction nodes in the function instruction list, control-flow operands point to label instruction objects through `MIR_OP_LABEL`. Later code duplicates labels by temporarily storing the copied label in the original label's `data` field and then redirects branch operands to the copied labels (`mir.c:2730-2780`).

## Calls And Prototypes

Prototypes are module items. `create_proto` allocates `struct MIR_proto`, interns the name, stores result types inline after the struct, creates an argument vector, interns argument names, and records `vararg_p` (`mir.c:1293-1312`). `MIR_new_proto_arr` rejects creation outside a module, validates result types, creates a `MIR_proto_item`, and adds it to the module (`mir.c:1315-1338`).

Calls connect to prototypes by operand 0. `MIR_new_insn_arr` requires call operand 0 to be a `MIR_OP_REF` to a `MIR_proto_item`; operand 1 is the callable value/address; result operands start at operand 2 and are followed by call arguments. It checks operand count against prototype result and argument counts, with extra operands only when the prototype is vararg (`mir.c:2228-2248`). Block argument memory operands are checked against prototype block parameter type and size (`mir.c:2249-2279`).

`MIR_finish_func` later skips the already-checked prototype operand and, if operand 1 is a direct reference, accepts imports, exports, forwards, or function items as callable references (`mir.c:1624-1639`).

## Finalization With `MIR_finish_func`

`MIR_finish_func` closes the currently open function. It requires `curr_func != NULL`, scans every instruction, rejects public use of internal `MIR_PHI` and `MIR_USE`, checks vararg restrictions for `MIR_VA_START`, rejects invalid `JRET` result usage and mixed `RET`/`JRET`, verifies return operand count, validates overflow-branch placement, then checks each operand against the expected mode and output/input role (`mir.c:1556-1741`).

For register operands, it resolves the register number in the function register table and derives the value mode from the declared register type (`mir.c:1655-1660`). For memory operands, it validates memory type, block displacement, and that base/index registers have integer mode (`mir.c:1661-1700`). For references and strings, it treats them as integer address values for mode checking (`mir.c:1708-1712`). It writes `op.value_mode` for later stages (`mir.c:1714-1719`).

If the function has no `RET` or `JRET` and does not end in an unconditional `JMP`, `MIR_finish_func` appends an implicit `MIR_RET` with zero-valued results (`mir.c:1743-1760`). Finally, it records `expr_p` and `jret_p` and clears `curr_func` (`mir.c:1762-1764`).

## Mutations And Lowering

`MIR_finish_func` is not purely validation: it can append an implicit return, set `value_mode` on operands, and set function flags (`mir.c:1714-1719`, `mir.c:1743-1764`). It does not perform the heavier simplifications.

The heavier lowering happens later during `MIR_link` through `simplify_func`. Observed transformations include converting string operands and floating constants into module data items and references, inserting moves, rewriting hard-register uses through temporary registers, decomposing memory addresses into register computations, normalizing multiple returns into one return with moves and a jump, removing unused labels, and rewriting/expanding inline calls (`mir.c:3325-3435`, `mir.c:3542-3619`, `mir.c:3922-4237`). `MIR_link` calls `simplify_func(ctx, item, TRUE)` for each function before interface installation (`mir.c:1969-1983`).

The JIT generator further duplicates and mutates a copy of the function instruction list. `generate_func_code` calls `_MIR_duplicate_func_insns`, builds CFG/SSA/target lowering/register allocation/code selection, publishes code, redirects the thunk, restores the original instruction list, and records `machine_code` (`mir-gen.c:9302-9501`). `_MIR_duplicate_func_insns` moves current instructions into `original_insns`, creates copied instructions for transformation, and later `_MIR_restore_func_insns` removes transformed copies and restores `original_insns` (`mir.c:2753-2812`).

## Limitations And Trade-Offs

- Direct API construction gives callers low-level control but requires them to create valid registers, prototypes, labels, and operand shapes.
- Some checks are immediate, but full operand mode/type validation is delayed until `MIR_finish_func`.
- `MIR_finish_func` appends a default return instead of requiring explicit returns in every fallthrough case.
- Text/API IR is not the final form used by native code generation; link-time simplification and generator-time transformations can substantially rewrite copied instructions.
- Labels being instruction objects simplifies branch target representation but means label identity is tied to instruction-list nodes.

## Relevance To RISC-V32 / Fantasy Computer Extraction

The API-level IR model is mostly `essential to MIR semantics`: modules, items, functions, registers, instructions, operands, labels, and prototypes. Direct C ABI details enter through call prototypes, block argument cases, global hard-register variables, and later target lowering. A fantasy subset could keep the construction model while limiting block arguments, varargs, hard-register locals, multi-result returns, long double, or external C calls.

For RISC-V32, immediate issues to verify are pointer type size (`MIR_T_P`), integer register width conventions, 32-bit instruction variants, block argument ABI lowering, long-double behavior, and whether call prototype semantics can be preserved while using a 32-bit target ABI.

## Open Questions

- Which `simplify_func` transformations are required for interpreter correctness versus JIT generation?
- Can users safely modify a finished function and call `MIR_finish_func` again, or are edits expected after finish without re-finalization?
- Are implicit returns intentional semantics or a convenience for generated MIR?
- What exact invariants does the generator require after `MIR_link` simplification?
- How should a preservation spec describe internal `MIR_OP_VAR` without exposing it as public IR?
