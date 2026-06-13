use crate::trap::ExecutionTrap;

#[derive(Clone, Debug)]
pub struct LinearMemory {
    bytes: Vec<u8>,
    heap_ptr: u32,
    stack_base: u32,
    allocation_count: u64,
    allocated_bytes: u64,
}

impl LinearMemory {
    pub fn new(linear_memory_size: u32, stack_size: u32) -> Self {
        let stack_base = linear_memory_size.saturating_sub(stack_size);
        Self {
            bytes: vec![0; linear_memory_size as usize],
            heap_ptr: 0,
            stack_base,
            allocation_count: 0,
            allocated_bytes: 0,
        }
    }

    pub fn initialize_data(
        &mut self,
        offset: u32,
        bytes: &[u8],
        zero_fill: u32,
    ) -> Result<(), ExecutionTrap> {
        let byte_len = bytes.len() as u32;
        self.check_store(offset, byte_len, 1)?;
        let zero_start = offset
            .checked_add(byte_len)
            .ok_or(ExecutionTrap::OutOfBoundsStore {
                addr: offset,
                size: byte_len,
            })?;
        self.check_store(zero_start, zero_fill, 1)?;
        let end = zero_start
            .checked_add(zero_fill)
            .ok_or(ExecutionTrap::OutOfBoundsStore {
                addr: zero_start,
                size: zero_fill,
            })?;
        self.bytes[offset as usize..offset as usize + bytes.len()].copy_from_slice(bytes);
        for byte in &mut self.bytes[zero_start as usize..end as usize] {
            *byte = 0;
        }
        self.heap_ptr = self.heap_ptr.max(end);
        Ok(())
    }

    pub fn alloc(&mut self, size: u32, align: u32) -> Result<u32, ExecutionTrap> {
        if align == 0 || !align.is_power_of_two() {
            return Err(ExecutionTrap::MisalignedStore {
                addr: self.heap_ptr,
                align,
            });
        }
        let aligned = align_up(self.heap_ptr, align).ok_or(ExecutionTrap::OutOfMemory {
            requested: size,
            align,
        })?;
        let end = aligned
            .checked_add(size)
            .ok_or(ExecutionTrap::OutOfMemory {
                requested: size,
                align,
            })?;
        if end > self.stack_base {
            return Err(ExecutionTrap::HeapStackCollision {
                requested: size,
                align,
            });
        }
        self.heap_ptr = end;
        self.allocation_count += 1;
        self.allocated_bytes += u64::from(size);
        Ok(aligned)
    }

    pub fn check_load(&self, addr: u32, size: u32, align: u32) -> Result<(), ExecutionTrap> {
        if align != 0 && addr % align != 0 {
            return Err(ExecutionTrap::MisalignedLoad { addr, align });
        }
        let end = addr
            .checked_add(size)
            .ok_or(ExecutionTrap::OutOfBoundsLoad { addr, size })?;
        if end as usize > self.bytes.len() {
            return Err(ExecutionTrap::OutOfBoundsLoad { addr, size });
        }
        Ok(())
    }

    pub fn check_store(&self, addr: u32, size: u32, align: u32) -> Result<(), ExecutionTrap> {
        if align != 0 && addr % align != 0 {
            return Err(ExecutionTrap::MisalignedStore { addr, align });
        }
        let end = addr
            .checked_add(size)
            .ok_or(ExecutionTrap::OutOfBoundsStore { addr, size })?;
        if end as usize > self.bytes.len() {
            return Err(ExecutionTrap::OutOfBoundsStore { addr, size });
        }
        Ok(())
    }

    pub fn load_i32(&self, addr: u32) -> Result<i32, ExecutionTrap> {
        self.check_load(addr, 4, 4)?;
        let bytes = self.four_bytes(addr);
        Ok(i32::from_le_bytes(bytes))
    }

    pub fn load_u32(&self, addr: u32) -> Result<u32, ExecutionTrap> {
        self.check_load(addr, 4, 4)?;
        let bytes = self.four_bytes(addr);
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn store_i32(&mut self, addr: u32, value: i32) -> Result<(), ExecutionTrap> {
        self.check_store(addr, 4, 4)?;
        self.bytes[addr as usize..addr as usize + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn store_u32(&mut self, addr: u32, value: u32) -> Result<(), ExecutionTrap> {
        self.check_store(addr, 4, 4)?;
        self.bytes[addr as usize..addr as usize + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn load_u8(&self, addr: u32) -> Result<u8, ExecutionTrap> {
        self.check_load(addr, 1, 1)?;
        Ok(self.bytes[addr as usize])
    }

    pub fn store_u8(&mut self, addr: u32, value: u8) -> Result<(), ExecutionTrap> {
        self.check_store(addr, 1, 1)?;
        self.bytes[addr as usize] = value;
        Ok(())
    }

    pub fn allocation_count(&self) -> u64 {
        self.allocation_count
    }

    pub fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    pub fn size(&self) -> u32 {
        self.bytes.len() as u32
    }

    fn four_bytes(&self, addr: u32) -> [u8; 4] {
        let start = addr as usize;
        [
            self.bytes[start],
            self.bytes[start + 1],
            self.bytes[start + 2],
            self.bytes[start + 3],
        ]
    }
}

fn align_up(value: u32, align: u32) -> Option<u32> {
    let mask = align.checked_sub(1)?;
    value.checked_add(mask).map(|v| v & !mask)
}
