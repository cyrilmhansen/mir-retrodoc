# mircap Module Image

`ModuleImage` is the immutable loaded program image for MIR-F0. It stores stable
IDs and serialized program structure. It does not store runtime state.

## Schema Shape

The schema uses a table/range-oriented model:

- `types`
- `symbols`
- `functions`
- `dataSegments`
- `blocks`
- `instructions`
- `operands`
- optional `sourceSpans`
- optional `metadata`

The Rust skeleton currently stores vectors directly and gives functions and
blocks explicit ID lists. Functions also store a `value_types` table so
validators can type-check operands without executing the module. The schema
keeps the table/range shape through
`firstBlock`, `blockCount`, `firstInstruction`, `instructionCount`,
`firstOperand`, and `operandCount`.

## Nested Model vs Table/Range Model

Nested model: `Module -> Function -> Block -> Instruction`.

- Benefit: simpler to read by hand.
- Cost: harder to build dense indexes across the whole image.
- Cost: less natural for conversion into later ECS/SoA structures.

Table/range model: top-level arrays with stable IDs and ranges.

- Benefit: supports load-time indexing and validation.
- Benefit: keeps stable IDs separate from dense runtime indexes.
- Benefit: easier to attach traces and code metadata by ID without mutating the
  module image.
- Cost: requires stricter validation of ranges and parent relationships.

Recommendation: use the table/range model for the serialized image. Use nested
views only as derived API conveniences.

## Excluded State

- runtime counters;
- trace snapshots;
- compiled-code addresses;
- code-cache ownership;
- editor selections/cursors/transient graph state;
- interpreter stacks;
- compiler temporary state.
- execution memory limits and memory trap state.

## Data Segments

Data segments are immutable module-image declarations:

- data symbol reference;
- linear-memory offset;
- initialized bytes;
- zero-fill length.

They describe initial memory contents. They do not allocate host memory and do
not store runtime pointers.

There is not yet a `data_addr` or `global_addr` instruction. Current execution
tests use heap allocation through `alloc`; direct access to data segments is the
next missing piece for exercising initialized globals.
