@0xb6bb7d6f2399d361;

# Experimental MIR-F0 immutable module image.
# This schema is not upstream MIR and does not claim full MIR compatibility.
# Runtime counters, compiled-code addresses, editor state, and mutable workspace
# data are intentionally excluded.

struct ModuleImage {
  header @0 :Header;
  module @1 :Module;
  types @2 :List(TypeDef);
  symbols @3 :List(Symbol);
  functions @4 :List(Function);
  dataSegments @5 :List(DataSegment);
  blocks @6 :List(Block);
  instructions @7 :List(Instruction);
  operands @8 :List(Operand);
  sourceSpans @9 :List(SourceSpan);
  metadata @10 :List(Metadata);
}

struct Header {
  schemaName @0 :Text;
  formatVersion @1 :UInt32;
  producerName @2 :Text;
  producerVersion @3 :Text;
  sourceLanguage @4 :Text;
  targetAssumptions @5 :Text;
  featureFlags @6 :List(Text);
}

struct Module {
  id @0 :UInt32;
  name @1 :Text;
}

struct TypeDef {
  id @0 :UInt32;
  kind @1 :TypeKind;
}

enum TypeKind {
  void @0;
  i32 @1;
  u32 @2;
  addr32 @3;
  unsupportedI64 @4;
  unsupportedFloat @5;
  unsupportedLongDouble @6;
  unsupportedAggregate @7;
  unsupportedVarargs @8;
  unsupportedHostCAbi @9;
}

struct Symbol {
  id @0 :UInt32;
  name @1 :Text;
  kind @2 :SymbolKind;
}

enum SymbolKind {
  function @0;
  data @1;
  runtimeHelper @2;
}

struct Function {
  id @0 :UInt32;
  symbol @1 :UInt32;
  params @2 :List(UInt32);
  results @3 :List(UInt32);
  valueCount @4 :UInt32;
  valueTypes @5 :List(UInt32);
  firstBlock @6 :UInt32;
  blockCount @7 :UInt32;
  flags @8 :UInt32;
  sourceSpan @9 :UInt32;
}

struct DataSegment {
  symbol @0 :UInt32;
  offset @1 :UInt32;
  bytes @2 :Data;
  zeroFill @3 :UInt32;
}

struct Block {
  id @0 :UInt32;
  parentFunction @1 :UInt32;
  firstInstruction @2 :UInt32;
  instructionCount @3 :UInt32;
  terminator @4 :UInt32;
  sourceSpan @5 :UInt32;
}

struct Instruction {
  id @0 :UInt32;
  opcode @1 :Opcode;
  firstResult @2 :UInt32;
  resultCount @3 :UInt32;
  firstOperand @4 :UInt32;
  operandCount @5 :UInt32;
  sourceSpan @6 :UInt32;
}

enum Opcode {
  constI32 @0;
  constU32 @1;
  copy @2;
  addI32 @3;
  subI32 @4;
  mulI32 @5;
  eqI32 @6;
  neI32 @7;
  ltI32 @8;
  branch @9;
  branchIf @10;
  call @11;
  ret @12;
  trap @13;
  alloc @14;
  loadI32 @15;
  loadU32 @16;
  storeI32 @17;
  storeU32 @18;
  addrAdd @19;
  unsupportedI64 @20;
  unsupportedIndirectCall @21;
}

struct Operand {
  union {
    value @0 :UInt32;
    immI32 @1 :Int32;
    immU32 @2 :UInt32;
    block @3 :UInt32;
    function @4 :UInt32;
    symbol @5 :UInt32;
    type @6 :UInt32;
  }
}

struct SourceSpan {
  id @0 :UInt32;
  file @1 :Text;
  line @2 :UInt32;
  column @3 :UInt32;
}

struct Metadata {
  key @0 :Text;
  value @1 :Text;
}
