use crate::error::{ExecutionError, RunError};
use crate::frame::Frame;
use crate::memory::LinearMemory;
use crate::profile::ExecutionProfile;
use crate::trace::{
    BlockTrace, CallEdgeTrace, FunctionTrace, TraceOutcome, TraceSnapshot, TraceState,
};
use crate::trap::ExecutionTrap;
use crate::value::Value;
use mircap::image::Function;
use mircap::{
    Block, BlockId, FunctionId, Instruction, ModuleImage, Opcode, Operand, SymbolKind, ValueId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub values: Vec<Value>,
    pub executed_instruction_count: u64,
}

pub struct Runner {
    image: ModuleImage,
    profile: ExecutionProfile,
    memory: LinearMemory,
    trace: TraceState,
}

impl Runner {
    pub fn new(image: ModuleImage, profile: ExecutionProfile) -> Result<Self, ExecutionError> {
        image.validate().map_err(ExecutionError::Validation)?;
        let mut memory = LinearMemory::new(profile.linear_memory_size, profile.stack_size);
        for data in &image.data_segments {
            memory
                .initialize_data(data.offset, &data.bytes, data.zero_fill)
                .map_err(ExecutionError::Trap)?;
        }
        Ok(Self {
            image,
            profile,
            memory,
            trace: TraceState::default(),
        })
    }

    pub fn run_entry_by_name(
        &mut self,
        name: &str,
        args: &[Value],
    ) -> Result<ExecutionResult, RunError> {
        let function = self
            .image
            .functions
            .iter()
            .find(|function| {
                self.image
                    .symbol(function.symbol)
                    .map(|symbol| symbol.kind == SymbolKind::Function && symbol.name == name)
                    .unwrap_or(false)
            })
            .ok_or_else(|| ExecutionError::Internal(format!("entry function not found: {name}")))?;
        self.run_entry(function.id, args)
    }

    pub fn run_entry(
        &mut self,
        entry: FunctionId,
        args: &[Value],
    ) -> Result<ExecutionResult, RunError> {
        self.trace = TraceState::default();
        self.trace.entry_function = Some(entry);

        let result = self.run(entry, args);
        match &result {
            Ok(result) => self.trace.outcome = TraceOutcome::Returned(result.values.clone()),
            Err(ExecutionError::Trap(trap)) => {
                if let Some(function) = self.trace.current_function {
                    self.trace.record_trap(function);
                }
                self.trace.outcome = TraceOutcome::Trapped(trap.clone())
            }
            Err(_) => {}
        }
        result
    }

    pub fn trace_snapshot(&self) -> TraceSnapshot {
        let functions =
            self.trace
                .function_calls
                .iter()
                .map(|(function, calls)| {
                    let blocks =
                        self.image
                            .function(*function)
                            .map(|function| {
                                function
                                    .blocks
                                    .iter()
                                    .filter_map(|block| {
                                        self.trace.block_entries.get(block).map(|entries| {
                                            BlockTrace {
                                                block: *block,
                                                entries: *entries,
                                            }
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                    FunctionTrace {
                        function: *function,
                        calls: *calls,
                        executed_instructions: self
                            .trace
                            .function_instruction_counts
                            .get(function)
                            .copied()
                            .unwrap_or(0),
                        branches: self
                            .trace
                            .function_branch_counts
                            .get(function)
                            .copied()
                            .unwrap_or(0),
                        call_instructions: self
                            .trace
                            .function_call_instruction_counts
                            .get(function)
                            .copied()
                            .unwrap_or(0),
                        address_instructions: self
                            .trace
                            .function_address_instruction_counts
                            .get(function)
                            .copied()
                            .unwrap_or(0),
                        allocations: self
                            .trace
                            .function_allocations
                            .get(function)
                            .copied()
                            .unwrap_or(0),
                        memory_reads: self
                            .trace
                            .function_memory_reads
                            .get(function)
                            .copied()
                            .unwrap_or(0),
                        memory_writes: self
                            .trace
                            .function_memory_writes
                            .get(function)
                            .copied()
                            .unwrap_or(0),
                        returns: self
                            .trace
                            .function_returns
                            .get(function)
                            .copied()
                            .unwrap_or(0),
                        traps: self
                            .trace
                            .function_traps
                            .get(function)
                            .copied()
                            .unwrap_or(0),
                        blocks,
                    }
                })
                .collect();

        let call_edges = self
            .trace
            .call_edges
            .iter()
            .map(|((caller, callee), calls)| CallEdgeTrace {
                caller: *caller,
                callee: *callee,
                calls: *calls,
            })
            .collect();

        TraceSnapshot {
            module_id: self.image.module.id,
            module_name: self.image.module.name.clone(),
            entry_function: self.trace.entry_function.unwrap_or(FunctionId(0)),
            outcome: self.trace.outcome.clone(),
            executed_instruction_count: self.trace.executed_instruction_count,
            branch_count: self.trace.branch_count,
            call_instruction_count: self.trace.call_instruction_count,
            address_instruction_count: self.trace.address_instruction_count,
            memory_read_count: self.trace.memory_read_count,
            memory_write_count: self.trace.memory_write_count,
            return_count: self.trace.return_count,
            trap_count: self.trace.trap_count,
            functions,
            call_edges,
            maximum_call_depth_reached: self.trace.maximum_call_depth_reached,
            memory_profile: self.profile.clone(),
            allocation_count: self.memory.allocation_count(),
            allocated_bytes: self.memory.allocated_bytes(),
        }
    }

    fn run(&mut self, entry: FunctionId, args: &[Value]) -> Result<ExecutionResult, RunError> {
        let mut stack = Vec::new();
        self.push_frame(&mut stack, entry, args, Vec::new())?;

        loop {
            if self.trace.executed_instruction_count >= self.profile.max_instructions {
                return Err(ExecutionTrap::FuelExhausted {
                    max_instructions: self.profile.max_instructions,
                }
                .into());
            }

            let frame = stack
                .last()
                .ok_or_else(|| ExecutionError::Internal("empty call stack".to_string()))?;
            self.trace.current_function = Some(frame.function);
            let block = self.current_block(frame)?;
            let insn_id = *block.instructions.get(frame.instruction_position).ok_or(
                ExecutionTrap::InvalidBlock {
                    function: frame.function,
                    block: frame.current_block,
                },
            )?;
            let insn = self
                .image
                .instruction(insn_id)
                .ok_or(ExecutionTrap::InvalidInstruction {
                    instruction: insn_id,
                })?
                .clone();

            self.trace.executed_instruction_count += 1;
            self.trace.record_instruction(frame.function);
            match insn.opcode {
                Opcode::Branch | Opcode::BranchIf => self.trace.record_branch(frame.function),
                Opcode::Call => self.trace.record_call_instruction(frame.function),
                Opcode::AddrAdd | Opcode::DataAddr => {
                    self.trace.record_address_instruction(frame.function)
                }
                _ => {}
            }
            match insn.opcode {
                Opcode::ConstI32 => self.exec_const_i32(&mut stack, &insn)?,
                Opcode::ConstU32 => self.exec_const_u32(&mut stack, &insn)?,
                Opcode::ConstI64 => self.exec_const_i64(&mut stack, &insn)?,
                Opcode::ConstF32 => self.exec_const_f32(&mut stack, &insn)?,
                Opcode::ConstF64 => self.exec_const_f64(&mut stack, &insn)?,
                Opcode::Copy => self.exec_copy(&mut stack, &insn)?,
                Opcode::AddI32
                | Opcode::SubI32
                | Opcode::MulI32
                | Opcode::EqI32
                | Opcode::NeI32
                | Opcode::LtI32 => self.exec_i32_binop(&mut stack, &insn)?,
                Opcode::AddU32
                | Opcode::SubU32
                | Opcode::MulU32
                | Opcode::EqU32
                | Opcode::NeU32
                | Opcode::LtU32
                | Opcode::LeU32
                | Opcode::GtU32
                | Opcode::GeU32 => self.exec_u32_binop(&mut stack, &insn)?,
                Opcode::AddI64
                | Opcode::SubI64
                | Opcode::MulI64
                | Opcode::EqI64
                | Opcode::NeI64
                | Opcode::LtI64 => self.exec_i64_binop(&mut stack, &insn)?,
                Opcode::AddF32 | Opcode::SubF32 | Opcode::MulF32 | Opcode::DivF32 => {
                    self.exec_f32_binop(&mut stack, &insn)?
                }
                Opcode::NegF32 => self.exec_f32_unop(&mut stack, &insn)?,
                Opcode::AddF64 | Opcode::SubF64 | Opcode::MulF64 | Opcode::DivF64 => {
                    self.exec_f64_binop(&mut stack, &insn)?
                }
                Opcode::NegF64 => self.exec_f64_unop(&mut stack, &insn)?,
                Opcode::Branch => self.exec_branch(&mut stack, &insn)?,
                Opcode::BranchIf => self.exec_branch_if(&mut stack, &insn)?,
                Opcode::Call => self.exec_call(&mut stack, &insn)?,
                Opcode::Ret => {
                    if let Some(values) = self.exec_return(&mut stack, &insn)? {
                        return Ok(ExecutionResult {
                            values,
                            executed_instruction_count: self.trace.executed_instruction_count,
                        });
                    }
                }
                Opcode::Trap => {
                    return Err(ExecutionTrap::ExplicitTrap {
                        instruction: insn.id,
                    }
                    .into())
                }
                Opcode::Alloc => self.exec_alloc(&mut stack, &insn)?,
                Opcode::LoadI32 => self.exec_load_i32(&mut stack, &insn)?,
                Opcode::LoadU32 => self.exec_load_u32(&mut stack, &insn)?,
                Opcode::LoadI64 => self.exec_load_i64(&mut stack, &insn)?,
                Opcode::StoreI32 => self.exec_store_i32(&mut stack, &insn)?,
                Opcode::StoreU32 => self.exec_store_u32(&mut stack, &insn)?,
                Opcode::StoreI64 => self.exec_store_i64(&mut stack, &insn)?,
                Opcode::LoadU8 => self.exec_load_u8(&mut stack, &insn)?,
                Opcode::StoreU8 => self.exec_store_u8(&mut stack, &insn)?,
                Opcode::AddrAdd => self.exec_addr_add(&mut stack, &insn)?,
                Opcode::DataAddr => self.exec_data_addr(&mut stack, &insn)?,
                Opcode::EqF32
                | Opcode::NeF32
                | Opcode::LtF32
                | Opcode::LeF32
                | Opcode::GtF32
                | Opcode::GeF32 => self.exec_f32_cmp(&mut stack, &insn)?,
                Opcode::EqF64
                | Opcode::NeF64
                | Opcode::LtF64
                | Opcode::LeF64
                | Opcode::GtF64
                | Opcode::GeF64 => self.exec_f64_cmp(&mut stack, &insn)?,
                Opcode::I32ToF32
                | Opcode::F32ToI32
                | Opcode::I32ToF64
                | Opcode::F64ToI32
                | Opcode::F32ToF64
                | Opcode::F64ToF32 => self.exec_float_convert(&mut stack, &insn)?,
                | Opcode::UnsupportedIndirectCall => {
                    return Err(ExecutionTrap::UnsupportedInstruction {
                        instruction: insn.id,
                        opcode: format!("{:?}", insn.opcode),
                    }
                    .into());
                }
            }
        }
    }

    fn push_frame(
        &mut self,
        stack: &mut Vec<Frame>,
        function_id: FunctionId,
        args: &[Value],
        return_destinations: Vec<ValueId>,
    ) -> Result<(), RunError> {
        if stack.len() >= self.profile.max_call_depth {
            return Err(ExecutionTrap::StackOverflow {
                max_depth: self.profile.max_call_depth,
            }
            .into());
        }
        let function = self.function(function_id)?;
        if args.len() != function.params.len() {
            return Err(ExecutionError::Internal(format!(
                "entry/call arity mismatch for function {function_id}"
            )));
        }
        let first_block = *function.blocks.first().ok_or(ExecutionTrap::InvalidBlock {
            function: function_id,
            block: BlockId(0),
        })?;
        let mut frame = Frame::new(
            function_id,
            first_block,
            function.value_count,
            return_destinations,
        );
        for (idx, arg) in args.iter().enumerate() {
            frame.write(ValueId(idx as u32), arg.clone());
        }
        stack.push(frame);
        self.trace.record_function_call(function_id, stack.len());
        self.trace.record_block_entry(first_block);
        Ok(())
    }

    fn exec_const_i32(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let value = match insn.operands.first() {
            Some(Operand::ImmI32(value)) => Value::I32(*value),
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, value)
    }

    fn exec_const_u32(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let value = match insn.operands.first() {
            Some(Operand::ImmU32(value)) => Value::U32(*value),
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, value)
    }

    fn exec_const_f32(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let value = match insn.operands.first() {
            Some(Operand::ImmF32(bits)) => Value::F32(*bits),
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, value)
    }

    fn exec_const_f64(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let value = match insn.operands.first() {
            Some(Operand::ImmF64(bits)) => Value::F64(*bits),
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, value)
    }

    fn exec_copy(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let source = self.value_operand(stack, insn, 0)?;
        self.write_result_and_advance(stack, insn, source)
    }

    fn exec_i32_binop(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let lhs =
            self.value_operand(stack, insn, 0)?
                .as_i32()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let rhs =
            self.value_operand(stack, insn, 1)?
                .as_i32()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let value = match insn.opcode {
            Opcode::AddI32 => Value::I32(lhs.wrapping_add(rhs)),
            Opcode::SubI32 => Value::I32(lhs.wrapping_sub(rhs)),
            Opcode::MulI32 => Value::I32(lhs.wrapping_mul(rhs)),
            Opcode::EqI32 => Value::U32((lhs == rhs) as u32),
            Opcode::NeI32 => Value::U32((lhs != rhs) as u32),
            Opcode::LtI32 => Value::U32((lhs < rhs) as u32),
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, value)
    }

    fn exec_f32_binop(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let lhs =
            self.value_operand(stack, insn, 0)?
                .as_f32()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let rhs =
            self.value_operand(stack, insn, 1)?
                .as_f32()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let result = match insn.opcode {
            Opcode::AddF32 => lhs + rhs,
            Opcode::SubF32 => lhs - rhs,
            Opcode::MulF32 => lhs * rhs,
            Opcode::DivF32 => lhs / rhs,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, Value::F32(result.to_bits()))
    }

    fn exec_f32_unop(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let operand =
            self.value_operand(stack, insn, 0)?
                .as_f32()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let result = match insn.opcode {
            Opcode::NegF32 => -operand,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, Value::F32(result.to_bits()))
    }

    fn exec_f64_binop(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let lhs =
            self.value_operand(stack, insn, 0)?
                .as_f64()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let rhs =
            self.value_operand(stack, insn, 1)?
                .as_f64()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let result = match insn.opcode {
            Opcode::AddF64 => lhs + rhs,
            Opcode::SubF64 => lhs - rhs,
            Opcode::MulF64 => lhs * rhs,
            Opcode::DivF64 => lhs / rhs,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, Value::F64(result.to_bits()))
    }

    fn exec_f64_unop(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let operand =
            self.value_operand(stack, insn, 0)?
                .as_f64()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let result = match insn.opcode {
            Opcode::NegF64 => -operand,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, Value::F64(result.to_bits()))
    }

    fn exec_f32_cmp(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let lhs =
            self.value_operand(stack, insn, 0)?
                .as_f32()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let rhs =
            self.value_operand(stack, insn, 1)?
                .as_f32()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let result = match insn.opcode {
            Opcode::EqF32 => lhs == rhs,
            Opcode::NeF32 => lhs != rhs,
            Opcode::LtF32 => lhs < rhs,
            Opcode::LeF32 => lhs <= rhs,
            Opcode::GtF32 => lhs > rhs,
            Opcode::GeF32 => lhs >= rhs,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, Value::I32(result as i32))
    }

    fn exec_f64_cmp(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let lhs =
            self.value_operand(stack, insn, 0)?
                .as_f64()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let rhs =
            self.value_operand(stack, insn, 1)?
                .as_f64()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let result = match insn.opcode {
            Opcode::EqF64 => lhs == rhs,
            Opcode::NeF64 => lhs != rhs,
            Opcode::LtF64 => lhs < rhs,
            Opcode::LeF64 => lhs <= rhs,
            Opcode::GtF64 => lhs > rhs,
            Opcode::GeF64 => lhs >= rhs,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, Value::I32(result as i32))
    }

    fn exec_float_convert(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let val = self.value_operand(stack, insn, 0)?;
        let result = match insn.opcode {
            Opcode::I32ToF32 => {
                let i = val.as_i32().ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
                Value::F32((i as f32).to_bits())
            }
            Opcode::F32ToI32 => {
                let f = val.as_f32().ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
                Value::I32(f as i32)
            }
            Opcode::I32ToF64 => {
                let i = val.as_i32().ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
                Value::F64((i as f64).to_bits())
            }
            Opcode::F64ToI32 => {
                let f = val.as_f64().ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
                Value::I32(f as i32)
            }
            Opcode::F32ToF64 => {
                let f = val.as_f32().ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
                Value::F64((f as f64).to_bits())
            }
            Opcode::F64ToF32 => {
                let f = val.as_f64().ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
                Value::F32((f as f32).to_bits())
            }
            _ => return Err(ExecutionTrap::InvalidInstruction {
                instruction: insn.id,
            }.into())
        };
        self.write_result_and_advance(stack, insn, result)
    }

    fn exec_branch(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let target = match insn.operands.first() {
            Some(Operand::Block(block)) => *block,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.enter_block(stack, target)
    }

    fn exec_branch_if(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let cond = match self.value_operand(stack, insn, 0)? {
            Value::U32(value) => value != 0,
            _ => {
                return Err(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                }
                .into())
            }
        };
        let target = match insn.operands.get(1) {
            Some(Operand::Block(block)) => *block,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        let false_target = match insn.operands.get(2) {
            Some(Operand::Block(block)) => *block,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        if cond {
            self.enter_block(stack, target)
        } else {
            self.enter_block(stack, false_target)
        }
    }

    fn exec_call(&mut self, stack: &mut Vec<Frame>, insn: &Instruction) -> Result<(), RunError> {
        let caller = self.current_frame(stack)?.function;
        let callee = match insn.operands.first() {
            Some(Operand::Function(function)) => *function,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        let function = self.function(callee)?;
        let mut args = Vec::new();
        for idx in 1..insn.operands.len() {
            args.push(self.value_operand(stack, insn, idx)?);
        }
        if args.len() != function.params.len() || insn.results.len() != function.results.len() {
            return Err(ExecutionTrap::CallArityMismatch {
                instruction: insn.id,
            }
            .into());
        }
        self.current_frame_mut(stack)?.instruction_position += 1;
        self.trace.record_call_edge(caller, callee);
        self.push_frame(stack, callee, &args, insn.results.clone())
    }

    fn exec_alloc(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let size = self.u32_operand(stack, insn, 0)?;
        let align = self.u32_operand(stack, insn, 1)?;
        let addr = self.memory.alloc(size, align)?;
        self.trace
            .record_allocation(self.current_frame(stack)?.function);
        self.write_result_and_advance(stack, insn, Value::Addr32(addr))
    }

    fn exec_load_i32(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let addr = self.addr_operand(stack, insn, 0)?;
        let value = self.memory.load_i32(addr)?;
        self.trace
            .record_memory_read(self.current_frame(stack)?.function);
        self.write_result_and_advance(stack, insn, Value::I32(value))
    }

    fn exec_load_u32(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let addr = self.addr_operand(stack, insn, 0)?;
        let value = self.memory.load_u32(addr)?;
        self.trace
            .record_memory_read(self.current_frame(stack)?.function);
        self.write_result_and_advance(stack, insn, Value::U32(value))
    }

    fn exec_store_i32(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let addr = self.addr_operand(stack, insn, 0)?;
        let value =
            self.value_operand(stack, insn, 1)?
                .as_i32()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        self.memory.store_i32(addr, value)?;
        self.trace
            .record_memory_write(self.current_frame(stack)?.function);
        self.current_frame_mut(stack)?.instruction_position += 1;
        Ok(())
    }

    fn exec_store_u32(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let addr = self.addr_operand(stack, insn, 0)?;
        let value = match self.value_operand(stack, insn, 1)? {
            Value::U32(value) => value,
            _ => {
                return Err(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                }
                .into())
            }
        };
        self.memory.store_u32(addr, value)?;
        self.trace
            .record_memory_write(self.current_frame(stack)?.function);
        self.current_frame_mut(stack)?.instruction_position += 1;
        Ok(())
    }

    fn exec_addr_add(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let base = self.addr_operand(stack, insn, 0)?;
        let offset = match self.value_operand(stack, insn, 1)? {
            Value::U32(value) => value,
            _ => {
                return Err(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                }
                .into())
            }
        };
        let addr = base
            .checked_add(offset)
            .ok_or(ExecutionTrap::AddressOverflow { base, offset })?;
        self.write_result_and_advance(stack, insn, Value::Addr32(addr))
    }

    fn exec_return(
        &mut self,
        stack: &mut Vec<Frame>,
        insn: &Instruction,
    ) -> Result<Option<Vec<Value>>, RunError> {
        let values = {
            let mut values = Vec::new();
            for idx in 0..insn.operands.len() {
                values.push(self.value_operand(stack, insn, idx)?);
            }
            values
        };
        let finished = stack
            .pop()
            .ok_or_else(|| ExecutionError::Internal("return with empty stack".to_string()))?;
        self.trace.record_return(finished.function);
        if let Some(caller) = stack.last_mut() {
            if finished.return_destinations.len() != values.len() {
                return Err(ExecutionTrap::ReturnArityMismatch {
                    instruction: insn.id,
                }
                .into());
            }
            for (dest, value) in finished.return_destinations.into_iter().zip(values) {
                if !caller.write(dest, value) {
                    return Err(ExecutionTrap::InvalidInstruction {
                        instruction: insn.id,
                    }
                    .into());
                }
            }
            Ok(None)
        } else {
            Ok(Some(values))
        }
    }

    fn write_result_and_advance(
        &self,
        stack: &mut [Frame],
        insn: &Instruction,
        value: Value,
    ) -> Result<(), RunError> {
        let Some(result) = insn.results.first().copied() else {
            return Err(ExecutionTrap::InvalidInstruction {
                instruction: insn.id,
            }
            .into());
        };
        let frame = self.current_frame_mut(stack)?;
        if !frame.write(result, value) {
            return Err(ExecutionTrap::InvalidInstruction {
                instruction: insn.id,
            }
            .into());
        }
        frame.instruction_position += 1;
        Ok(())
    }

    fn value_operand(
        &self,
        stack: &[Frame],
        insn: &Instruction,
        idx: usize,
    ) -> Result<Value, RunError> {
        let value_id = match insn.operands.get(idx) {
            Some(Operand::Value(value)) => *value,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        let frame = self.current_frame(stack)?;
        frame.read(value_id).ok_or(
            ExecutionTrap::UninitializedValue {
                function: frame.function,
                value: value_id.0,
            }
            .into(),
        )
    }

    fn u32_operand(
        &self,
        stack: &[Frame],
        insn: &Instruction,
        idx: usize,
    ) -> Result<u32, RunError> {
        match insn.operands.get(idx) {
            Some(Operand::ImmU32(value)) => Ok(*value),
            Some(Operand::Value(_)) => match self.value_operand(stack, insn, idx)? {
                Value::U32(value) => Ok(value),
                Value::I32(value) if value >= 0 => Ok(value as u32),
                _ => Err(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                }
                .into()),
            },
            _ => Err(ExecutionTrap::InvalidInstruction {
                instruction: insn.id,
            }
            .into()),
        }
    }

    fn addr_operand(
        &self,
        stack: &[Frame],
        insn: &Instruction,
        idx: usize,
    ) -> Result<u32, RunError> {
        match self.value_operand(stack, insn, idx)? {
            Value::Addr32(addr) => Ok(addr),
            _ => Err(ExecutionTrap::UnsupportedType {
                function: self.current_frame(stack)?.function,
            }
            .into()),
        }
    }

    fn enter_block(&mut self, stack: &mut [Frame], block: BlockId) -> Result<(), RunError> {
        let function = self.current_frame(stack)?.function;
        let target = self
            .image
            .block(block)
            .ok_or(ExecutionTrap::InvalidBlock { function, block })?;
        if target.parent != function {
            return Err(ExecutionTrap::InvalidBlock { function, block }.into());
        }
        let frame = self.current_frame_mut(stack)?;
        frame.current_block = block;
        frame.instruction_position = 0;
        self.trace.record_block_entry(block);
        Ok(())
    }

    fn current_frame<'a>(&self, stack: &'a [Frame]) -> Result<&'a Frame, RunError> {
        stack
            .last()
            .ok_or_else(|| ExecutionError::Internal("empty call stack".to_string()))
    }

    fn current_frame_mut<'a>(&self, stack: &'a mut [Frame]) -> Result<&'a mut Frame, RunError> {
        stack
            .last_mut()
            .ok_or_else(|| ExecutionError::Internal("empty call stack".to_string()))
    }

    fn current_block<'a>(&'a self, frame: &Frame) -> Result<&'a Block, RunError> {
        self.image.block(frame.current_block).ok_or(
            ExecutionTrap::InvalidBlock {
                function: frame.function,
                block: frame.current_block,
            }
            .into(),
        )
    }

    fn exec_u32_binop(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let lhs = match self.value_operand(stack, insn, 0)? {
            Value::U32(val) => val,
            _ => {
                return Err(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                }
                .into())
            }
        };
        let rhs = match self.value_operand(stack, insn, 1)? {
            Value::U32(val) => val,
            _ => {
                return Err(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                }
                .into())
            }
        };
        let value = match insn.opcode {
            Opcode::AddU32 => Value::U32(lhs.wrapping_add(rhs)),
            Opcode::SubU32 => Value::U32(lhs.wrapping_sub(rhs)),
            Opcode::MulU32 => Value::U32(lhs.wrapping_mul(rhs)),
            Opcode::EqU32 => Value::U32((lhs == rhs) as u32),
            Opcode::NeU32 => Value::U32((lhs != rhs) as u32),
            Opcode::LtU32 => Value::U32((lhs < rhs) as u32),
            Opcode::LeU32 => Value::U32((lhs <= rhs) as u32),
            Opcode::GtU32 => Value::U32((lhs > rhs) as u32),
            Opcode::GeU32 => Value::U32((lhs >= rhs) as u32),
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, value)
    }

    fn exec_data_addr(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let sym_id = match insn.operands.first() {
            Some(Operand::Symbol(sym_id)) => *sym_id,
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        let offset = match self.value_operand(stack, insn, 1)? {
            Value::U32(val) => val,
            _ => {
                return Err(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                }
                .into())
            }
        };
        let ds = self
            .image
            .data_segments
            .iter()
            .find(|ds| ds.symbol == sym_id)
            .ok_or_else(|| {
                ExecutionError::Internal(format!("missing data segment symbol {:?}", sym_id))
            })?;
        let segment_len = ds.bytes.len() as u32 + ds.zero_fill;
        if offset > segment_len {
            return Err(ExecutionTrap::OutOfBoundsLoad {
                addr: ds.offset + offset,
                size: 1,
            }
            .into());
        }
        let addr = ds
            .offset
            .checked_add(offset)
            .ok_or_else(|| ExecutionTrap::AddressOverflow {
                base: ds.offset,
                offset,
            })?;
        self.write_result_and_advance(stack, insn, Value::Addr32(addr))
    }

    fn exec_load_u8(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let addr = self.addr_operand(stack, insn, 0)?;
        let value = self.memory.load_u8(addr)?;
        self.trace
            .record_memory_read(self.current_frame(stack)?.function);
        self.write_result_and_advance(stack, insn, Value::U32(value as u32))
    }

    fn exec_store_u8(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let addr = self.addr_operand(stack, insn, 0)?;
        let value = match self.value_operand(stack, insn, 1)? {
            Value::U32(value) => value,
            _ => {
                return Err(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                }
                .into())
            }
        };
        self.memory.store_u8(addr, (value & 0xFF) as u8)?;
        self.trace
            .record_memory_write(self.current_frame(stack)?.function);
        self.current_frame_mut(stack)?.instruction_position += 1;
        Ok(())
    }

    fn exec_const_i64(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let value = match insn.operands.first() {
            Some(Operand::ImmI64(value)) => Value::I64(*value),
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, value)
    }

    fn exec_i64_binop(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let lhs =
            self.value_operand(stack, insn, 0)?
                .as_i64()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let rhs =
            self.value_operand(stack, insn, 1)?
                .as_i64()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        let value = match insn.opcode {
            Opcode::AddI64 => Value::I64(lhs.wrapping_add(rhs)),
            Opcode::SubI64 => Value::I64(lhs.wrapping_sub(rhs)),
            Opcode::MulI64 => Value::I64(lhs.wrapping_mul(rhs)),
            Opcode::EqI64 => Value::I32((lhs == rhs) as i32),
            Opcode::NeI64 => Value::I32((lhs != rhs) as i32),
            Opcode::LtI64 => Value::I32((lhs < rhs) as i32),
            _ => {
                return Err(ExecutionTrap::InvalidInstruction {
                    instruction: insn.id,
                }
                .into())
            }
        };
        self.write_result_and_advance(stack, insn, value)
    }

    fn exec_load_i64(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let addr = self.addr_operand(stack, insn, 0)?;
        let value = self.memory.load_i64(addr)?;
        self.trace
            .record_memory_read(self.current_frame(stack)?.function);
        self.write_result_and_advance(stack, insn, Value::I64(value))
    }

    fn exec_store_i64(&mut self, stack: &mut [Frame], insn: &Instruction) -> Result<(), RunError> {
        let addr = self.addr_operand(stack, insn, 0)?;
        let value =
            self.value_operand(stack, insn, 1)?
                .as_i64()
                .ok_or(ExecutionTrap::UnsupportedType {
                    function: self.current_frame(stack)?.function,
                })?;
        self.memory.store_i64(addr, value)?;
        self.trace
            .record_memory_write(self.current_frame(stack)?.function);
        self.current_frame_mut(stack)?.instruction_position += 1;
        Ok(())
    }

    fn function<'a>(&'a self, function: FunctionId) -> Result<&'a Function, RunError> {
        self.image
            .function(function)
            .ok_or_else(|| ExecutionError::Internal(format!("missing function {function}")))
    }
}
