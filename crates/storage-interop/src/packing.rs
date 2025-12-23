//! Storage slot packing utilities aligned with Solidity's layout rules.

use alloy_primitives::U256;

use crate::{layout::Packable, storage::StorageOps, InteropError, Result};

pub struct PackedSlot(pub U256);

impl StorageOps for PackedSlot {
    fn load(&self, _slot: U256) -> Result<U256> {
        Ok(self.0)
    }

    fn store(&mut self, _slot: U256, value: U256) -> Result<()> {
        self.0 = value;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FieldLocation {
    pub offset_slots: usize,
    pub offset_bytes: usize,
    pub size: usize,
}

impl FieldLocation {
    #[inline]
    pub const fn new(offset_slots: usize, offset_bytes: usize, size: usize) -> Self {
        Self {
            offset_slots,
            offset_bytes,
            size,
        }
    }
}

#[inline]
pub fn create_element_mask(byte_count: usize) -> U256 {
    if byte_count >= 32 {
        U256::MAX
    } else {
        (U256::ONE << (byte_count * 8)) - U256::ONE
    }
}

#[inline]
pub fn extract_packed_value<T: Packable>(
    slot_value: U256,
    offset: usize,
    bytes: usize,
) -> Result<T> {
    if offset + bytes > 32 {
        return Err(InteropError::PackedSlotOverflow { offset, bytes });
    }

    let shift_bits = offset * 8;
    let mask = create_element_mask(bytes);

    T::from_word((slot_value >> shift_bits) & mask)
}

#[inline]
pub fn insert_packed_value<T: Packable>(
    current: U256,
    value: &T,
    offset: usize,
    bytes: usize,
) -> Result<U256> {
    if offset + bytes > 32 {
        return Err(InteropError::PackedSlotOverflow { offset, bytes });
    }

    let field_value = value.to_word();
    let shift_bits = offset * 8;
    let mask = create_element_mask(bytes);

    let clear_mask = !(mask << shift_bits);
    let cleared = current & clear_mask;
    let positioned = (field_value & mask) << shift_bits;
    Ok(cleared | positioned)
}

#[inline]
pub fn zero_packed_value(current: U256, offset: usize, bytes: usize) -> Result<U256> {
    if offset + bytes > 32 {
        return Err(InteropError::PackedSlotOverflow { offset, bytes });
    }

    let mask = create_element_mask(bytes);
    let shifted_mask = mask << (offset * 8);
    Ok(current & !shifted_mask)
}

#[inline]
pub const fn calc_element_slot(idx: usize, elem_bytes: usize) -> usize {
    (idx * elem_bytes) / 32
}

#[inline]
pub const fn calc_element_offset(idx: usize, elem_bytes: usize) -> usize {
    (idx * elem_bytes) % 32
}

#[inline]
pub const fn calc_element_loc(idx: usize, elem_bytes: usize) -> FieldLocation {
    FieldLocation::new(
        calc_element_slot(idx, elem_bytes),
        calc_element_offset(idx, elem_bytes),
        elem_bytes,
    )
}

#[inline]
pub const fn calc_packed_slot_count(n: usize, elem_bytes: usize) -> usize {
    (n * elem_bytes).div_ceil(32)
}
