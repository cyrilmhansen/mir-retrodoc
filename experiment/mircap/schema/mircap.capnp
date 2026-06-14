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
  results @11 :List(UInt32);
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
  i64 @4;
  unsupportedFloat @5;
  unsupportedLongDouble @6;
  unsupportedAggregate @7;
  unsupportedVarargs @8;
  unsupportedHostCAbi @9;
  f32 @10;
  f64 @11;
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
  addU32 @9;
  subU32 @10;
  mulU32 @11;
  eqU32 @12;
  neU32 @13;
  ltU32 @14;
  leU32 @15;
  gtU32 @16;
  geU32 @17;
  branch @18;
  branchIf @19;
  call @20;
  ret @21;
  trap @22;
  alloc @23;
  loadI32 @24;
  loadU32 @25;
  storeI32 @26;
  storeU32 @27;
  loadU8 @28;
  storeU8 @29;
  addrAdd @30;
  dataAddr @31;
  constI64 @32;
  unsupportedIndirectCall @33;
  addI64 @34;
  subI64 @35;
  mulI64 @36;
  eqI64 @37;
  neI64 @38;
  ltI64 @39;
  loadI64 @40;
  storeI64 @41;
  constF32 @42;
  constF64 @43;
  addF32 @44;
  subF32 @45;
  mulF32 @46;
  divF32 @47;
  negF32 @48;
  eqF32 @49;
  neF32 @50;
  ltF32 @51;
  leF32 @52;
  gtF32 @53;
  geF32 @54;
  addF64 @55;
  subF64 @56;
  mulF64 @57;
  divF64 @58;
  negF64 @59;
  eqF64 @60;
  neF64 @61;
  ltF64 @62;
  leF64 @63;
  gtF64 @64;
  geF64 @65;
  i32ToF32 @66;
  f32ToI32 @67;
  i32ToF64 @68;
  f64ToI32 @69;
  f32ToF64 @70;
  f64ToF32 @71;
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
    immI64 @7 :Int64;
    immF32 @8 :Float32;
    immF64 @9 :Float64;
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
