use crate::error::SpaceError;
use crate::ids::{BlockIx, FunctionIx, InstructionIx, OperandIx, SymbolIx, ValueIx};
use crate::space::{
    BlockRec, DataSegmentRec, FunctionRec, IdMaps, InstructionRec, OperandRec, ProgramSpace,
    SymbolRec, ValueRec, ValueRole,
};
use mircap::ids::ValueId;
use mircap::image::ModuleImage;
use std::collections::HashMap;

impl ProgramSpace {
    pub fn from_module_image(image: &ModuleImage) -> Result<Self, SpaceError> {
        // 1. Validate the ModuleImage first
        image.validate().map_err(SpaceError::Validation)?;

        let mut space = ProgramSpace {
            name: image.module.name.clone(),
            functions: Vec::new(),
            blocks: Vec::new(),
            instructions: Vec::new(),
            operands: Vec::new(),
            values: Vec::new(),
            edges: Vec::new(),
            data_segments: Vec::new(),
            symbols: Vec::new(),
            maps: IdMaps::default(),
        };

        // Pass 1: Import Symbols
        for (idx, sym) in image.symbols.iter().enumerate() {
            let sym_ix = SymbolIx(idx);
            space.symbols.push(SymbolRec {
                id: sym.id,
                name: sym.name.clone(),
                kind: sym.kind,
            });
            space.maps.symbols.insert(sym.id, sym_ix);
        }

        // Pass 2: Import Data Segments
        for ds in &image.data_segments {
            let length = ds.bytes.len() as u32 + ds.zero_fill;
            space.data_segments.push(DataSegmentRec {
                symbol: ds.symbol,
                offset: ds.offset,
                length,
                bytes: ds.bytes.clone(),
                zero_fill: ds.zero_fill,
            });
        }

        // Pass 3: Pre-allocate Functions and their scoped Values
        for (idx, func) in image.functions.iter().enumerate() {
            let func_ix = FunctionIx(idx);
            space.maps.functions.insert(func.id, func_ix);

            // Populate all values defined in this function
            let param_count = func.params.len() as u32;
            for v_idx in 0..func.value_count {
                let val_id = ValueId(v_idx);
                let type_id = *func.value_types.get(v_idx as usize).ok_or_else(|| {
                    SpaceError::Inconsistency(format!(
                        "Value {} in function {} has no type mapping",
                        v_idx, func.id.0
                    ))
                })?;
                let type_kind = image.type_kind(type_id).ok_or_else(|| {
                    SpaceError::Inconsistency(format!(
                        "Type ID {} in function {} is unresolved",
                        type_id.0, func.id.0
                    ))
                })?;

                let role = if v_idx < param_count {
                    ValueRole::Parameter
                } else {
                    ValueRole::Local
                };

                let val_ix = ValueIx(space.values.len());
                space.values.push(ValueRec {
                    id: val_id,
                    parent: func_ix,
                    type_kind,
                    role,
                });
                space.maps.values.insert((func.id, val_id), val_ix);
            }
        }

        // Pass 4: Pre-allocate Blocks
        for (idx, block) in image.blocks.iter().enumerate() {
            let block_ix = BlockIx(idx);
            space.maps.blocks.insert(block.id, block_ix);
        }

        // Pass 5: Pre-allocate Instructions
        for (idx, insn) in image.instructions.iter().enumerate() {
            let insn_ix = InstructionIx(idx);
            space.maps.instructions.insert(insn.id, insn_ix);
        }

        // Parent mappings helper: Map instruction to parent BlockIx
        let mut insn_parents = HashMap::new();
        for (b_idx, block) in image.blocks.iter().enumerate() {
            let block_ix = BlockIx(b_idx);
            for &insn_id in &block.instructions {
                insn_parents.insert(insn_id, block_ix);
            }
        }

        // Pass 6: Populate Function Records
        for func in &image.functions {
            let mut func_blocks = Vec::new();
            for &block_id in &func.blocks {
                let block_ix = space.maps.blocks.get(&block_id).ok_or_else(|| {
                    SpaceError::Inconsistency(format!(
                        "Function {} references missing block {}",
                        func.id.0, block_id.0
                    ))
                })?;
                func_blocks.push(*block_ix);
            }

            if func_blocks.is_empty() {
                return Err(SpaceError::Inconsistency(format!(
                    "Validated function {} has no blocks",
                    func.id.0
                )));
            }

            // Entry block rule: Use the first block in the block list
            let entry = func_blocks[0];

            let mut params = Vec::new();
            for i in 0..func.params.len() {
                let val_id = ValueId(i as u32);
                let val_ix = space.maps.values[&(func.id, val_id)];
                params.push(val_ix);
            }

            let mut results = Vec::new();
            for &ty_id in &func.results {
                let type_kind = image.type_kind(ty_id).ok_or_else(|| {
                    SpaceError::Inconsistency(format!("Unresolved return type {}", ty_id.0))
                })?;
                results.push(type_kind);
            }

            space.functions.push(FunctionRec {
                id: func.id,
                symbol: func.symbol,
                params,
                results,
                blocks: func_blocks,
                entry,
            });
        }

        // Pass 7: Populate Block Records (edges will be built next)
        for block in &image.blocks {
            let parent_ix = space.maps.functions[&block.parent];
            let mut block_insns = Vec::new();
            for &insn_id in &block.instructions {
                let insn_ix = space.maps.instructions.get(&insn_id).ok_or_else(|| {
                    SpaceError::Inconsistency(format!(
                        "Block {} references missing instruction {}",
                        block.id.0, insn_id.0
                    ))
                })?;
                block_insns.push(*insn_ix);
            }

            space.blocks.push(BlockRec {
                id: block.id,
                parent: parent_ix,
                instructions: block_insns,
                outgoing: Vec::new(),
                incoming: Vec::new(),
            });
        }

        // Pass 8: Populate Instruction Records & Operands
        for insn in &image.instructions {
            let parent_block_ix = insn_parents.get(&insn.id).ok_or_else(|| {
                SpaceError::Inconsistency(format!(
                    "Instruction {} has no parent block mapping",
                    insn.id.0
                ))
            })?;
            let parent_block_rec = &image.blocks[parent_block_ix.0];
            let parent_func_id = parent_block_rec.parent;

            let mut insn_results = Vec::new();
            for &res_val_id in &insn.results {
                let val_ix = space
                    .maps
                    .values
                    .get(&(parent_func_id, res_val_id))
                    .ok_or_else(|| {
                        SpaceError::Inconsistency(format!(
                            "Instruction {} result value {} is not registered in function {}",
                            insn.id.0, res_val_id.0, parent_func_id.0
                        ))
                    })?;
                insn_results.push(*val_ix);
            }

            let mut insn_operands = Vec::new();
            for op in &insn.operands {
                let op_rec = match op {
                    mircap::Operand::Value(val_id) => {
                        let val_ix = space
                            .maps
                            .values
                            .get(&(parent_func_id, *val_id))
                            .ok_or_else(|| {
                                SpaceError::Inconsistency(format!(
                                    "Operand references unregistered value {} in function {}",
                                    val_id.0, parent_func_id.0
                                ))
                            })?;
                        OperandRec::Value(*val_ix)
                    }
                    mircap::Operand::ImmI32(val) => OperandRec::ImmI32(*val),
                    mircap::Operand::ImmU32(val) => OperandRec::ImmU32(*val),
                    mircap::Operand::Block(blk_id) => {
                        let blk_ix = space.maps.blocks.get(blk_id).ok_or_else(|| {
                            SpaceError::Inconsistency(format!(
                                "Operand references unregistered block {}",
                                blk_id.0
                            ))
                        })?;
                        OperandRec::Block(*blk_ix)
                    }
                    mircap::Operand::Function(func_id) => {
                        let func_ix = space.maps.functions.get(func_id).ok_or_else(|| {
                            SpaceError::Inconsistency(format!(
                                "Operand references unregistered function {}",
                                func_id.0
                            ))
                        })?;
                        OperandRec::Function(*func_ix)
                    }
                    mircap::Operand::Symbol(sym_id) => {
                        let sym_ix = space.maps.symbols.get(sym_id).ok_or_else(|| {
                            SpaceError::Inconsistency(format!(
                                "Operand references unregistered symbol {}",
                                sym_id.0
                            ))
                        })?;
                        OperandRec::Symbol(*sym_ix)
                    }
                    mircap::Operand::Type(ty_id) => OperandRec::Type(*ty_id),
                };

                let op_ix = OperandIx(space.operands.len());
                space.operands.push(op_rec);
                insn_operands.push(op_ix);
            }

            space.instructions.push(InstructionRec {
                id: insn.id,
                parent: *parent_block_ix,
                opcode: insn.opcode,
                results: insn_results,
                operands: insn_operands,
            });
        }

        // Pass 9: Construct CFG
        space.build_cfg()?;

        Ok(space)
    }
}
