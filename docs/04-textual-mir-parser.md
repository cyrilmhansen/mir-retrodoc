# Textual MIR Parser

## Parser Entry Points

The public text parser entry point is `MIR_scan_string(ctx, str)`, declared in `mir.h` when `MIR_NO_SCAN` is not set (`mir.h:617-618`) and implemented in `mir.c:6286-6800`.

The scanner is initialized during `_MIR_init` if scanning is enabled (`mir.c:778-780`). `scan_init` allocates `struct scan_ctx`, creates temporary vectors for errors, function variables, types, instruction operands, label names, a label descriptor hash table, and an instruction-name table populated from `MIR_insn_name` (`mir.c:6802-6819`). `scan_finish` destroys those parser-only structures (`mir.c:6822-6830`).

## Scanner/Token Model

The parser uses a small token model, not an AST. `token_t` stores a token code plus one scalar/string/name payload. Token codes include integer, float, double, long double, name, string, newline, EOF, parentheses, comma, semicolon, and colon (`mir.c:5867-5884`).

`struct scan_ctx` stores parser state: `jmp_buf` for error recovery, an accumulated error buffer, temporary vectors for parsed variables/types/operands, line number, instruction-name table, input string position, current label-name vector, and label descriptor table (`mir.c:5897-5909`).

`scan_token` reads directly from the input string using `get_string_char`/`unget_string_char`, skips spaces and comments, returns newline tokens, scans string literals, names, and signed/unsigned numeric forms, and reports lexical errors through `scan_error` (`mir.c:6110-6194`, with number/string helpers at `mir.c:5921-6108`).

## Module And Item Parsing

`MIR_scan_string` loops over logical instructions/directives separated by newline or semicolon. It first collects leading `name:` labels into `label_names`, then classifies the following name as a directive, data type, or instruction mnemonic (`mir.c:6306-6403`).

Directives are recognized by string comparison: `module`, `endmodule`, `proto`, `func`, `endfunc`, `export`, `import`, `forward`, `bss`, `ref`, `lref`, `expr`, `string`, `local`, and `global` (`mir.c:6331-6387`). Data items are recognized by `str2type`; other names are looked up in `insn_name_tab` (`mir.c:6388-6397`).

After operands are parsed, directive handlers call the normal constructors:

- `module`: `MIR_new_module`, then clear the label descriptor table (`mir.c:6590-6595`).
- `endmodule`: `MIR_finish_module` (`mir.c:6596-6601`).
- `bss`, `ref`, `lref`, `expr`, `string`: `MIR_new_bss`, `MIR_new_ref_data`, `MIR_new_lref_data`, `MIR_new_expr_data`, `MIR_new_string_data` (`mir.c:6602-6665`).
- `proto`: `MIR_new_proto_arr` or `MIR_new_vararg_proto_arr` (`mir.c:6666-6679`).
- `func`: `MIR_new_func_arr` or `MIR_new_vararg_func_arr` (`mir.c:6680-6696`).
- `endfunc`: `MIR_finish_func` (`mir.c:6697-6702`).
- `local` / `global`: `MIR_new_func_reg` or `MIR_new_global_func_reg` (`mir.c:6705-6718`).
- data values: `MIR_new_data` after packing bytes into `temp_data` (`mir.c:6720-6785`).
- ordinary instruction: `MIR_new_insn_arr`, then `MIR_append_insn` if inside a function (`mir.c:6786-6789`).

## Function Parsing

Function and prototype signatures are parsed through temporary memory operands. `read_func_proto` separates leading result types from named arguments, requires arguments to have names, stores result types in `scan_types`, and stores argument `MIR_var_t` values in `scan_vars`; block argument sizes are carried through the temporary memory operand base field (`mir.c:6216-6234`).

A `func` directive requires an open module and no already-open function, then calls either `MIR_new_func_arr` or `MIR_new_vararg_func_arr` depending on `...` (`mir.c:6680-6696`). `endfunc` requires an open function, rejects operands, clears the local parser `func` pointer, and calls `MIR_finish_func` (`mir.c:6697-6702`).

Local and global declarations are only valid inside a function and cannot have labels (`mir.c:6383-6387`). They are parsed as typed temporary memory operands and converted into function registers after the line is complete (`mir.c:6705-6718`).

## Instruction Parsing

If the leading name is not a directive or type, it is looked up in `insn_name_tab`. `MIR_UNSPEC`, `MIR_USE`, and `MIR_PHI` are rejected as not portable/scannable text (`mir.c:6391-6397`).

Labels before an instruction are converted to label instruction objects through `create_label_desc(..., TRUE)` and appended to the current function before the instruction (`mir.c:6398-6400`). Then operands are parsed into `scan_insn_ops`. At line end, the parser creates the instruction with `MIR_new_insn_arr` and appends it to the function (`mir.c:6786-6789`).

This means textual MIR shares the same instruction-construction checks as direct API construction for `MIR_new_insn_arr`, plus additional text-specific syntax checks.

## Operand Parsing

Name operands are context-sensitive. Without a following colon, a name can be an export/import/forward name, a label reference for branch/lref/laddr/switch forms, a function register, or an item reference found in the current module (`mir.c:6420-6447`).

Names followed by `:` are parsed as types for function/prototype args, local/global variables, or memory operands. For memory operands, the parser supports displacement, `(base,index,scale)` addressing, and alias/nonalias suffixes (`mir.c:6450-6554`). Integer, float, double, long-double, and string tokens become corresponding immediate/string operands in `scan_insn_ops` (`mir.c:6557-6576`).

For textual string data, the parser verifies one string operand and calls `MIR_new_string_data` (`mir.c:6657-6665`). For string operands inside instructions, the instruction initially receives a `MIR_OP_STR`; later link-time simplification can turn strings into data items and references (`mir.c:3353-3368`).

## Label/Reference Resolution

Forward label references are supported through `label_desc_tab`. `create_label_desc` looks up a label name; if absent, it creates a `MIR_LABEL` instruction object immediately with `MIR_new_label`, records whether it is a definition, and returns that label pointer. If a later definition is seen, the same label object is marked defined; duplicate definitions are rejected (`mir.c:6200-6214`).

Branch operands and `lref` operands use `MIR_new_label_op(ctx, create_label_desc(..., FALSE))` when referencing labels before or after definition (`mir.c:6430-6439`). Instruction labels use `create_label_desc(..., TRUE)` and are appended to the function at the definition site (`mir.c:6398-6400`).

Symbol references are not forward-created in the same way. A name used as an item reference must already be present in `item_tab_find(ctx, name, module)` unless it is an import/export/forward directive or a label/register case (`mir.c:6440-6446`). Cross-module imports are represented explicitly by `import` items and resolved later during `MIR_link`.

The label descriptor table is cleared at each new module (`mir.c:6594-6595`). `MIR_load_module` later performs additional validation for label-reference data: labels in an `lref` must belong to a function, and if two labels are present they must belong to the same function (`mir.c:1840-1913`).

## Error Handling

Scanner errors go through `scan_error`, which prefixes messages with the current line number, appends the message to `error_msg_buf`, and `longjmp`s to the recovery point in `MIR_scan_string` (`mir.c:5918-5935`). The main parser loop recovers to newline or EOF after a scan error, allowing multiple syntax errors to accumulate (`mir.c:6306-6311`). At the end, if `error_msg_buf` is non-empty, the parser calls the context error function with `MIR_syntax_error` (`mir.c:6798-6799`).

This is text-parser-specific error recovery. Constructor calls such as `MIR_new_func_arr`, `MIR_new_insn_arr`, and `MIR_finish_func` use normal MIR error handling directly and are shared with API construction.

## AST Vs Direct IR Construction

No retained AST was observed. The parser uses tokens plus temporary vectors, then directly calls MIR constructors. Modules, items, functions, registers, labels, instructions, operands, and data are created as the text is read. Forward labels are represented by actual `MIR_LABEL` instruction objects created before their definition, not AST nodes.

Textual MIR is therefore close to the internal IR model: labels are instruction objects, instructions are opcode plus operands, functions are instruction lists, and modules contain item lists. The main differences are parser conveniences such as named labels, directive syntax, temporary signature operands, string literal handling, and immediate syntax.

## Limitations And Trade-Offs

- Text parsing is single-pass over the input string with temporary vectors and a label table. This is compact and direct, but it limits forward symbolic references to labels; item references generally need prior declarations/imports/forwards.
- Parser validation is split: syntax/directive checks are local to `MIR_scan_string`, structural instruction checks are shared through `MIR_new_insn_arr`, and deeper per-function mode/type checks occur in `MIR_finish_func`.
- `MIR_UNSPEC`, `MIR_USE`, and `MIR_PHI` are rejected by textual scanning as non-portable/internal (`mir.c:6395-6397`).
- Error recovery is line-oriented, not grammar-tree based.
- Because construction happens during parsing, a syntax error after earlier successful lines may leave partially created MIR objects in the context until `MIR_finish`.

## Relevance To RISC-V32 / Fantasy Computer Extraction

The textual parser mostly builds target-neutral MIR IR and is `essential to MIR semantics` if textual MIR is preserved. Target/ABI-specific details enter through accepted types, long-double canonicalization, hard-register global variables, block argument syntax, and later simplification/linking.

For a fantasy subset, retaining the direct parser could be useful, but the accepted grammar may need a clearly documented subset. Removing C ABI features would imply rejecting or redefining global hard-register variables, block argument ABI cases, varargs, long double, external C symbols, and possibly label-address/lref features.

For RISC-V32, the parser itself is not the main obstacle. The risk is that parsed constructs such as `p`, block args, hard register names, and long double must match the target runtime and backend.

## Open Questions

- Does binary MIR reading follow the same direct-construction model and label handling?
- Are partially constructed modules after `MIR_scan_string` syntax errors considered recoverable or should callers discard the context?
- Which textual MIR constructs are required by C2MIR output versus handwritten MIR examples?
- Is there a documented grammar beyond the source comment near `MIR_scan_string`?
- Should a fantasy subset preserve textual compatibility by rejecting unsupported directives, or define a distinct syntax?
